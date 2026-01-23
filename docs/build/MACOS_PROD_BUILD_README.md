# macOS Production Build

**Purpose**: Create optimized, distributable .app bundle and .dmg installer for macOS.

**Time**: ~5-10 minutes for full release build

**Prerequisites**: [MACOS_DEPENDENCIES_README.md](MACOS_DEPENDENCIES_README.md) completed

---

## Quick Start

```bash
cd desktop
cargo tauri build --target universal-apple-darwin
```

**Output**: Universal binary (Intel + Apple Silicon) in:
```
desktop/src-tauri/target/universal-apple-darwin/release/bundle/
├── dmg/
│   └── Blazor Agile Board_0.1.0_universal.dmg  # Installer
└── macos/
    └── Blazor Agile Board.app                   # App bundle
```

---

## Build Types

### Universal Binary (Recommended)

**Target**: Both Intel (x86_64) and Apple Silicon (ARM64) in one .app

```bash
cargo tauri build --target universal-apple-darwin
```

**Pros**:
- ✅ Single download for all macOS users
- ✅ Simpler distribution
- ✅ Apple's recommended approach
- ✅ Required for Mac App Store

**Cons**:
- ⚠️ Larger file size (~2x)
- ⚠️ Longer build time (~2x)

**When to use**: Production releases, Mac App Store submission

---

### Intel Only (x86_64)

**Target**: Intel Macs only

```bash
cargo tauri build --target x86_64-apple-darwin
```

**Output**:
```
desktop/src-tauri/target/x86_64-apple-darwin/release/bundle/
```

**When to use**: Internal testing, debugging Intel-specific issues

---

### Apple Silicon Only (ARM64)

**Target**: M1/M2/M3 Macs only

```bash
cargo tauri build --target aarch64-apple-darwin
```

**Output**:
```
desktop/src-tauri/target/aarch64-apple-darwin/release/bundle/
```

**When to use**: Internal testing, debugging ARM-specific issues

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
cargo tauri build --target universal-apple-darwin
```

**This step** (takes ~5-10 minutes):
- Compiles Rust with optimizations (`--release`)
- Strips debug symbols
- Creates .app bundle
- Creates .dmg installer
- Code signs (if configured)

**Expected output**:
```
    Compiling pm-core v0.1.0
    Compiling pm-db v0.1.0
    ...
    Finished release [optimized] target(s) in 8m 12s
    Building application bundle...
    Creating DMG installer...
    Finished 2 bundles at:
        /path/to/desktop/src-tauri/target/universal-apple-darwin/release/bundle/dmg/Blazor Agile Board_0.2.0_universal.dmg
        /path/to/desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/Blazor Agile Board.app
```

---

## Build Artifacts

### .app Bundle

**Location**:
```
desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/
└── Blazor Agile Board.app/
    ├── Contents/
    │   ├── Info.plist         # App metadata
    │   ├── MacOS/
    │   │   └── blazor-agile-board  # Binary (universal)
    │   └── Resources/
    │       ├── icons/
    │       └── ...
```

**Size**: ~15-25 MB (universal binary)

**Usage**: Direct installation (drag to /Applications)

**Test locally**:
```bash
open "desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/Blazor Agile Board.app"
```

---

### .dmg Installer

**Location**:
```
desktop/src-tauri/target/universal-apple-darwin/release/bundle/dmg/
└── Blazor Agile Board_0.2.0_universal.dmg
```

**Size**: ~18-30 MB (compressed)

**Contents**: .app bundle + drag-to-Applications background

**Usage**: Distribution to end users

**Test locally**:
```bash
open "desktop/src-tauri/target/universal-apple-darwin/release/bundle/dmg/Blazor Agile Board_0.2.0_universal.dmg"
```

**Mount and inspect**:
```bash
hdiutil attach "Blazor Agile Board_0.2.0_universal.dmg"
ls /Volumes/Blazor\ Agile\ Board/
hdiutil detach /Volumes/Blazor\ Agile\ Board
```

---

## Code Signing & Notarization

### Prerequisites

1. **Apple Developer Account** ($99/year)
2. **Developer ID Application Certificate** (in Keychain)
3. **App-specific password** for notarization

### Configure Code Signing

**Edit** `desktop/src-tauri/tauri.conf.json`:

```json
{
  "tauri": {
    "bundle": {
      "macOS": {
        "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)",
        "entitlements": "entitlements.plist",
        "providerShortName": "TEAM_ID"
      }
    }
  }
}
```

**Create** `desktop/src-tauri/entitlements.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
</dict>
</plist>
```

### Sign and Notarize

```bash
# Build with code signing
cargo tauri build --target universal-apple-darwin

# Notarize (requires Apple credentials)
xcrun notarytool submit \
    "desktop/src-tauri/target/universal-apple-darwin/release/bundle/dmg/Blazor Agile Board_0.2.0_universal.dmg" \
    --apple-id "your@email.com" \
    --team-id "TEAM_ID" \
    --password "app-specific-password" \
    --wait

