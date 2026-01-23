# Linux Production Build

**Purpose**: Create optimized, distributable packages for Linux (.deb, .AppImage, .rpm).

**Time**: ~5-10 minutes for full release build

**Prerequisites**: [LINUX_DEPENDENCIES_README.md](LINUX_DEPENDENCIES_README.md) completed

---

## Quick Start

```bash
cd desktop
cargo tauri build
```

**Output**: Distribution packages in:
```
desktop/src-tauri/target/release/bundle/
├── deb/
│   └── blazor-agile-board_0.1.0_amd64.deb     # Debian/Ubuntu package
├── appimage/
│   └── blazor-agile-board_0.1.0_amd64.AppImage # Universal Linux binary
└── rpm/
    └── blazor-agile-board-0.1.0-1.x86_64.rpm   # Fedora/RHEL package
```

---

## Package Types

### AppImage (Recommended for Universal Distribution)

**Format**: Self-contained executable

**Pros**:
- ✅ Works on all distributions
- ✅ No installation required
- ✅ Portable (run from USB)
- ✅ Includes all dependencies
- ✅ Easy rollback (just keep old file)

**Cons**:
- ⚠️ Larger file size (~25-40 MB)
- ⚠️ No automatic updates (use AppImageUpdate)
- ⚠️ Manual desktop integration

**Use case**: General Linux users, distribution-agnostic releases

---

### .deb Package (Debian/Ubuntu/Mint/Pop!_OS)

**Format**: Debian package manager

**Pros**:
- ✅ Native package management
- ✅ Automatic dependency resolution
- ✅ Desktop integration
- ✅ Update via `apt upgrade`
- ✅ Familiar to Debian/Ubuntu users

**Cons**:
- ⚠️ Debian-based distros only
- ⚠️ Requires repository or manual download

**Use case**: Debian/Ubuntu users, PPA distribution

---

### .rpm Package (Fedora/RHEL/openSUSE)

**Format**: RPM Package Manager

**Pros**:
- ✅ Native package management
- ✅ Automatic dependency resolution
- ✅ Desktop integration
- ✅ Update via `dnf upgrade`
- ✅ Familiar to Fedora/RHEL users

**Cons**:
- ⚠️ Red Hat-based distros only
- ⚠️ Requires repository or manual download

**Use case**: Fedora/RHEL/CentOS users, Copr distribution

---

## Full Production Build Process

### 1. Pre-Build Checklist

```bash
# Update version in Cargo.toml
cd desktop/src-tauri
# Edit Cargo.toml: version = "0.2.0"

# Update version in tauri.conf.json
# Edit tauri.conf.json: "version": "0.2.0"

# Commit version bump
git add Cargo.toml tauri.conf.json
git commit -m "chore: bump version to 0.2.0"
git tag v0.2.0
```

### 2. Clean Build Environment

```bash
# Clean previous builds
cargo clean

# Clean frontend artifacts
cd ../../frontend
dotnet clean

cd ../desktop
```

### 3. Run Tests

```bash
# Backend tests
cd ../backend
cargo test --workspace --release

# Frontend tests
cd ../frontend
dotnet test --configuration Release

# All tests must pass before production build
```

### 4. Build Frontend (Release)

```bash
cd frontend
dotnet publish ProjectManagement.Wasm \
    -c Release \
    -o ../desktop/src-tauri/target/wwwroot
```

**This step**:
- Optimizes Blazor for production
- Minifies JavaScript
- Enables AOT compilation
- Bundles static assets

**Output**: ~2MB of optimized Blazor files in `target/wwwroot/`

### 5. Build Tauri (Release)

```bash
cd ../desktop
cargo tauri build
```

**This step** (takes ~5-10 minutes):
- Compiles Rust with optimizations (`--release`)
- Strips debug symbols
- Creates .deb package
- Creates .AppImage
- Creates .rpm package (if tools available)

