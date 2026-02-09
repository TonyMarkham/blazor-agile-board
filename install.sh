#!/usr/bin/env bash
# install.sh - Install Blazor Agile Board tools into <repo>/.pm/bin/
#
# Usage:
#   # From within a git repo:
#   curl -fsSL https://raw.githubusercontent.com/TonyMarkham/blazor-agile-board/main/install.sh | bash
#
#   # Or run directly:
#   bash install.sh
#
# Environment variables:
#   PM_VERSION     - Specific version to install (default: latest)
#   PM_REPO        - GitHub repo (default: TonyMarkham/blazor-agile-board)

set -euo pipefail

# =========================================================================
# Configuration
# =========================================================================

REPO="${PM_REPO:-TonyMarkham/blazor-agile-board}"

# =========================================================================
# Output Helpers
# =========================================================================

# Only use colors if stdout is a terminal
if [ -t 1 ]; then
    BLUE='\033[0;34m' GREEN='\033[0;32m' RED='\033[0;31m' NC='\033[0m'
else
    BLUE='' GREEN='' RED='' NC=''
fi

info()  { echo -e "${BLUE}[info]${NC}  $*"; }
ok()    { echo -e "${GREEN}[ok]${NC}    $*"; }
error() { echo -e "${RED}[error]${NC} $*" >&2; exit 1; }

# =========================================================================
# Repository Root Detection
# =========================================================================

find_repo_root() {
    if ! command -v git &>/dev/null; then
        error "git is required but not found"
    fi

    local root
    root=$(git rev-parse --show-toplevel 2>/dev/null) \
        || error "Not inside a git repository. Run this from within a repo."
    echo "$root"
}

# =========================================================================
# Platform Detection
# =========================================================================

detect_target() {
    local arch os

    # Detect CPU architecture
    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac

    # Detect operating system
    case "$(uname -s)" in
        Darwin) os="apple-darwin" ;;
        Linux)  os="unknown-linux-gnu" ;;
        MINGW*|MSYS*|CYGWIN*)
            error "Use install.ps1 for Windows: powershell -c 'irm .../install.ps1 | iex'"
            ;;
        *) error "Unsupported OS: $(uname -s)" ;;
    esac

    echo "${arch}-${os}"
}

# =========================================================================
# Version Detection
# =========================================================================

get_latest_version() {
    # Prefer gh CLI (authenticated, faster, no rate limit)
    if command -v gh &>/dev/null; then
        gh release view --repo "$REPO" --json tagName -q .tagName 2>/dev/null && return
    fi

    # Fall back to GitHub API via curl
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
        | grep '"tag_name"' \
        | head -1 \
        | sed 's/.*"tag_name": *"//;s/".*//'
}

# =========================================================================
# Main
# =========================================================================

