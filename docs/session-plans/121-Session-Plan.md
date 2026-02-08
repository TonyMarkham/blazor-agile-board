# Session 121: Portable Distribution + Repo-Aware Tauri

## Context

Two previously separate sessions merged into one coherent goal: **make all tools (CLI, server, desktop) share a single repo-local `.pm/` directory** for config, database, port discovery, and installed binaries.

**The core bug** (documented in `CLAUDE_FUCKED_UP.md`): Tauri always uses `~/Library/Application Support/com.projectmanager.app/.server/` as its server directory, while the CLI uses `<cwd>/.pm/`. They never overlap, so the CLI cannot discover a Tauri-spawned server. Session 120 planning incorrectly claimed this was "fully solved" without reading the Tauri code.

**The wrong fix** (proposed in `CLAUDE_FUCKED_UP.md`): Changing `DEFAULT_PORT` from 8000 to 0. This would break Tauri entirely because `is_available(0)` always returns true (port 0 means "OS picks any port"), causing an infinite restart loop.

**The correct fix**: Every process finds `.pm/` the same way — `git rev-parse --show-toplevel` returns the repo root, append `/.pm/`. No environment variables. No cwd assumptions. Works from any subdirectory. Works for any language's repo.

For Tauri launched via double-click (outside a terminal/repo context), the installer writes a `config.json` next to the binary with the repo root path as a fallback.

---

## Sub-Session Breakdown

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[121.1](121.1-Session-Plan.md)** | Infrastructure: workspace metadata, `.pm/` layout, `.gitignore`, justfile migration | ~25k | **COMPLETE** |
| **[121.2](121.2-Session-Plan.md)** | Tauri repo-awareness: git-based config discovery, `pm desktop` command | ~35k | Pending |
| **[121.3](121.3-Session-Plan.md)** | Release distribution: justfile commands, install scripts, `config.json` | ~30k | Pending |
| **[121.4](121.4-Session-Plan.md)** | Data sync: fix .gitignore, export/import commands, git hooks | ~40k | Pending |

---

## Session 121.1: Infrastructure — COMPLETE

**Files Created:**
- `.pm/.gitignore` — Selective git tracking for `.pm/` directory

**Files Modified:**
- `Cargo.toml` (root) — Add `[workspace.package]` section + typo fix
- `backend/crates/pm-cli/Cargo.toml` — Inherit workspace metadata
- `backend/pm-server/Cargo.toml` — Inherit workspace metadata
- `backend/crates/pm-config/Cargo.toml` — Inherit workspace metadata
- `backend/crates/pm-core/Cargo.toml` — Inherit workspace metadata (added beyond plan)
- `backend/crates/pm-db/Cargo.toml` — Inherit workspace metadata (added beyond plan)
- `backend/crates/pm-auth/Cargo.toml` — Inherit workspace metadata (added beyond plan)
- `backend/crates/pm-proto/Cargo.toml` — Inherit workspace metadata (added beyond plan)
- `backend/crates/pm-ws/Cargo.toml` — Inherit workspace metadata (added beyond plan)
- `.gitignore` — Remove `Cargo.lock`, `.pm/`, `.server`; add `dist/`
- `justfile` — Change `config_dir` from `.server` to `.pm`
- `backend/config.example.toml` — Update `.server/` references to `.pm/` (4 places)
- `desktop/src-tauri/tauri.conf.json` — Update resource source path

**Files Newly Tracked:**
- `Cargo.lock` — Reproducible builds for binary project
- `.pm/data.json` — Exported agile data for git sync (tracked via `.pm/.gitignore` negation)

---

## Session 121.2: Tauri Repo-Awareness

**Core principle**: Every process finds `.pm/` via `git rev-parse --show-toplevel`. Zero environment variables for **directory and binary discovery**. Runtime config env vars (`PM_SERVER_PORT`, `PM_LOG_LEVEL`, etc.) remain — they are 12-factor parent→child config, a separate concern.

