# Session 110: Portable Distribution via GitHub Releases

## Summary

Set up portable distribution for all three artifacts (`pm` CLI, `pm-server`, Tauri desktop app). Users download a single archive per platform, extract anywhere, and run locally. No installers, no DMG, no /Applications, no .deb. Data lives in `.pm/` relative to the working directory.

Releases are built manually on 4 machines and uploaded to GitHub Releases. Install scripts allow installation without cloning the repo.

## Architecture

```
Build Machine (any platform)                  GitHub Releases
┌──────────────────────────┐                 ┌─────────────────────────────────┐
│  just release-build      │                 │  v0.1.0                         │
│  ┌────────────────────┐  │   just release  │  ├─ pm-0.1.0-aarch64-apple-     │
│  │ cargo build (cli)  │  │  ────────────>  │  │  darwin.tar.gz               │
│  │ cargo build (srv)  │  │                 │  ├─ pm-0.1.0-x86_64-apple-      │
│  │ cargo tauri build  │  │                 │  │  darwin.tar.gz               │
│  │ tar/zip → dist/    │  │                 │  ├─ pm-0.1.0-x86_64-unknown-    │
│  └────────────────────┘  │                 │  │  linux-gnu.tar.gz            │
└──────────────────────────┘                 │  └─ pm-0.1.0-x86_64-pc-         │
                                             │     windows-msvc.zip            │
End User                                     └─────────────────────────────────┘
┌──────────────────────────┐                           │
│  curl ... | bash         │  <─────────────────────────
│  ┌────────────────────┐  │
│  │ pm                 │  │  CLI tool
│  │ pm-server          │  │  Backend server
│  │ Project Manager.app│  │  Tauri desktop GUI
│  │ .pm/               │  │  Local data directory
│  └────────────────────┘  │
└──────────────────────────┘
```

## Platform Archive Contents

| Platform | Archive | Contents |
|----------|---------|----------|
| macOS Apple Silicon | `pm-0.1.0-aarch64-apple-darwin.tar.gz` | `pm`, `pm-server`, `Project Manager.app/` |
| macOS Intel | `pm-0.1.0-x86_64-apple-darwin.tar.gz` | `pm`, `pm-server`, `Project Manager.app/` |
| Linux x64 | `pm-0.1.0-x86_64-unknown-linux-gnu.tar.gz` | `pm`, `pm-server`, `project-manager` (AppImage) |
| Windows x64 | `pm-0.1.0-x86_64-pc-windows-msvc.zip` | `pm.exe`, `pm-server.exe`, `project-manager.exe` |

---

## Step 1: Add `[workspace.package]` to Root `Cargo.toml`

**File:** `Cargo.toml` (MODIFY)

Insert between `[workspace]` and `[workspace.dependencies]`:

```toml
[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/OWNER/blazor-agile-board"
```

Repository URL placeholder — update once real URL is provided.

---

## Step 2: Update `pm-cli/Cargo.toml` — Inherit Workspace Metadata

**File:** `backend/crates/pm-cli/Cargo.toml` (MODIFY)

```toml
[package]
name = "pm-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI tool for Blazor Agile Board project management"

[[bin]]
name = "pm"
path = "src/main.rs"

[dependencies]
# ... unchanged ...
```

---

## Step 3: Update `pm-server/Cargo.toml` — Inherit Workspace Metadata

**File:** `backend/pm-server/Cargo.toml` (MODIFY)

```toml
[package]
name = "pm-server"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Backend server for Blazor Agile Board project management"

[[bin]]
name = "pm-server"
path = "src/main.rs"

[dependencies]
# ... unchanged ...
```

---

## Step 4: Update `.gitignore`

**File:** `.gitignore` (MODIFY)

- **Remove** line 3: `Cargo.lock` (binary projects should commit lockfile for reproducible builds)
- **Add** after the Rust section:

```gitignore
# Release archives
dist/
```

Then run:
```bash
cargo generate-lockfile
```

---

## Step 5: Add Release Distribution Commands to Justfile

**File:** `justfile` (MODIFY)

**New variables** (after existing cargo flags ~line 52):

```just
# === Release Distribution ===
dist_dir := "dist"
version := "0.1.0"
target_triple := `rustc -vV | grep host | cut -d' ' -f2`
archive_name := "pm-" + version + "-" + target_triple
```

**New section:**

