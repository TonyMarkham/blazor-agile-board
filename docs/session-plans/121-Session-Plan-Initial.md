# Session 121: Portable Distribution + Repo-Aware Tauri

## Context

Two previously separate sessions merged into one:

**Session 120.3** — Tauri uses a global `app_data_dir` while CLI uses `<cwd>/.pm/`. They never overlap, so CLI can't discover a Tauri-spawned server. CLAUDE_FUCKED_UP.md caught the gap but proposed a fix (DEFAULT_PORT=0) that would break Tauri entirely (is_available(0) always returns true → infinite restart loop). The real fix: make Tauri respect `PM_CONFIG_DIR` so all tools share `<repo>/.pm/`.

**Session 110** — Portable distribution. All three binaries (pm, pm-server, Tauri desktop) install into `<repo>/.pm/bin/`. Database (`data.db`) and config (`config.toml`) are tracked in git. Runtime files are gitignored.

### Confirmed `.pm/` Directory Layout

```
<repo>/
└── .pm/
    ├── data.db                  # ✅ TRACKED — project management database
    ├── config.toml              # ✅ TRACKED — shared server config
    │
    ├── bin/                     # ❌ GITIGNORED — installed binaries
    │   ├── pm                   #    CLI executable
    │   ├── pm-server            #    Backend server
    │   └── Project Manager.app/ #    Tauri desktop (macOS)
    │
    ├── data.db-wal              # ❌ GITIGNORED — SQLite runtime
    ├── data.db-shm              # ❌ GITIGNORED — SQLite runtime
    ├── server.json              # ❌ GITIGNORED — port discovery file
    ├── server.lock              # ❌ GITIGNORED — process lock
    ├── logs/                    # ❌ GITIGNORED — pm-server logs
    └── tauri/                   # ❌ GITIGNORED — Tauri runtime data
```

---

## Investigation Findings (Reference)

### Session 120 Planning Failure
Both plan files claim "Tauri is fully solved" (line 8) without reading:
- `desktop/src-tauri/src/server/config.rs:19` — own `DEFAULT_PORT: u16 = 8000`
- `desktop/src-tauri/src/server/lifecycle.rs:249` — passes port via `PM_SERVER_PORT` env var
- `desktop/src-tauri/src/server/port.rs` — `PortManager` range scans 8000-8100

### CLAUDE_FUCKED_UP.md Fix Is Wrong
Changing `DEFAULT_PORT` to 0 would break Tauri:
1. `is_available(0)` → `TcpListener::bind(("127.0.0.1", 0))` → always succeeds
2. `PortManager::find_available(0, ...)` returns `Ok(0)` without scanning range
3. Tauri stores `actual_port = 0`, HealthChecker polls port 0 → infinite restart loop
4. `websocket_url()` → `ws://127.0.0.1:0/ws` → frontend can't connect

### The Real Gap: Directory Mismatch
| Mode | PM_CONFIG_DIR | Port file |
|------|--------------|-----------|
| CLI | `<cwd>/.pm/` | `<repo>/.pm/server.json` |
| Tauri | `~/Library/Application Support/.../.server/` | global app data dir |

CLI can't find Tauri's port file because they use different directories.

---

## Sub-Session Breakdown

| Sub-Session | Scope | Status |
|-------------|-------|--------|
| **121.1** | Infrastructure: workspace metadata, `.pm/` layout, `.pm/.gitignore` | Pending |
| **121.2** | Tauri repo-awareness: `PM_CONFIG_DIR` support, `pm desktop` command, `just dev` | Pending |
| **121.3** | Release distribution: justfile commands, install scripts, archive layout | Pending |

---

## Sub-Session 121.1: Infrastructure

### Step 1: Add `[workspace.package]` to Root Cargo.toml

**File: `Cargo.toml`** (root)

Insert between `[workspace]` and `[workspace.dependencies]`:
```toml
[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/OWNER/blazor-agile-board"
```

### Step 2: Inherit workspace metadata in pm-cli