**Files Modified:**
- `backend/crates/pm-config/src/config.rs` — `config_dir()` uses git, not env vars
- `backend/crates/pm-config/src/tests/mod.rs` — Tests use `config_dir_from_git()` + git init
- `backend/crates/pm-config/src/tests/port_file.rs` — Remove `PM_CONFIG_DIR` env var usage
- `backend/config.example.toml` — Remove env var comment
- `.pm/config.toml` — Remove env var comment
- `desktop/src-tauri/Cargo.toml` — Add `pm-config = { workspace = true }`
- `desktop/src-tauri/src/lib.rs` — Use `Config::config_dir()` with double-click fallback, add `PmDir` newtype for managed state
- `desktop/src-tauri/src/identity/mod.rs` — Move `user.json` from global `app_data_dir()` to `<repo>/.pm/` via `PmDir` state (**additional scope found during code audit** — `user.json` was hardcoded to `~/Library/Application Support/` like `server_dir` was)
- `desktop/src-tauri/src/server/lifecycle.rs` — Remove `PM_CONFIG_DIR` env, remove `PM_SERVER_BIN` env, fix `find_server_binary()`, set child process cwd to repo root
- `backend/crates/pm-cli/src/commands.rs` — Add `Desktop` variant
- `backend/crates/pm-cli/src/main.rs` — Add `launch_desktop()`, `find_tauri_binary()`
- `CLAUDE_FUCKED_UP.md` — Document correct fix

**Verification:** `just check-backend && just clippy-backend && just test-backend`

---

## Session 121.3: Release Distribution

**Files Created:**
- `install.sh` — macOS/Linux installer (writes `.pm/bin/config.json`)
- `install.ps1` — Windows installer (writes `.pm\bin\config.json`)

**Files Modified:**
- `justfile` — Distribution variables and commands

**Key addition vs original plan:** Install scripts write `.pm/bin/config.json` with `{"repo_root": "<absolute_path>"}` so Tauri can find the repo when double-clicked outside a terminal.

**Verification:** `just --list | grep archive && bash -n install.sh`

---

## Session 121.4: Data Sync

**Core problem:** Binary SQLite files (`data.db`) cannot be merged by git. When two developers modify work items independently, git produces an unresolvable binary conflict. The solution is SQLite for local performance + JSON export for git distribution.

**Files Created:**
- `backend/crates/pm-cli/src/sync_commands.rs` — `pm sync export` and `pm sync import` commands

**Files Modified:**
- `.pm/.gitignore` — Track `data.json`, ignore `data.db`
- `backend/crates/pm-cli/src/commands.rs` — Add `Sync` variant
- `backend/crates/pm-cli/src/main.rs` — Add sync handler

**Key design decisions:**
- Conflict resolution: Timestamp-based "last write wins" (each entity has `updated_at`)
- UUIDs prevent ID collisions between developers
- Git hooks (pre-commit auto-export, post-merge auto-import) documented but user-installed

**Verification:** `just check-rs-cli && just test-rs-cli`

---

## HOTFIX: `.pm/.gitignore` (MUST DO Before 121.2)

**CRITICAL — SHIPPED BUG**: Session 121.1 shipped with `!data.db` in `.pm/.gitignore`, force-tracking binary SQLite into git. Binary files CANNOT be merged — two developers editing work items independently produces an **unresolvable binary conflict**. This must be fixed before any further work.

**Fix** (5 minutes):

1. Replace the entire `.pm/.gitignore` with the corrected version from 121.1 Step 5 (which has been updated to use `data.db` + `!data.json` instead of `!data.db`)
2. Run `git rm --cached .pm/data.db 2>/dev/null || true`
3. Commit: `git add .pm/.gitignore && git commit -m "Hotfix: stop tracking binary data.db, track data.json instead"`

**Verification**:
```bash
git check-ignore .pm/data.db      # Should output: .pm/data.db
git check-ignore .pm/data.json    # Should output nothing (tracked)
```