```just
# ============================================================================
# Release Distribution Commands
# ============================================================================

# Build all portable artifacts for current platform
build-portable:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build -p pm-cli --release
    cargo build -p pm-server --release
    case "$(uname -s)" in
        Darwin)  cargo tauri build --bundles app ;;
        Linux)   cargo tauri build --bundles appimage ;;
        MINGW*|MSYS*|CYGWIN*) cargo tauri build --bundles none ;;
    esac

# Create platform archive with all artifacts
archive:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p {{dist_dir}}
    STAGING=$(mktemp -d)

    # CLI + server binaries
    cp target/release/pm${BINARY_EXT:-} "$STAGING/"
    cp target/release/pm-server${BINARY_EXT:-} "$STAGING/"

    # Tauri app (platform-specific)
    case "$(uname -s)" in
        Darwin)
            cp -r "target/release/bundle/macos/Project Manager.app" "$STAGING/"
            tar -czf "{{dist_dir}}/{{archive_name}}.tar.gz" -C "$STAGING" .
            ;;
        Linux)
            cp target/release/bundle/appimage/*.AppImage "$STAGING/project-manager"
            tar -czf "{{dist_dir}}/{{archive_name}}.tar.gz" -C "$STAGING" .
            ;;
        MINGW*|MSYS*|CYGWIN*)
            cp "target/release/project-manager.exe" "$STAGING/"
            cd "$STAGING" && zip -r "{{justfile_directory()}}/{{dist_dir}}/{{archive_name}}.zip" . && cd -
            ;;
    esac

    rm -rf "$STAGING"
    echo "Archive created: {{dist_dir}}/{{archive_name}}.*"
    ls -lh {{dist_dir}}/

# Build + archive in one step
release-build: build-portable archive

# Create GitHub release + upload (first platform)
# Usage: just release 0.1.0
release tag:
    gh release create "v{{tag}}" \
        --title "v{{tag}}" \
        --generate-notes \
        {{dist_dir}}/{{archive_name}}.*

# Upload to existing release (additional platforms)
# Usage: just release-upload 0.1.0
release-upload tag:
    gh release upload "v{{tag}}" {{dist_dir}}/{{archive_name}}.*

# Clean release archives
clean-dist:
    rm -rf {{dist_dir}}
```

**Tauri `--bundles` flag per platform:**
- macOS: `--bundles app` — produces portable `.app` only, NO `.dmg`
- Linux: `--bundles appimage` — portable single-file executable
- Windows: `--bundles none` — raw `.exe` (WebView2 pre-installed on Win10 21H2+ / Win11)

**Add to help recipe:**

```just
    echo "📦 Release Distribution:"
    echo "  just release-build             - Build + archive all artifacts"
    echo "  just release 0.1.0             - Create GitHub release (first platform)"
    echo "  just release-upload 0.1.0      - Upload to existing release (other platforms)"
    echo "  just clean-dist                - Remove release archives"
    echo ""
```

---

## Step 6: Create `install.sh` — macOS/Linux Install Script

**File:** `install.sh` (NEW)

Users run without cloning the repo:
```bash
curl -sSL https://raw.githubusercontent.com/OWNER/blazor-agile-board/main/install.sh | bash
```

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="OWNER/blazor-agile-board"

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Darwin) os_name="apple-darwin" ;;
    Linux)  os_name="unknown-linux-gnu" ;;
    *)
        echo "Error: Unsupported OS: $OS"
        echo "Windows users: use install.ps1 instead"
        exit 1
        ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)  arch_name="x86_64" ;;
    arm64|aarch64) arch_name="aarch64" ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

TARGET="${arch_name}-${os_name}"

# Get latest release tag
echo "Fetching latest release..."
LATEST=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4)

if [ -z "$LATEST" ]; then
    echo "Error: Could not determine latest release"
    exit 1
fi

VERSION="${LATEST#v}"
ARCHIVE="pm-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARCHIVE}"

echo "Downloading ${ARCHIVE}..."
curl -fSL "$URL" -o "$ARCHIVE"

echo "Extracting..."
tar -xzf "$ARCHIVE"
rm "$ARCHIVE"

chmod +x pm pm-server

# macOS: clear quarantine attribute so Gatekeeper doesn't block
if [ "$OS" = "Darwin" ] && [ -d "Project Manager.app" ]; then
    xattr -cr "Project Manager.app" 2>/dev/null || true
fi

echo ""
echo "Installed pm ${VERSION} for ${TARGET}:"
echo "  ./pm --help              CLI tool"
echo "  ./pm-server              Backend server"
if [ "$OS" = "Darwin" ]; then
    echo "  open \"Project Manager.app\"  Desktop app"
else
    echo "  ./project-manager        Desktop app"
