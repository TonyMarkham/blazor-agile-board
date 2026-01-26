# FIX: Inconsistent Config File Workflow

**Created**: 2026-01-25
**Priority**: Medium
**Status**: Technical Debt

---

## The Problem

Two config files, two completely different creation workflows:

| Config | Location | How Created | Template Source |
|--------|----------|-------------|-----------------|
| pm-server | `.server/config.toml` | Extracted from bundled resources | `../../.pm/config.toml` |
| Tauri | `.tauri/config.toml` | Runtime `load_or_create()` with Rust defaults | None (hardcoded) |

This is inconsistent garbage:
- One has a bundled template, one doesn't
- One is extracted on first run, one is generated
- Defaults live in different places (TOML file vs Rust code)
- No single source of truth

---

## Current Implementation

**pm-server config** (lib.rs lines 57-69):
```rust
let pm_config_dest = server_dir.join(PM_SERVER_CONFIG_FILENAME);
if !pm_config_dest.exists()
    && let Ok(resource_dir) = app.path().resource_dir()
{
    let pm_config_src = resource_dir
        .join(SERVER_DATA_DIR)
        .join(PM_SERVER_CONFIG_FILENAME);
    if pm_config_src.exists() {
        std::fs::copy(&pm_config_src, &pm_config_dest)?;
    }
}
```

**Tauri config** (lib.rs line 72):
```rust
let config = ServerConfig::load_or_create(&tauri_dir)
```

---

## Desired State

Both configs should work the same way. Options:

### Option A: Both use bundled templates
- Bundle `config.example.toml` for both
- Extract on first run
- Single source of truth in TOML files
- Users can see/edit templates

### Option B: Both use runtime defaults
- No bundled templates
- Both use `load_or_create()` with Rust defaults
- Defaults documented in code
- Cleaner bundle, but defaults hidden in code

### Option C: Unified config
- Single config file for both pm-server and Tauri
- pm-server reads relevant sections
- Tauri reads relevant sections
- Simplest for users

---

## Recommendation

**Option A** - Both use bundled templates:
- Consistent workflow
- Templates serve as documentation
- Users can customize before first run
- Easy to diff against defaults

---

## Files to Change

- `desktop/src-tauri/src/lib.rs` - Unify config extraction
- `desktop/src-tauri/src/server/config.rs` - Remove `load_or_create`, use extraction pattern
- `desktop/src-tauri/tauri.conf.json` - Bundle Tauri config template
- Create `tauri.config.example.toml` template

---

## Session Reference

Identified during Session 44, Step 17 (bundling config).