This hotfix was originally identified as 121.4 Step 1 but is pulled forward because **the bug is already shipped**.

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check-backend` passes
- [ ] `just test-backend` passes
- [ ] No uncommitted changes in working tree
- [ ] `.pm/.gitignore` hotfix applied (see above)

---

## Complete Files Summary

### Create (4 files)

| File | Sub-Session | Purpose |
|------|-------------|---------|
| `.pm/.gitignore` | 121.1 + 121.4 | Selective tracking: `data.json` + `config.toml` tracked, `data.db` + runtime files ignored |
| `install.sh` | 121.3 | macOS/Linux install script (writes `config.json`) |
| `install.ps1` | 121.3 | Windows install script (writes `config.json`) |
| `backend/crates/pm-cli/src/sync_commands.rs` | 121.4 | `pm sync export` and `pm sync import` commands |

### Modify (17 files)

| File | Sub-Session | Change |
|------|-------------|--------|
| `Cargo.toml` (root) | 121.1 | Add `[workspace.package]` |
| `backend/crates/pm-cli/Cargo.toml` | 121.1 | Inherit workspace metadata |
| `backend/pm-server/Cargo.toml` | 121.1 | Inherit workspace metadata |
| `backend/crates/pm-config/Cargo.toml` | 121.1 | Inherit workspace metadata |
| `.gitignore` | 121.1 | Remove `Cargo.lock`, `.pm/`, `.server`; add `dist/` |
| `justfile` | 121.1 + 121.3 | `.server` to `.pm`, distribution commands |
| `backend/config.example.toml` | 121.1 + 121.2 | Update `.server/` to `.pm/`, remove env var comment |
| `desktop/src-tauri/tauri.conf.json` | 121.1 | Resource source path |
| `backend/crates/pm-config/src/config.rs` | 121.2 | `config_dir()` uses git, add `config_dir_from_git()` |
| `backend/crates/pm-config/src/tests/mod.rs` | 121.2 | Replace `PM_CONFIG_DIR` env var with git init in temp dir |
| `desktop/src-tauri/Cargo.toml` | 121.2 | Add `pm-config` dependency |
| `desktop/src-tauri/src/lib.rs` | 121.2 | Git-based `server_dir`, `PmDir` managed state, double-click fallback |
| `desktop/src-tauri/src/identity/mod.rs` | 121.2 | Move `user.json` from global `app_data_dir()` to `<repo>/.pm/` via `PmDir` |
| `desktop/src-tauri/src/server/lifecycle.rs` | 121.2 | Remove env vars, fix binary discovery, set child cwd |
| `backend/crates/pm-cli/src/commands.rs` | 121.2 + 121.4 | Add `Desktop` and `Sync` variants |
| `backend/crates/pm-cli/src/main.rs` | 121.2 + 121.4 | `launch_desktop()`, `find_tauri_binary()`, sync handlers |
| `CLAUDE_FUCKED_UP.md` | 121.2 | Document correct fix and port 0 trap |

### Newly Git-Tracked (2 files)

| File | Sub-Session | Purpose |
|------|-------------|---------|
| `Cargo.lock` | 121.1 | Reproducible builds for binary project |
| `.pm/data.json` | 121.4 | Exported agile board state (JSON, git-mergeable) — created by `pm sync export` |

**NOT tracked** (gitignored): `.pm/data.db` — binary SQLite cannot be merged by git. `data.json` is the sync format.

---

## Expected Directory Structure

After all three sub-sessions are complete, the repo should have this layout:

```
<any-repo>/                                        # Any git repository (any language)
├── .pm/                                           # Project management data
│   ├── .gitignore                                 # Selective tracking rules (121.1)
│   ├── config.toml                                # Server config          ← git-tracked
│   ├── user.json                                  # Desktop user identity  ← git-tracked
│   ├── data.db                                    # SQLite database        ← gitignored (local)
│   ├── data.json                                  # JSON export            ← git-tracked (sync)
│   ├── data.db-wal                                #   WAL runtime file     ← gitignored
│   ├── data.db-shm                                #   SHM runtime file     ← gitignored
│   ├── server.json                                # Port discovery file    ← gitignored (ephemeral)
│   ├── server.lock                                # Process lock           ← gitignored (ephemeral)
│   ├── logs/                                      # Server logs            ← gitignored
│   │   └── pm-server.log
│   ├── tauri/                                     # Tauri desktop runtime  ← gitignored
│   │   ├── config.toml                            #   Tauri's ServerConfig (port range, restarts, etc.)
│   │   └── tauri.log                              #   Tauri's own logs
│   └── bin/                                       # Installed binaries     ← gitignored (121.3)
│       ├── pm                                     #   CLI
│       ├── pm-server                              #   Backend server
│       ├── config.json                            #   {"repo_root": "/abs/path"} for Tauri
│       └── Project Manager.app/                   #   macOS Tauri app (or plain binary on Linux)
│           └── Contents/MacOS/project-manager
├── install.sh                                     # macOS/Linux installer  (121.3)
├── install.ps1                                    # Windows installer      (121.3)
└── dist/                                          # Build artifacts        ← gitignored
    └── pm-0.1.0-aarch64-apple-darwin.tar.gz       #   Release archive