**Expected output**:
```
    Compiling pm-core v0.1.0
    Compiling pm-db v0.1.0
    ...
    Finished release [optimized] target(s) in 7m 32s
    Building .deb package...
    Building AppImage...
    Building .rpm package...
    Finished 3 bundles at:
        /path/to/desktop/src-tauri/target/release/bundle/deb/blazor-agile-board_0.2.0_amd64.deb
        /path/to/desktop/src-tauri/target/release/bundle/appimage/blazor-agile-board_0.2.0_amd64.AppImage
        /path/to/desktop/src-tauri/target/release/bundle/rpm/blazor-agile-board-0.2.0-1.x86_64.rpm
```

---

## Build Artifacts

### AppImage

**Location**:
```
desktop/src-tauri/target/release/bundle/appimage/
└── blazor-agile-board_0.2.0_amd64.AppImage
```

**Size**: ~25-40 MB (includes all dependencies)

**Make executable**:
```bash
chmod +x blazor-agile-board_0.2.0_amd64.AppImage
```

**Run**:
```bash
./blazor-agile-board_0.2.0_amd64.AppImage
```

**Desktop Integration** (creates .desktop file):
```bash
# First run prompts to integrate
./blazor-agile-board_0.2.0_amd64.AppImage

# Or manually integrate
./blazor-agile-board_0.2.0_amd64.AppImage --appimage-portable-home
```

**Uninstall**:
```bash
# Just delete the file
rm blazor-agile-board_0.2.0_amd64.AppImage
rm -rf ~/.config/blazor-agile-board  # User data
```

---

### .deb Package

**Location**:
```
desktop/src-tauri/target/release/bundle/deb/
└── blazor-agile-board_0.2.0_amd64.deb
```

**Size**: ~15-25 MB

**Install**:
```bash
# Ubuntu/Debian
sudo dpkg -i blazor-agile-board_0.2.0_amd64.deb
sudo apt-get install -f  # Fix dependencies if needed

# Or use apt directly
sudo apt install ./blazor-agile-board_0.2.0_amd64.deb
```

**Install location**: `/usr/bin/blazor-agile-board`

**Data location**: `~/.config/com.blazor-agile-board/.pm/`

**Launch**:
```bash
blazor-agile-board
# Or find in application menu
```

**Uninstall**:
```bash
sudo apt remove blazor-agile-board
```

**Inspect package**:
```bash
dpkg -c blazor-agile-board_0.2.0_amd64.deb  # List contents
dpkg -I blazor-agile-board_0.2.0_amd64.deb  # Package info
```

---

### .rpm Package

**Location**:
```
desktop/src-tauri/target/release/bundle/rpm/
└── blazor-agile-board-0.2.0-1.x86_64.rpm
```

**Size**: ~15-25 MB

**Install**:
```bash
# Fedora/RHEL/CentOS
sudo dnf install ./blazor-agile-board-0.2.0-1.x86_64.rpm

# Or with rpm directly
sudo rpm -i blazor-agile-board-0.2.0-1.x86_64.rpm
```

**Install location**: `/usr/bin/blazor-agile-board`

**Data location**: `~/.config/com.blazor-agile-board/.pm/`

**Launch**:
```bash
blazor-agile-board
# Or find in application menu
```

**Uninstall**:
```bash
sudo dnf remove blazor-agile-board
```

**Inspect package**:
```bash
rpm -qlp blazor-agile-board-0.2.0-1.x86_64.rpm  # List contents
rpm -qip blazor-agile-board-0.2.0-1.x86_64.rpm  # Package info
```

---

## Build Specific Package Types

### Build Only AppImage

```bash
cargo tauri build --bundles appimage
```

### Build Only .deb

```bash
cargo tauri build --bundles deb
```

### Build Only .rpm

**Prerequisites**: Install `rpmbuild` first

```bash
# Debian/Ubuntu
sudo apt install rpm

# Fedora (usually pre-installed)
sudo dnf install rpm-build

# Then build
cargo tauri build --bundles rpm
```

---

## Optimization Tips

### Reduce Binary Size

**Edit** `desktop/src-tauri/Cargo.toml`:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization, slower build
strip = true        # Strip debug symbols
panic = "abort"     # Smaller panic handling
```

**Trade-offs**:
- ✅ ~30-40% smaller binary
- ⚠️ ~2x longer build time
- ⚠️ Slightly slower runtime (negligible for most apps)

### Faster Release Builds

**Edit** `desktop/src-tauri/Cargo.toml`:

```toml
[profile.release]
opt-level = 3       # Maximum performance
lto = "thin"        # Faster than "fat" LTO
codegen-units = 16  # Parallel codegen (faster build)