main() {
    info "Blazor Agile Board Installer"
    echo ""

    # Find repo root via git
    local repo_root
    repo_root=$(find_repo_root)
    info "Repository: ${repo_root}"

    local install_dir="${repo_root}/.pm/bin"

    # Detect platform
    local target
    target=$(detect_target)
    info "Platform: ${target}"

    # Determine version
    local version="${PM_VERSION:-}"
    if [ -z "$version" ]; then
        info "Detecting latest version..."
        version=$(get_latest_version)
        [ -z "$version" ] && error "Could not determine latest version. Set PM_VERSION manually."
    fi
    info "Version: ${version}"

    # Build download URL
    local ver_num="${version#v}"   # Strip leading 'v' if present
    local archive_name="pm-${ver_num}-${target}"
    local archive_file="${archive_name}.tar.gz"
    local url="https://github.com/${REPO}/releases/download/${version}/${archive_file}"

    # Create install directory
    mkdir -p "$install_dir"

    # Download to temp directory (cleaned up on exit via trap)
    info "Downloading ${archive_file}..."
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    if command -v gh &>/dev/null; then
        gh release download "$version" --repo "$REPO" \
            --pattern "$archive_file" --dir "$tmp_dir" 2>/dev/null \
            || curl -fSL "$url" -o "$tmp_dir/$archive_file"
    else
        curl -fSL "$url" -o "$tmp_dir/$archive_file"
    fi

    # Extract archive
    info "Extracting archive..."
    tar xzf "$tmp_dir/$archive_file" -C "$tmp_dir"

    # Verify archive structure
    [ -d "$tmp_dir/$archive_name/bin" ] || error "Archive missing expected bin/ directory"

    # Install CLI binaries to .pm/bin/
    info "Installing CLI binaries to ${install_dir}/"
    cp "$tmp_dir/$archive_name/bin/pm" "$install_dir/" 2>/dev/null || true
    cp "$tmp_dir/$archive_name/bin/pm-server" "$install_dir/" 2>/dev/null || true
    chmod +x "$install_dir/pm" "$install_dir/pm-server" 2>/dev/null || true

    # Install Tauri desktop app if present
    local app_installed=false
    if [ -d "$tmp_dir/$archive_name/bin/Project Manager.app" ]; then
        info "Installing Tauri desktop app..."
        local app_dir="${repo_root}/.pm"
        cp -r "$tmp_dir/$archive_name/bin/Project Manager.app" "$app_dir/" 2>/dev/null || true
        if [ -d "$app_dir/Project Manager.app" ]; then
            ok "Installed desktop app to ${app_dir}/Project Manager.app"
            app_installed=true
        fi
    elif [ -f "$tmp_dir/$archive_name/bin/project-manager" ]; then
        # Linux AppImage
        info "Installing desktop app (AppImage)..."
        cp "$tmp_dir/$archive_name/bin/project-manager" "$install_dir/" 2>/dev/null || true
        chmod +x "$install_dir/project-manager" 2>/dev/null || true
        if [ -f "$install_dir/project-manager" ]; then
            ok "Installed desktop app to ${install_dir}/project-manager"
            app_installed=true
        fi
    fi

    # Write config.json for Tauri double-click support.
    # When Tauri is launched outside a terminal, git rev-parse fails.
    # This file tells Tauri where the repo root is.
    info "Writing config.json (repo_root for Tauri)..."
    cat > "$install_dir/config.json" << CONFIGJSON
{"repo_root": "${repo_root}"}
CONFIGJSON
    ok "Created ${install_dir}/config.json"

    # ↑ config.json is consumed by two functions from Session 121.2:
    #   - Config::config_dir_from_binary_config() (Step 1, fallback #2)
    #   - find_server_dir_from_binary() (Step 5)
    # Without this file, double-click Tauri falls through to ~/.pm/ global.

    # Create .pm/.gitignore if it doesn't exist (idempotent)
    local pm_dir="${repo_root}/.pm"
    if [ ! -f "${pm_dir}/.gitignore" ]; then
        info "Creating .pm/.gitignore..."
        cat > "${pm_dir}/.gitignore" << 'GITIGNORE'
# Runtime files - not tracked
# data.json and config.toml ARE tracked (not listed here = tracked).

# SQLite database (local performance — use data.json for git sync)
data.db

# Negate root .gitignore's *.json pattern if one exists
!data.json

bin/
*.db-wal
*.db-shm
server.json
server.lock
logs/
log/
tauri/

# Desktop app (build artifact)
*.app/
project-manager
GITIGNORE
        ok "Created .pm/.gitignore"
    fi

    # Print success message
    echo ""
    ok "Installation complete!"
    echo ""
    echo "  📦 Installed Components:"
    echo ""
    echo "    CLI Tools (${install_dir}):"
    echo "      • pm          - Project management CLI"
    echo "      • pm-server   - Backend server"
    echo ""

    if [ "$app_installed" = true ]; then
        if [ -d "${repo_root}/.pm/Project Manager.app" ]; then
            echo "    Desktop App (macOS):"
            echo "      • ${repo_root}/.pm/Project Manager.app"
            echo ""
            echo "  🚀 Launch Desktop App:"
            echo "    open \"${repo_root}/.pm/Project Manager.app\""
            echo ""
            echo "    Or move to Applications folder:"
            echo "    mv \"${repo_root}/.pm/Project Manager.app\" /Applications/"
        elif [ -f "$install_dir/project-manager" ]; then
            echo "    Desktop App (Linux AppImage):"
            echo "      • ${install_dir}/project-manager"
            echo ""
            echo "  🚀 Launch Desktop App:"
            echo "    ${install_dir}/project-manager"
        fi
        echo ""
    fi

    echo "  💻 CLI Usage:"
    echo "    .pm/bin/pm project list --pretty"
    echo "    .pm/bin/pm desktop"
    echo ""
    echo "  📝 Add to PATH (optional):"
    echo "    export PATH=\"${repo_root}/.pm/bin:\$PATH\""
    echo ""
}

main "$@"
