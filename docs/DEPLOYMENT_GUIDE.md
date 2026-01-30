# Deployment Guide

This project ships as a **Tauri desktop application** with a Rust backend and a Blazor WASM frontend.

---

## Production Build (Desktop)

```bash
just build
```

This runs the release build pipeline:
- builds the Rust backend
- publishes the WASM frontend
- packages the Tauri desktop app

---

## Release Build (No Packaging)

```bash
just build-release
```

This builds backend + frontend in release mode without Tauri packaging.

---

## Development Build

```bash
just dev
```

This builds and runs the app in development mode.

---

## Configuration

Active config file:
- `.server/config.toml`

Create it from the example:

```bash
just setup-config
```

Config defaults are documented in:
- `backend/config.example.toml`

---

## Environment Overrides

You can override config values with environment variables prefixed with `PM_`.
See `backend/config.example.toml` for the list.