# Use mold linker (fastest)
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

**Install mold**:
```bash
sudo apt install mold  # Ubuntu/Debian
sudo dnf install mold  # Fedora
sudo pacman -S mold    # Arch
```

### Benchmark Binary Size

```bash
# Before optimization
du -sh desktop/src-tauri/target/release/blazor-agile-board

# After optimization
cargo clean
# Apply size optimizations in Cargo.toml
cargo tauri build
du -sh desktop/src-tauri/target/release/blazor-agile-board
```

---

## Distribution

### GitHub Releases

```bash
# Create release on GitHub
gh release create v0.2.0 \
    --title "v0.2.0" \
    --notes "Release notes here" \
    "desktop/src-tauri/target/release/bundle/appimage/blazor-agile-board_0.2.0_amd64.AppImage" \
    "desktop/src-tauri/target/release/bundle/deb/blazor-agile-board_0.2.0_amd64.deb" \
    "desktop/src-tauri/target/release/bundle/rpm/blazor-agile-board-0.2.0-1.x86_64.rpm"
```

### PPA (Ubuntu Personal Package Archive)

**Prerequisites**: Launchpad account

**Steps**:
1. Sign .deb with GPG key
2. Upload to PPA
3. Users install via:
   ```bash
   sudo add-apt-repository ppa:your-username/blazor-agile-board
   sudo apt update
   sudo apt install blazor-agile-board
   ```

