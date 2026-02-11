# Linux Dependencies

**Purpose**: One-time setup of development tools for building the Blazor Agile Board desktop application on Linux.

**Time Estimate**: ~10-20 minutes (downloads + installations)

**Minimum Requirements**:
- Ubuntu 20.04+, Debian 11+, Fedora 36+, Arch Linux (recent), or equivalent
- ~3GB free disk space
- Internet connection
- sudo access

**Note**: Commands shown for Debian/Ubuntu. See distribution-specific sections for others.

---

## Quick Reference

After completing this setup, you should have:
- ✅ Build essentials (gcc, g++, make, pkg-config)
- ✅ GTK3 and WebKit2GTK development libraries
- ✅ System libraries (libssl, libayatana-appindicator, etc.)
- ✅ Rust 1.93.0+ and Cargo
- ✅ .NET SDK 10.0
- ✅ Tauri CLI
- ✅ SQLx CLI (for database migrations)
- ✅ Just (task runner for build automation)
- ✅ Protocol Buffers compiler (protoc)

---

## Distribution-Specific Quick Start

### Ubuntu / Debian / Pop!_OS / Linux Mint

```bash
# System libraries and build tools
sudo apt update
sudo apt install -y \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf

# Then proceed to Rust, .NET, Tauri CLI sections below
```

### Fedora / RHEL / CentOS

```bash
# System libraries and build tools
sudo dnf update
sudo dnf install -y \
    gcc \
    gcc-c++ \
    make \
    pkgconf \
    curl \
    wget \
    file \
    openssl-devel \
    gtk3-devel \
    webkit2gtk4.1-devel \
    libappindicator-gtk3-devel \
    librsvg2-devel

# Then proceed to Rust, .NET, Tauri CLI sections below
```

### Arch Linux / Manjaro

```bash
# System libraries and build tools
sudo pacman -Syu
sudo pacman -S --needed \
    base-devel \
    curl \
    wget \
    file \
    openssl \
    gtk3 \
    webkit2gtk-4.1 \
    libappindicator-gtk3 \
    librsvg \
    patchelf

# Then proceed to Rust, .NET, Tauri CLI sections below
```

### openSUSE

```bash
# System libraries and build tools
sudo zypper refresh
sudo zypper install -t pattern devel_basis
sudo zypper install -y \
    curl \
    wget \
    file \
    libopenssl-devel \
    gtk3-devel \
    webkit2gtk3-devel \
    libappindicator3-devel \
    librsvg-devel

# Then proceed to Rust, .NET, Tauri CLI sections below
```

---

## 1. System Libraries (Already covered above)

The distribution-specific commands above install:

**Build Tools**:
- `gcc`, `g++`, `make` - C/C++ compiler toolchain
- `pkg-config` / `pkgconf` - Library configuration helper

**Required Libraries**:
- `libssl-dev` / `openssl-devel` - TLS/SSL support
- `libgtk-3-dev` / `gtk3-devel` - GTK3 UI toolkit
- `libwebkit2gtk-4.1-dev` / `webkit2gtk4.1-devel` - WebView component
- `libayatana-appindicator3-dev` / `libappindicator-gtk3-devel` - System tray support
- `librsvg2-dev` / `librsvg-devel` - SVG rendering
- `patchelf` - Binary patching tool (for AppImage)

**Verification**:
```bash
# Check build tools
gcc --version
pkg-config --version

# Check GTK3
pkg-config --modversion gtk+-3.0

# Check WebKit2GTK
pkg-config --modversion webkit2gtk-4.1
```

---

## 2. Rust & Cargo

**Required for**: Tauri backend compilation

**Installation** (Official rustup installer):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

When prompted:
- Select option **1** (default installation)
- Installation takes ~3-5 minutes

**Important**: Restart your terminal after installation (or run `source $HOME/.cargo/env`)

**Verification**:
```bash
rustc --version
cargo --version
```
**Expected output**: Version 1.93.0 or later for both

**Updating Rust**:
```bash
# Update rustup itself (the installer/manager)
rustup self update

# Update Rust toolchain to latest stable
rustup update stable

# Verify new version
rustc --version
```

**Troubleshooting**:
- `command not found: cargo` → Restart terminal or check `~/.cargo/bin` is in PATH
- Check for available updates: `rustup check`

---

## 3. .NET SDK 10.0

**Required for**: Blazor frontend compilation

**Installation**:

### Ubuntu / Debian

```bash
# Add Microsoft package repository
wget https://packages.microsoft.com/config/ubuntu/$(lsb_release -rs)/packages-microsoft-prod.deb -O packages-microsoft-prod.deb
sudo dpkg -i packages-microsoft-prod.deb
rm packages-microsoft-prod.deb

# Install .NET SDK
sudo apt update
sudo apt install -y dotnet-sdk-10.0
```

### Fedora

```bash
# Add Microsoft package repository
sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc
sudo wget -O /etc/yum.repos.d/microsoft-prod.repo https://packages.microsoft.com/config/fedora/$(rpm -E %fedora)/prod.repo

# Install .NET SDK
sudo dnf install -y dotnet-sdk-10.0
```

