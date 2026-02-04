# Standalone/Portable Executable Workflow

This document describes how to build and distribute the Blazor Agile Board as a portable, self-contained executable that works with git-controlled data directories.

## Goal

- Portable executable that can be placed anywhere
- Database and config files (`.pm` directory) live in git repo
- Executable itself NOT in git (gitignored)
- Executable looks for `.pm` relative to where it's run from

## Directory Structure

```
your-repo/
├── .pm/                    # In git - database, configs
│   ├── config.json
│   └── tenants/
│       └── main.db
├── pm                      # NOT in git - portable executable
├── .gitignore
└── README.md
```

## Building Portable Executable

### Tauri Configuration

In `tauri.conf.json`, configure for portable builds:

```json
{
  "bundle": {
    "targets": [
      "app",      // macOS: Portable .app bundle
      "appimage"  // Linux: Portable AppImage
    ],
    "nsis": {
      "portable": true  // Windows: Portable .exe
    }
  }
}
```

### Platform-specific Portable Formats

**macOS:**
- `app` - Portable .app bundle (no installer)

**Windows:**
- `nsis` with `portable: true` - Creates portable .exe instead of installer

**Linux:**
- `appimage` - Fully portable single-file executable

## Application Code: Data Directory Resolution

The app should look for `.pm` in the current working directory (where user runs it from):

```rust
use std::env;
use std::path::PathBuf;

fn get_pm_directory() -> PathBuf {
    // Use current working directory (where user runs the app from)
    let cwd = env::current_dir()
        .expect("Failed to get current directory");

    let pm_dir = cwd.join(".pm");

    // Create if doesn't exist
    std::fs::create_dir_all(&pm_dir)
        .expect("Failed to create .pm directory");

    pm_dir
}
```

**Key point:** Use `env::current_dir()` not `env::current_exe()`. This way the app looks for `.pm` where it's **run from**, not where the executable lives.

### Alternative: CLI Argument for Custom Location

Even more flexible:

```rust
let pm_dir = std::env::args()
    .nth(1)
    .map(PathBuf::from)
    .unwrap_or_else(|| env::current_dir().unwrap().join(".pm"));
```

Usage:
```bash
./pm                  # Uses ./.pm
./pm /path/to/.pm     # Uses custom location
```

## .gitignore Configuration

```gitignore
# Ignore the executable
pm
pm.exe
*.app

# Keep .pm directory in git
# (no .pm/ entry here)
```

## Distribution Mechanisms

### Option 1: cargo-binstall (Recommended for Rust users)

Downloads pre-built binaries instead of compiling. Works with GitHub Releases.

**Setup:**
1. Publish releases to GitHub with binaries attached
2. Users install with:

```bash
# Install cargo-binstall first (one-time)
cargo install cargo-binstall

# Then install your app to current directory
cargo binstall pm --root .
```

This installs to `./bin/pm` instead of globally.

**Requirements:**
- GitHub Releases with binaries named following cargo-binstall conventions
- Optional: `.cargo/binstall.toml` in your repo to customize download URLs

### Option 2: Custom justfile Command (Recommended)

Add to your `justfile`:

```bash
install-app:
    #!/usr/bin/env bash
    LATEST=$(curl -s https://api.github.com/repos/youruser/yourrepo/releases/latest | grep tag_name | cut -d '"' -f 4)
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    curl -L "https://github.com/youruser/yourrepo/releases/download/${LATEST}/pm-${OS}-${ARCH}" -o pm
    chmod +x pm
    echo "Installed pm ${LATEST}"
```

Usage:
```bash
just install-app
```

### Option 3: Simple Install Script

Create `install.sh` in your repo:

```bash
#!/usr/bin/env bash
set -e

# Detect platform
case "$(uname -s)" in
    Darwin) OS="macos" ;;
    Linux)  OS="linux" ;;
    MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
    *) echo "Unsupported OS"; exit 1 ;;
esac

ARCH=$(uname -m)
LATEST=$(curl -s https://api.github.com/repos/youruser/yourrepo/releases/latest | grep tag_name | cut -d '"' -f 4)

echo "Installing pm ${LATEST} for ${OS}-${ARCH}..."
curl -L "https://github.com/youruser/yourrepo/releases/download/${LATEST}/pm-${OS}-${ARCH}" -o pm
chmod +x pm

echo "✓ Installed to ./pm"
```

Usage:
```bash
curl -sSL https://raw.githubusercontent.com/youruser/yourrepo/main/install.sh | bash
```

### Option 4: Publish to crates.io

If you publish to crates.io, users can:

```bash
# Install globally
cargo install pm

# Or install to current directory
cargo install pm --root .
```

**Note:** This compiles from source (slow), not pre-built binaries.

## Recommended Workflow

1. **Build portable binaries**: `cargo tauri build` with portable configuration
2. **Upload to GitHub Releases**: Manually or via CI/CD
3. **Add `just install-app` command**: See Option 2 above
4. **Users run `just install-app`** in their repo to get latest binary

This keeps it simple and doesn't require publishing to crates.io or setting up cargo-binstall conventions.

## Usage Pattern

```bash
# User installs executable in repo root
just install-app

# Run from repo root
./pm

# App finds ./.pm/ and uses it
# All changes to .pm/ can be committed to git
git add .pm/
git commit -m "Update database"
```

## GitHub Release Workflow

To automate releases, you can use GitHub Actions to build and publish binaries on tag pushes. This ensures consistent builds across platforms and makes distribution seamless.