**File: `backend/crates/pm-cli/Cargo.toml`**

Update `[package]` section:
```toml
[package]
name = "pm-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI tool for Blazor Agile Board project management"
```

### Step 3: Inherit workspace metadata in pm-server

**File: `backend/pm-server/Cargo.toml`**

Same pattern as Step 2.

### Step 4: Create `.pm/.gitignore`

**File: `.pm/.gitignore`** (NEW)

```gitignore
# Runtime files — not tracked
bin/
*.db-wal
*.db-shm
server.json
server.lock
logs/
tauri/
```

This ensures `data.db` and `config.toml` ARE tracked (not listed in gitignore).

### Step 5: Update root `.gitignore`

**File: `.gitignore`**

- Remove `Cargo.lock` (binary projects should commit lockfile)
- Add `dist/` (release archives)

Then run: `cargo generate-lockfile`

### Verification (121.1)
```bash
just check-backend
just test-backend
git status   # .pm/.gitignore should be new, Cargo.lock should be unignored
```

---

## Sub-Session 121.2: Tauri Repo-Awareness

### Step 1: Make Tauri respect `PM_CONFIG_DIR`

**File: `desktop/src-tauri/src/lib.rs`** (lines 42-50)

Currently:
```rust
let app_data_dir = app.path().app_data_dir()?;
let server_dir = app_data_dir.join(SERVER_DATA_DIR);
```

Change to:
```rust
let app_data_dir = app.path().app_data_dir()?;

// If PM_CONFIG_DIR is set (e.g., by `pm desktop`), use it for server data.
// This makes Tauri repo-aware: server data lives in <repo>/.pm/ alongside CLI data.
let server_dir = match std::env::var("PM_CONFIG_DIR") {
    Ok(dir) => PathBuf::from(dir),
    Err(_) => app_data_dir.join(SERVER_DATA_DIR),
};
```

This is the critical fix. When `PM_CONFIG_DIR` is set:
- `server_dir` = `<repo>/.pm/`
- Tauri passes `PM_CONFIG_DIR=<repo>/.pm/` to pm-server (`lifecycle.rs:249` — already does this)
- pm-server writes `server.json` to `<repo>/.pm/` (`main.rs:194` — already does this)
- CLI reads from `<cwd>/.pm/server.json` (`main.rs:167` — already does this)
- **Everything collocates. No changes needed to server or port file code.**

### Step 2: Add `pm desktop` subcommand

**File: `backend/crates/pm-cli/src/commands.rs`**

Add variant:
```rust
/// Launch the desktop app for this repo
Desktop,
```

**File: `backend/crates/pm-cli/src/main.rs`**

Handle before server discovery (`pm desktop` is a launcher — it spawns Tauri which spawns pm-server):
```rust
let cli = Cli::parse();

// Desktop launches Tauri (which spawns pm-server itself) — handle before server discovery
if matches!(cli.command, Commands::Desktop) {
    return launch_desktop();
}
```

Add `launch_desktop()` function:
- Sets `PM_CONFIG_DIR` to `<cwd>/.pm/`
- Creates `.pm/` dir if needed
- Finds Tauri binary:
  1. `PM_TAURI_BIN` env var (development override)
  2. `<cwd>/.pm/bin/Project Manager.app/Contents/MacOS/project-manager` (macOS installed)
  3. `<cwd>/.pm/bin/project-manager` (Linux/Windows installed)
  4. Workspace `target/debug` or `target/release` (development fallback)
- Spawns Tauri with `PM_CONFIG_DIR=<cwd>/.pm/` env var
- Exits

### Step 3: Update `just dev`

**File: `justfile`** (line 454)

Change:
```just
dev:
    just build-dev
    PM_CONFIG_DIR="{{justfile_directory()}}/.pm" cargo tauri dev
```

### Step 4: Update `CLAUDE_FUCKED_UP.md`