### Arch Linux

```bash
# .NET is in community repository
sudo pacman -S dotnet-sdk
```

### Manual Install (All Distributions)

If the above doesn't work, use the official installer script:
```bash
wget https://dot.net/v1/dotnet-install.sh
chmod +x dotnet-install.sh
./dotnet-install.sh --channel 10.0
```

Add to PATH in `~/.bashrc` or `~/.zshrc`:
```bash
export DOTNET_ROOT=$HOME/.dotnet
export PATH=$PATH:$HOME/.dotnet
```

**Verification**:
```bash
dotnet --version
```
**Expected output**: `10.0.x` (e.g., 10.0.0, 10.0.1)

**Troubleshooting**:
- `dotnet: command not found` → Check PATH configuration
- Multiple SDKs can coexist; check: `dotnet --list-sdks`

---

## 4. Tauri CLI

**Required for**: Building and running the desktop application

**Installation** (via Cargo - no Node.js needed):
```bash
cargo install tauri-cli
```

This compiles the Tauri CLI from source. Takes ~2-3 minutes (one-time compilation, cached afterward).

**Verification**:
```bash
cargo tauri --version
```
**Expected output**: `tauri-cli 1.x.x` or later

**Troubleshooting**:
- Compilation errors → Ensure system libraries installed (step 1)
- Linker errors → Install `build-essential` or equivalent
- `cargo: command not found` → Ensure Rust installation completed (step 2)

---

## 5. SQLx CLI

**Required for**: Database migrations and compile-time SQL verification

**What it does**: SQLx CLI manages database migrations and validates SQL queries at compile time. This project uses SQLite with SQLx migrations in `backend/crates/pm-db/migrations/`.

**Installation** (SQLite-only, no PostgreSQL/MySQL dependencies):
```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

This installs only SQLite support, making it faster and lighter. Takes ~2-3 minutes.

**Verification**:
```bash
sqlx --version
```
**Expected output**: `sqlx-cli 0.8.x` or later

**Common commands** (you'll use these):
```bash
# Run migrations (from backend/ directory)
sqlx migrate run --database-url sqlite:.pm/data.db

# Create new migration
sqlx migrate add create_new_table

# Revert last migration
sqlx migrate revert --database-url sqlite:.pm/data.db
```

**Troubleshooting**:
- If installed with wrong features, reinstall: `cargo install sqlx-cli --force --no-default-features --features sqlite`
- Migration errors → Check database path is correct
- Compile-time verification requires `DATABASE_URL` env var (optional for this project)

---

## 6. Just (Task Runner)

**Required for**: Build automation and development workflows

**What it does**: Just is a command runner (like Make, but better). This project uses a `justfile` to automate common tasks like `just dev` (run development build) and `just build` (production build).

**Installation**:
```bash
cargo install just
```

Takes ~1-2 minutes.

**Verification**:
```bash
just --version
```
**Expected output**: `just 1.x.x` or later

**Common commands** (from project root):
```bash
# List all available tasks
just --list

# Run development build
just dev

# Run production build
just build

# Run backend tests
just test-backend

# Run frontend tests
just test-frontend

# Clean all build artifacts
just clean
```

**Why Just instead of Make?**
- Simpler syntax than Make
- Better error messages
- Cross-platform compatibility
- Native to Rust ecosystem

**Troubleshooting**:
- `just: command not found` → Ensure `~/.cargo/bin` is in PATH
- See project's `justfile` for all available tasks

---

## 7. Protocol Buffers (protoc)

**Required for**: Compiling `.proto` files into Rust code for WebSocket communication

**What it does**: Protocol Buffers (protobuf) is a binary serialization format used for efficient real-time communication. The `pm-proto` crate compiles `proto/messages.proto` into Rust code during build.

**Installation**:

### Ubuntu / Debian
```bash
sudo apt install -y protobuf-compiler
```

### Fedora / RHEL
```bash
sudo dnf install -y protobuf-compiler
```

### Arch Linux
```bash
sudo pacman -S protobuf
```

### Manual Install (All Distributions)
If package manager version is too old:
```bash
# Download latest release from GitHub
PROTOC_VERSION=21.12  # Check https://github.com/protocolbuffers/protobuf/releases
curl -LO https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip
unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip -d $HOME/.local
rm protoc-${PROTOC_VERSION}-linux-x86_64.zip

# Add to PATH in ~/.bashrc or ~/.zshrc
export PATH="$HOME/.local/bin:$PATH"
```

**Verification**:
```bash
protoc --version
```
**Expected output**: `libprotoc 3.x.x` or later

**CRITICAL First-Time Setup**:

The `pm-proto` crate outputs generated Rust code to `src/generated/`. **This directory must exist before the first build**, otherwise you'll encounter:
```
Failed to compile protobuf definitions: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

Create it with:
```bash
# From repository root
mkdir -p backend/crates/pm-proto/src/generated
```

**Why?** The build script (`build.rs`) uses `prost_build` to compile `.proto` files, but it doesn't create the output directory automatically.

**This is a one-time setup** - once created, the directory persists and future builds work automatically.

**ARM64 Linux (Raspberry Pi) Configuration**:

On ARM64 Linux systems (Raspberry Pi), the bundled protoc binary in the `Grpc.Tools` NuGet package may crash with exit code 139 (segmentation fault). To use the system-installed protoc instead, set the `PROTOBUF_PROTOC` environment variable:

```bash
# Add to ~/.bashrc or ~/.zshrc for permanent configuration
export PROTOBUF_PROTOC=/usr/bin/protoc

# Or set temporarily for a single build
PROTOBUF_PROTOC=/usr/bin/protoc dotnet build
```

**To make this permanent**, add it to your shell configuration:
```bash
# For bash
echo 'export PROTOBUF_PROTOC=/usr/bin/protoc' >> ~/.bashrc
source ~/.bashrc

# For zsh
echo 'export PROTOBUF_PROTOC=/usr/bin/protoc' >> ~/.zshrc
source ~/.zshrc
```

**Why this works**: The `Grpc.Tools` package checks the `PROTOBUF_PROTOC` environment variable before using its bundled protoc binary. This allows you to override with the system version on platforms where the bundled binary is incompatible.

**Troubleshooting**:
- `protoc: command not found` → Install protobuf-compiler or add to PATH
- Build fails with "No such file or directory" (Rust) → Create `backend/crates/pm-proto/src/generated/`
- `.NET build error: "protoc exited with code 139"` (ARM64) → Set `PROTOBUF_PROTOC=/usr/bin/protoc` environment variable (see ARM64 section above)
- Proto syntax errors → Check `proto/messages.proto` for valid protobuf3 syntax

---

## Complete Environment Verification

Run all verification commands together:

```bash
echo "=== Build Tools ==="
gcc --version | head -1
pkg-config --version

echo -e "\n=== System Libraries ==="
pkg-config --modversion gtk+-3.0
pkg-config --modversion webkit2gtk-4.1

echo -e "\n=== Rust & Cargo ==="
rustc --version
cargo --version

echo -e "\n=== .NET SDK ==="
dotnet --version

echo -e "\n=== Tauri CLI ==="
cargo tauri --version

echo -e "\n=== SQLx CLI ==="
sqlx --version

echo -e "\n=== Just Task Runner ==="
just --version

echo -e "\n=== Protocol Buffers Compiler ==="
protoc --version
```

**All commands should succeed** with version numbers matching the requirements above.

---

## Common Issues

### "Package 'webkit2gtk-4.1' not found"
**Solution**:
- Ubuntu 20.04: Use `libwebkit2gtk-4.0-dev` instead
- Update your package lists: `sudo apt update`
- Check distribution-specific package names above

### "error: linker `cc` not found"
**Solution**: Install build essentials
```bash
sudo apt install build-essential  # Ubuntu/Debian
sudo dnf groupinstall "Development Tools"  # Fedora
```

### Cargo/Rust commands not found after installation
**Solution**: Restart terminal or run `source $HOME/.cargo/env`

### GTK or WebKit warnings during build
**Solution**: Usually non-critical, but ensure you have `-dev` packages installed

### "error: failed to run custom build command for `openssl-sys`"
**Solution**: Install OpenSSL development headers
```bash
sudo apt install libssl-dev  # Ubuntu/Debian
sudo dnf install openssl-devel  # Fedora
```

---

## Next Steps

Dependencies installed? Proceed to:
- **Development builds**: [LINUX_DEV_BUILD_README.md](LINUX_DEV_BUILD_README.md)
- **Production builds**: [LINUX_PROD_BUILD_README.md](LINUX_PROD_BUILD_README.md)

---

## Maintenance

**Updating dependencies**:
```bash
# Update system packages
sudo apt update && sudo apt upgrade  # Ubuntu/Debian
sudo dnf update  # Fedora
sudo pacman -Syu  # Arch

# Update Rust (two-step process)
rustup self update      # Update rustup itself
rustup update stable    # Update Rust toolchain

# Update .NET SDK (via package manager or manual install)
sudo apt install --only-upgrade dotnet-sdk-10.0

# Update Cargo tools
cargo install tauri-cli --force
cargo install sqlx-cli --force --no-default-features --features sqlite
cargo install just --force

# Update protoc (if installed via package manager)
sudo apt install --only-upgrade protobuf-compiler  # Ubuntu/Debian
sudo dnf update protobuf-compiler  # Fedora
sudo pacman -S protobuf  # Arch (auto-updates with system)
```

**Checking for updates**:
```bash
rustup check            # Shows available Rust updates
dotnet --list-sdks      # Shows installed .NET versions
cargo install --list    # Shows installed Cargo tools
```

---

## Linux-Specific Notes

**Wayland Support**:
- Tauri supports both X11 and Wayland
- Set `GDK_BACKEND=wayland` environment variable to force Wayland
- Some systems auto-detect

**AppImage Permissions**:
After building, make AppImage executable:
```bash
chmod +x path/to/your-app.AppImage
```

**System Tray on Different Desktop Environments**:
- GNOME: May require extension for system tray
- KDE Plasma: Works out of the box
- XFCE/MATE/Cinnamon: Works out of the box

---

**Last Updated**: 2026-01-23