**See**: [Launchpad PPA Guide](https://help.launchpad.net/Packaging/PPA)

### Copr (Fedora Community Projects)

**Prerequisites**: Fedora account

**Steps**:
1. Create Copr project
2. Upload .src.rpm
3. Users install via:
   ```bash
   sudo dnf copr enable your-username/blazor-agile-board
   sudo dnf install blazor-agile-board
   ```

**See**: [Copr User Documentation](https://docs.pagure.org/copr.copr/)

### Flathub (Flatpak)

**Format**: Sandboxed application

**Prerequisites**: Flatpak manifest and build

**See**: [Tauri Flatpak Guide](https://tauri.app/v1/guides/distribution/flatpak)

### Snap Store

**Format**: Snapd package

**Prerequisites**: Snapcraft account

**See**: [Snapcraft Documentation](https://snapcraft.io/docs)

---

## Verification

### Test AppImage

```bash
# Make executable
chmod +x blazor-agile-board_0.2.0_amd64.AppImage

# Run
./blazor-agile-board_0.2.0_amd64.AppImage

# Test on different distribution (in VM or container)
docker run -it --rm \
    -e DISPLAY=$DISPLAY \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    -v $(pwd):/app \
    ubuntu:22.04 \
    /app/blazor-agile-board_0.2.0_amd64.AppImage
```

### Test .deb Installation

```bash
# In Docker container (Ubuntu)
docker run -it --rm ubuntu:22.04 bash
# Inside container:
apt update
apt install -y ./blazor-agile-board_0.2.0_amd64.deb
blazor-agile-board --version
```

### Test .rpm Installation

```bash
# In Docker container (Fedora)
docker run -it --rm fedora:39 bash
# Inside container:
dnf install -y ./blazor-agile-board-0.2.0-1.x86_64.rpm
blazor-agile-board --version
```

### Verify Dependencies

```bash
# Check shared library dependencies
ldd desktop/src-tauri/target/release/blazor-agile-board

# Ensure all dependencies are available or bundled
```

### Verify Desktop Integration

```bash
# Check .desktop file
cat /usr/share/applications/blazor-agile-board.desktop

# Check icons
ls /usr/share/icons/hicolor/*/apps/blazor-agile-board.*

# Validate .desktop file
desktop-file-validate /usr/share/applications/blazor-agile-board.desktop
```

---

## Troubleshooting

### AppImage won't run: "cannot execute binary file"

**Symptom**: Permission denied or binary format error

**Solution**:
```bash
# Make executable
chmod +x blazor-agile-board_0.2.0_amd64.AppImage

# Check architecture
file blazor-agile-board_0.2.0_amd64.AppImage
# Should show: ELF 64-bit LSB executable, x86-64
```

### AppImage won't run: "FUSE error"

**Symptom**: AppImage requires FUSE to mount

**Solution**:
```bash
# Install FUSE
sudo apt install fuse libfuse2  # Ubuntu/Debian
sudo dnf install fuse fuse-libs  # Fedora

# Or extract and run
./blazor-agile-board_0.2.0_amd64.AppImage --appimage-extract
./squashfs-root/AppRun
```

### .deb build fails: "dpkg-deb not found"

**Symptom**: Missing dpkg tools

**Solution**:
```bash
sudo apt install dpkg-dev
```

### .rpm build fails: "rpmbuild not found"

**Symptom**: Missing RPM build tools

**Solution**:
```bash
# Ubuntu/Debian
sudo apt install rpm

# Fedora
sudo dnf install rpm-build rpmdevtools
```

### "error while loading shared libraries"

**Symptom**: Missing runtime dependencies

**Solution**:
```bash
# Check missing libraries
ldd desktop/src-tauri/target/release/blazor-agile-board | grep "not found"

# Install missing libraries (example)
sudo apt install libgtk-3-0 libwebkit2gtk-4.1-0
```

### .deb package conflicts with existing installation

**Symptom**: dpkg reports conflicts

**Solution**:
```bash
# Remove old package first
sudo apt remove blazor-agile-board

# Or force reinstall
sudo dpkg -i --force-overwrite blazor-agile-board_0.2.0_amd64.deb
```

### Large package size (>100 MB)

**Symptom**: Binary not optimized

**Solution**:
- Check `profile.release` optimizations in Cargo.toml
- Ensure `dotnet publish -c Release` (not Debug)
- Strip symbols: `strip = true` in Cargo.toml
- Check dependencies: `ldd` output shouldn't show unexpected libs

---

## CI/CD Integration

### GitHub Actions (Example)

```yaml
name: Build Linux Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Setup .NET
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '10.0.x'

      - name: Install system dependencies
        run: |
          sudo apt update
          sudo apt install -y \
            libgtk-3-dev \
            libwebkit2gtk-4.1-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev \
            patchelf

      - name: Build frontend
        run: |
          cd frontend
          dotnet publish ProjectManagement.Wasm -c Release -o ../desktop/src-tauri/target/wwwroot

      - name: Build Tauri
        run: |
          cd desktop
          cargo tauri build

      - name: Upload .deb
        uses: actions/upload-artifact@v4
        with:
          name: linux-deb
          path: desktop/src-tauri/target/release/bundle/deb/*.deb

      - name: Upload AppImage
        uses: actions/upload-artifact@v4
        with:
          name: linux-appimage
          path: desktop/src-tauri/target/release/bundle/appimage/*.AppImage

      - name: Upload .rpm
        uses: actions/upload-artifact@v4
        with:
          name: linux-rpm
          path: desktop/src-tauri/target/release/bundle/rpm/*.rpm
```

---

## Platform-Specific Notes

### Wayland vs X11

**AppImage and packages work on both**, but some users may need to force:

```bash
# Force X11
GDK_BACKEND=x11 blazor-agile-board

# Force Wayland
GDK_BACKEND=wayland blazor-agile-board
```

### High DPI Displays

**Should work automatically**, but can be forced:

```bash
# Force scaling
GDK_SCALE=2 blazor-agile-board

# Or via environment
export GDK_SCALE=2
```

### System Tray Support

**Requires AppIndicator** on most desktop environments:

```bash
# Ubuntu/Debian
sudo apt install libayatana-appindicator3-1

# Fedora
sudo dnf install libappindicator-gtk3

# GNOME requires extension
sudo apt install gnome-shell-extension-appindicator
```

---

## Next Steps

**Development workflow?** See [LINUX_DEV_BUILD_README.md](LINUX_DEV_BUILD_README.md)

**Manual testing?** See [../TESTING.md](../../desktop/TESTING.md)

**macOS/Windows builds?** See platform-specific production build guides

---

**Last Updated**: 2026-01-23