```

**How each process finds `.pm/`:**

```
┌──────────────────────────────────────────────────────────────┐
│ pm (CLI)          → git rev-parse --show-toplevel + /.pm/    │
│ pm-server         → git rev-parse --show-toplevel + /.pm/    │
│ Tauri (terminal)  → git rev-parse --show-toplevel + /.pm/    │
│ Tauri (dbl-click) → reads .pm/bin/config.json → repo_root   │
└──────────────────────────────────────────────────────────────┘
  All go through Config::config_dir() in pm-config,
  except Tauri double-click which falls back to config.json.
```

---

## Key Architecture Decisions

1. **`DEFAULT_PORT` stays at 8000**: Tauri's port scanning (8000-8100) is correct for desktop mode. The pm-server config uses `port = 0` (auto-assign) for direct CLI mode. Different use cases, different defaults.

2. **`git rev-parse --show-toplevel` is the primary discovery mechanism**: Every process uses a fallback chain: git repo root → `config.json` next to binary → `~/.pm/` global. No environment variables. Fallbacks handle Docker, system-wide installs, and non-git contexts.

3. **Selective git tracking via `.pm/.gitignore`**: The `.pm/` directory is no longer blanket-ignored at the root level. Instead, `.pm/.gitignore` ignores runtime files (including `data.db`) while allowing `data.json` and `config.toml` to be tracked. Binary SQLite cannot be merged by git — `data.json` is the git-friendly export format.

4. **`Cargo.lock` is committed**: Binary projects should commit lockfiles for reproducible builds. This is the Rust community convention for projects that produce executables.

5. **`config.json` for double-click Tauri**: When Tauri is launched outside a terminal (no git context), it reads `config.json` next to its binary to find the repo root. This file is written by the installer.

6. **No environment variables for discovery**: `PM_CONFIG_DIR`, `PM_SERVER_BIN`, and `PM_TAURI_BIN` are all eliminated. Directory discovery uses the 3-level fallback chain (git → config.json → global). Binary discovery uses sibling-to-exe + `.pm/bin/` + PATH. Runtime config env vars (`PM_SERVER_PORT`, `PM_AUTH_ENABLED`, etc.) used by lifecycle.rs to configure the spawned pm-server child process are a separate concern and remain unchanged.

7. **SQLite + JSON sync pattern**: `data.db` provides fast local queries. `data.json` (exported via `pm sync export`) is the git-tracked format for team sync. Pre-commit hooks auto-export, post-merge hooks auto-import. Conflict resolution uses timestamp-based "last write wins".

---

## Final Verification

After all four sub-sessions are complete:

```bash
# Full code validation
just check-backend && just clippy-backend && just test-backend

# Development flow
just dev
# In another terminal:
cat .pm/server.json              # Port file in repo-local .pm/
cargo run -p pm-cli -- project list --pretty  # Auto-discovers server
# Close Tauri

# Standalone server
cargo run -p pm-server &
cat .pm/server.json
cargo run -p pm-cli -- project list --pretty
kill %1

# Distribution
just release-build
tar tzf dist/pm-*.tar.gz

# Data sync
cargo run -p pm-cli -- sync export --output .pm/data.json
git add .pm/data.json
git diff --cached .pm/data.json     # Should show JSON, not binary
```
