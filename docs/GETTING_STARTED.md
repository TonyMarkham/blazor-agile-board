# Getting Started

This guide gets you from a clean checkout to a working build using the **justfile** commands defined in this repo.

## Prerequisites

- **just** (task runner)
- **.NET SDK** (version required by the repo)
- **Rust toolchain** (for backend crates)
- **Tauri prerequisites** (if you plan to run the desktop app)

> Tip: If you’re not sure which versions are required, start with the latest stable .NET SDK and Rust toolchain.

## Quick Start (Recommended)

From the repo root:

```bash
just setup-config
just restore
just check
```

What these do:
- `just setup-config` creates `.server/config.toml` from `backend/config.example.toml` (if missing)
- `just restore` restores frontend NuGet + backend Cargo dependencies
- `just check` runs full validation (restore → check/build → lint → test)

## Build and Run (Desktop App)

```bash
just dev
```

This runs a debug build and launches the Tauri desktop app.

## Frontend-Only Builds

```bash
just build-frontend
just test-frontend
```

## Backend-Only Builds

```bash
just build-backend
just test-backend
```

## Configuration

- Example config: `backend/config.example.toml`
- Active config: `.server/config.toml`

To (re)create the config:

```bash
just setup-config
```

### Environment Overrides

Configuration can be overridden by environment variables with the `PM_` prefix.
See the comments in `backend/config.example.toml` for the full list.

## Useful Commands

```bash
just help
```

This prints all available commands and their descriptions.
