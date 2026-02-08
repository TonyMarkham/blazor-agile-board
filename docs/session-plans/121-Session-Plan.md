# Session 121: Portable Distribution + Repo-Aware Tauri

## Context

Two previously separate sessions merged into one coherent goal: **make all tools (CLI, server, desktop) share a single repo-local `.pm/` directory** for config, database, port discovery, and installed binaries.

**The core bug** (documented in `CLAUDE_FUCKED_UP.md`): Tauri always uses `~/Library/Application Support/com.projectmanager.app/.server/` as its server directory, while the CLI uses `<cwd>/.pm/`. They never overlap, so the CLI cannot discover a Tauri-spawned server's port file. Session 120 planning incorrectly claimed this was "fully solved" without reading the Tauri code.

**The wrong fix** (proposed in `CLAUDE_FUCKED_UP.md`): Changing `DEFAULT_PORT` from 8000 to 0. This would break Tauri entirely because `is_available(0)` always returns true (port 0 means "OS picks any port"), causing an infinite restart loop.

**The correct fix**: A 3-line change in `desktop/src-tauri/src/lib.rs` that checks the `PM_CONFIG_DIR` environment variable before falling back to the global app data directory. Everything downstream (lifecycle.rs, pm-server, pm-config, CLI port file discovery) already works correctly.

---

## Sub-Session Breakdown

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[121.1](121.1-Session-Plan.md)** | Infrastructure: workspace metadata, `.pm/` layout, `.gitignore`, justfile migration | ~25k | ✅ **COMPLETE** |
| **[121.2](121.2-Session-Plan.md)** | Tauri repo-awareness: `PM_CONFIG_DIR`, `pm desktop` command, `just dev` | ~35k | Pending |
| **[121.3](121.3-Session-Plan.md)** | Release distribution: justfile commands, install scripts | ~30k | Pending |

---

## Session 121.1: Infrastructure ✅ **COMPLETE**

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
- `.pm/data.db` — Shared database (tracked via `.pm/.gitignore` negation)

**Verification Results:**
- ✅ `just check-backend` passes
- ✅ `just test-backend` all tests pass
- ✅ Workspace version inheritance verified (all crates at 0.1.0)
- ✅ `.pm/config.toml` created via `just setup-config`
- ⚠️ Pre-existing clippy warning in pm-config (collapsible_if), not introduced by 121.1

**Deviations from Plan (Intentional):**
- Description fields skipped in member crates (optional, user decision)
- All member crates updated (not just pm-cli, pm-server, pm-config) for complete consistency

---

## Session 121.2: Tauri Repo-Awareness

**Files Modified:**
- `desktop/src-tauri/src/lib.rs` — Check `PM_CONFIG_DIR` for `server_dir`
- `backend/crates/pm-cli/src/commands.rs` — Add `Desktop` variant
- `backend/crates/pm-cli/src/main.rs` — Add `launch_desktop()`, `find_tauri_binary()`
- `justfile` — Pass `PM_CONFIG_DIR` in `dev` command
- `CLAUDE_FUCKED_UP.md` — Document correct fix

**Verification:** `just check-backend && cargo run -p pm-cli -- --help`

---

## Session 121.3: Release Distribution

**Files Created:**
- `install.sh` — macOS/Linux installer
- `install.ps1` — Windows installer

**Files Modified:**
- `justfile` — Distribution variables and commands

**Verification:** `just --list | grep archive && bash -n install.sh`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check-backend` passes
- [ ] `just test-backend` passes
- [ ] No uncommitted changes in working tree

---

## Complete Files Summary

### Create (3 files)

| File | Sub-Session | Purpose |
|------|-------------|---------|
| `.pm/.gitignore` | 121.1 | Selective tracking: `data.db` + `config.toml` tracked, runtime files ignored |
| `install.sh` | 121.3 | macOS/Linux install script |
| `install.ps1` | 121.3 | Windows install script |

### Modify (12 files)

| File | Sub-Session | Change |
|------|-------------|--------|
| `Cargo.toml` (root) | 121.1 | Add `[workspace.package]` |
| `backend/crates/pm-cli/Cargo.toml` | 121.1 | Inherit workspace metadata |
| `backend/pm-server/Cargo.toml` | 121.1 | Inherit workspace metadata |
| `backend/crates/pm-config/Cargo.toml` | 121.1 | Inherit workspace metadata |
| `.gitignore` | 121.1 | Remove `Cargo.lock`, `.pm/`, `.server`; add `dist/` |
| `justfile` | 121.1 + 121.2 + 121.3 | `.server` to `.pm`, `PM_CONFIG_DIR` in dev, distribution commands |
| `backend/config.example.toml` | 121.1 | Update `.server/` comments to `.pm/` |
| `desktop/src-tauri/tauri.conf.json` | 121.1 | Resource source path |
| `desktop/src-tauri/src/lib.rs` | 121.2 | `PM_CONFIG_DIR` check for `server_dir` |
| `backend/crates/pm-cli/src/commands.rs` | 121.2 | Add `Desktop` variant |
| `backend/crates/pm-cli/src/main.rs` | 121.2 | Add `launch_desktop()`, `find_tauri_binary()` |
| `CLAUDE_FUCKED_UP.md` | 121.2 | Document correct diagnosis and fix |

### Newly Git-Tracked (2 files)

| File | Sub-Session | Purpose |
|------|-------------|---------|
| `Cargo.lock` | 121.1 | Reproducible builds for binary project |
| `.pm/data.db` | 121.1 | Shared project management database |

---

## Key Architecture Decisions

1. **`DEFAULT_PORT` stays at 8000**: Tauri's port scanning (8000-8100) is correct for desktop mode. The pm-server config uses `port = 0` (auto-assign) for direct CLI mode. Different use cases, different defaults.

2. **`.pm/` is the single source of truth**: All three entry points (CLI, server, desktop) read/write to `<repo>/.pm/` when in repo context. The `PM_CONFIG_DIR` environment variable is the coordination mechanism.

3. **Selective git tracking via `.pm/.gitignore`**: The `.pm/` directory is no longer blanket-ignored at the root level. Instead, `.pm/.gitignore` ignores runtime files while allowing `data.db` and `config.toml` to be tracked.

4. **`Cargo.lock` is committed**: Binary projects should commit lockfiles for reproducible builds. This is the Rust community convention for projects that produce executables.

---

## Final Verification

After all three sub-sessions are complete:

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
```