fi
echo ""
echo "Data is stored in .pm/ relative to where you run these commands."
```

---

## Step 7: Create `install.ps1` — Windows Install Script

**File:** `install.ps1` (NEW)

Users run without cloning the repo:
```powershell
irm https://raw.githubusercontent.com/OWNER/blazor-agile-board/main/install.ps1 | iex
```

```powershell
$ErrorActionPreference = "Stop"

$repo = "OWNER/blazor-agile-board"
$target = "x86_64-pc-windows-msvc"

# Get latest release tag
Write-Host "Fetching latest release..."
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
$tag = $release.tag_name
$version = $tag.TrimStart("v")

$archive = "pm-$version-$target.zip"
$url = "https://github.com/$repo/releases/download/$tag/$archive"

Write-Host "Downloading $archive..."
Invoke-WebRequest -Uri $url -OutFile $archive

Write-Host "Extracting..."
Expand-Archive -Path $archive -DestinationPath . -Force
Remove-Item $archive

Write-Host ""
Write-Host "Installed pm $version for $target:"
Write-Host "  .\pm.exe --help           CLI tool"
Write-Host "  .\pm-server.exe           Backend server"
Write-Host "  .\project-manager.exe     Desktop app"
Write-Host ""
Write-Host "Data is stored in .pm\ relative to where you run these commands."
```

---

## Release Workflow (Manual, 4 Machines)

**Prerequisites on each machine:**
- Rust toolchain installed
- `gh` CLI installed and authenticated (`gh auth login`)
- Repo cloned and on the correct tag/commit

**On Machine 1 (whichever goes first):**
```bash
just release-build        # builds all artifacts + creates archive in dist/
just release 0.1.0        # creates GitHub Release + uploads this platform's archive
```

**On Machines 2, 3, 4:**
```bash
just release-build        # builds all artifacts + creates archive in dist/
just release-upload 0.1.0 # uploads archive to the existing release
```

Order doesn't matter. Each machine auto-detects its target triple via `rustc -vV`. The GitHub Release ends up with 4 archives (one per platform).

---

## End-User Install Workflow

```bash
# macOS/Linux — one command, no repo clone needed
curl -sSL https://raw.githubusercontent.com/OWNER/blazor-agile-board/main/install.sh | bash

# Windows PowerShell — one command, no repo clone needed
irm https://raw.githubusercontent.com/OWNER/blazor-agile-board/main/install.ps1 | iex

# Everything is now in current directory:
./pm --help
./pm-server
open "Project Manager.app"   # macOS
./project-manager             # Linux
.\project-manager.exe         # Windows
```

---

## Known Caveats

1. **macOS Gatekeeper:** Unsigned `.app` triggers Gatekeeper on first run. The install script runs `xattr -cr` automatically. If downloaded manually, user must `xattr -d com.apple.quarantine "Project Manager.app"` or right-click > Open. Apple Developer signing ($99/year) eliminates this entirely.
2. **Windows WebView2:** The raw Tauri `.exe` requires WebView2 runtime. Pre-installed on Windows 10 21H2+ and Windows 11. Older Windows versions need manual WebView2 install from Microsoft.
3. **Linux AppImage:** May need `chmod +x` (handled by install script) and FUSE installed on some distros (`sudo apt install libfuse2`).
4. **Cargo.lock:** Currently gitignored. This session commits it — standard practice for binary projects to ensure reproducible builds.

---

## Verification Steps

1. **Build check:**
   ```bash
   just check-backend
   ```

2. **Build + archive:**
   ```bash
   just release-build
   ```

3. **Inspect archive contents:**
   ```bash
   tar tzf dist/pm-*.tar.gz
   # Should show: pm, pm-server, Project Manager.app/ (macOS)
   ```

4. **Test install script:**
   ```bash
   mkdir /tmp/test-install && cd /tmp/test-install
   bash /path/to/install.sh
   ./pm --version
   ./pm-server --version
   ```

5. **Test portability:** Run all three artifacts — verify they find/create `.pm/` in the current directory.

---

## Files Summary

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` (root) | MODIFY | Add `[workspace.package]` with version, edition, license, repository |
| `backend/crates/pm-cli/Cargo.toml` | MODIFY | Inherit workspace metadata |
| `backend/pm-server/Cargo.toml` | MODIFY | Inherit workspace metadata |
| `.gitignore` | MODIFY | Remove `Cargo.lock`, add `dist/` |
| `justfile` | MODIFY | Add release distribution variables + 6 new commands |
| `install.sh` | NEW | macOS/Linux install script (curl one-liner) |
| `install.ps1` | NEW | Windows install script (PowerShell one-liner) |

**Total: 7 files (2 new, 5 modified)**