Replace "How To Fix" section with correct diagnosis:
- The fix is NOT `DEFAULT_PORT = 0` (that breaks Tauri)
- The fix IS making Tauri respect `PM_CONFIG_DIR`
- Document the `is_available(0)` always-true bug

### What Does NOT Change
- `desktop/src-tauri/src/server/config.rs` — DEFAULT_PORT stays 8000
- `desktop/src-tauri/src/server/lifecycle.rs` — already passes PM_CONFIG_DIR
- `desktop/src-tauri/src/server/port.rs` — PortManager stays as-is
- `backend/pm-server/src/main.rs` — already writes port file to config_dir
- `backend/crates/pm-config/` — port file code already works

### Verification (121.2)
```bash
just check-backend && just clippy-backend && just test-backend

# Test development mode
just dev
# In another terminal:
cat .pm/server.json        # repo-local port file with actual port
pm project list --pretty   # auto-discovers via .pm/server.json
# Close Tauri, verify .pm/server.json cleaned up

# Test standalone server still works
cargo run -p pm-server &
cat .pm/server.json
cargo run -p pm-cli -- project list --pretty
kill %1
```

---

## Sub-Session 121.3: Release Distribution

### Step 1: Add release variables and commands to justfile

**File: `justfile`**

New variables:
```just
dist_dir := "dist"
version := "0.1.0"
target_triple := `rustc -vV | grep host | cut -d' ' -f2`
archive_name := "pm-" + version + "-" + target_triple
```

New commands:
- `build-portable` — builds all 3 artifacts (pm CLI, pm-server, Tauri app)
- `archive` — creates platform archive with binaries structured for `.pm/bin/`
- `release-build` — build + archive
- `release <tag>` — create GitHub release + upload
- `release-upload <tag>` — upload to existing release
- `clean-dist` — remove dist/

Archive structure (extracts to `.pm/bin/`):
```
bin/
├── pm
├── pm-server
└── Project Manager.app/  (or project-manager on Linux, project-manager.exe on Windows)
```

### Step 2: Create install.sh

**File: `install.sh`** (NEW)

- Detects OS + architecture
- Downloads latest release from GitHub
- Extracts to `<cwd>/.pm/bin/`
- Creates `.pm/.gitignore` if not present
- Prints usage instructions

### Step 3: Create install.ps1

**File: `install.ps1`** (NEW)

Windows equivalent of install.sh.

### Verification (121.3)
```bash
just release-build
tar tzf dist/pm-*.tar.gz   # should show bin/pm, bin/pm-server, etc.

# Test install script locally
mkdir /tmp/test-repo && cd /tmp/test-repo
bash /path/to/install.sh    # or test manually
ls .pm/bin/                  # should contain binaries
.pm/bin/pm --version
```

---

## Full Files Summary

### Create
| File | Sub-Session | Purpose |
|------|-------------|---------|
| `.pm/.gitignore` | 1 | Selective tracking (data.db + config.toml tracked, rest ignored) |
| `install.sh` | 3 | macOS/Linux install script |
| `install.ps1` | 3 | Windows install script |

### Modify
| File | Sub-Session | Change |
|------|-------------|--------|
| `Cargo.toml` (root) | 1 | Add `[workspace.package]` |
| `backend/crates/pm-cli/Cargo.toml` | 1 | Inherit workspace metadata |
| `backend/pm-server/Cargo.toml` | 1 | Inherit workspace metadata |
| `.gitignore` | 1 | Remove Cargo.lock, add dist/ |
| `desktop/src-tauri/src/lib.rs` | 2 | Check `PM_CONFIG_DIR` for `server_dir` |
| `backend/crates/pm-cli/src/commands.rs` | 2 | Add `Desktop` variant |
| `backend/crates/pm-cli/src/main.rs` | 2 | Add `launch_desktop()` |
| `justfile` | 2+3 | Set PM_CONFIG_DIR in dev, add release commands |
| `CLAUDE_FUCKED_UP.md` | 2 | Correct diagnosis and fix |