# Staple notarization ticket to DMG
xcrun stapler staple \
    "desktop/src-tauri/target/universal-apple-darwin/release/bundle/dmg/Blazor Agile Board_0.2.0_universal.dmg"
```

**Check notarization status**:
```bash
spctl -a -vv -t install "Blazor Agile Board_0.2.0_universal.dmg"
```

**Expected output**: `accepted` with notarization info

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
```

### Benchmark Binary Size

```bash
# Before optimization
du -sh "desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/Blazor Agile Board.app"

# After optimization
cargo clean
# Apply size optimizations in Cargo.toml
cargo tauri build --target universal-apple-darwin
du -sh "desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/Blazor Agile Board.app"
```

---

## Distribution

### GitHub Releases

```bash
# Create release on GitHub
gh release create v0.2.0 \
    --title "v0.2.0" \
    --notes "Release notes here" \
    "desktop/src-tauri/target/universal-apple-darwin/release/bundle/dmg/Blazor Agile Board_0.2.0_universal.dmg"
```

### Direct Download

1. Upload .dmg to CDN or file host
2. Provide download link: `https://your-site.com/downloads/Blazor-Agile-Board-0.2.0-universal.dmg`

### Mac App Store

**Additional requirements**:
- App Store provisioning profile
- App Store entitlements
- Sandbox restrictions
- Review process

**See**: [Tauri Mac App Store Guide](https://tauri.app/v1/guides/distribution/macos-app-store)

---

## Verification

### Test Installation

1. **Mount DMG**:
   ```bash
   open "Blazor Agile Board_0.2.0_universal.dmg"
   ```

2. **Drag app to Applications**

3. **Launch from Applications**:
   ```bash
   open /Applications/Blazor\ Agile\ Board.app
   ```

4. **Verify on both architectures** (if possible):
   - Intel Mac: App runs natively
   - Apple Silicon Mac: App runs natively (not via Rosetta)

### Check Architecture

```bash
# Check which architectures are included
lipo -info "/Applications/Blazor Agile Board.app/Contents/MacOS/blazor-agile-board"
```

**Expected output** (universal binary):
```
Architectures in the fat file: ... are: x86_64 arm64
```

### Verify Code Signing

```bash
codesign -dv --verbose=4 "/Applications/Blazor Agile Board.app"
```

**Check for**:
- Authority: "Developer ID Application: Your Name"
- Signature valid
- No errors

### Test Gatekeeper

```bash
# Simulate first launch (Gatekeeper check)
spctl --assess --type execute -vv "/Applications/Blazor Agile Board.app"
```

**Expected**: `accepted` (if notarized)

---

## Troubleshooting

### "Developer cannot be verified"

**Symptom**: macOS blocks unsigned app

**Solution**:
- Code sign and notarize (see Code Signing section)
- **Workaround** (development only): Right-click app → Open → Open anyway

### Universal binary build fails

**Symptom**: Cross-compilation errors

**Solution**:
```bash
# Install target architectures
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Retry build
cargo tauri build --target universal-apple-darwin
```

### DMG not created

**Symptom**: Only .app bundle exists, no .dmg

**Solution**:
- Check `tauri.conf.json` → `bundle.targets` includes `"dmg"`
- Ensure `hdiutil` is available (should be on all macOS)

### "App is damaged and can't be opened"

**Symptom**: Gatekeeper rejects the app

**Causes**:
1. Downloaded from internet (quarantine flag set)
2. Not notarized
3. Corrupt download

**Solution**:
```bash
# Remove quarantine flag (local testing only)
xattr -cr "/Applications/Blazor Agile Board.app"

# For distribution: Notarize the app
```

### Build fails with "disk full"

**Symptom**: Compilation fails with space errors

**Solution**:
```bash
# Clean old build artifacts
cargo clean
cd ../../frontend && dotnet clean

# Check space
df -h

# Remove old target directories if needed
rm -rf desktop/src-tauri/target
```

---

## CI/CD Integration

### GitHub Actions (Example)

```yaml
name: Build macOS Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-apple-darwin,aarch64-apple-darwin

      - name: Setup .NET
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '10.0.x'

      - name: Build frontend
        run: |
          cd frontend
          dotnet publish ProjectManagement.Wasm -c Release -o ../desktop/src-tauri/target/wwwroot

      - name: Build Tauri
        run: |
          cd desktop
          cargo tauri build --target universal-apple-darwin

      - name: Upload DMG
        uses: actions/upload-artifact@v4
        with:
          name: macos-dmg
          path: desktop/src-tauri/target/universal-apple-darwin/release/bundle/dmg/*.dmg
```

---

## Next Steps

**Development workflow?** See [MACOS_DEV_BUILD_README.md](MACOS_DEV_BUILD_README.md)

**Manual testing?** See [../TESTING.md](../../desktop/TESTING.md)

**Windows/Linux builds?** See platform-specific production build guides

---

**Last Updated**: 2026-01-23
