# justfile - task runner for Blazor Agile Board

# === Shell Configuration ===
# Configure PowerShell for Windows backtick evaluation
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# === Directory Structure ===
frontend_dir := "frontend"
backend_dir := "backend"
desktop_dir := "desktop"
config_dir := ".pm"
coverage_dir := "./coverage"

# === Distribution ===
# NOTE: Version auto-extracted from workspace Cargo.toml
dist_dir := "dist"
version := if os_family() == "unix" {
`cargo pkgid -p pm-cli | cut -d'#' -f2`
} else {
`(cargo pkgid -p pm-cli).Split('#')[1]`
}
target_triple := `rustc --print host-tuple`
archive_name := "pm-" + version + "-" + target_triple
archive_ext := if os_family() == "unix" { "tar.gz" } else { "zip" }

# === Solution File ===
frontend_solution := frontend_dir + "/ProjectManagement.slnx"

# === C# Production Projects ===
core_project := frontend_dir + "/ProjectManagement.Core/ProjectManagement.Core.csproj"
services_project := frontend_dir + "/ProjectManagement.Services/ProjectManagement.Services.csproj"
components_project := frontend_dir + "/ProjectManagement.Components/ProjectManagement.Components.csproj"
wasm_project := frontend_dir + "/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj"

# === C# Test Projects ===
core_tests := frontend_dir + "/ProjectManagement.Core.Tests/ProjectManagement.Core.Tests.csproj"
services_tests := frontend_dir + "/ProjectManagement.Services.Tests/ProjectManagement.Services.Tests.csproj"
components_tests := frontend_dir + "/ProjectManagement.Components.Tests/ProjectManagement.Components.Tests.csproj"

# === Rust Backend Packages ===
rust_server := "pm-server"
rust_core := "pm-core"
rust_db := "pm-db"
rust_auth := "pm-auth"
rust_proto := "pm-proto"
rust_ws := "pm-ws"
rust_config := "pm-config"
rust_cli := "pm-cli"

# === Build Configurations ===
config_debug := "Debug"
config_release := "Release"

# === Configuration Files ===
config_example := backend_dir + "/config.example.toml"
config_file := config_dir + "/config.toml"

# === Dotnet Flags ===
dotnet_no_restore := "--no-restore"
dotnet_verbosity_normal := "--verbosity normal"
dotnet_coverage_collector := "XPlat Code Coverage"
dotnet_filter_prefix := "FullyQualifiedName~"

# === Cargo Flags ===
cargo_workspace := "--workspace"
cargo_all_targets := "--all-targets"
cargo_all_features := "--all-features"
cargo_release := "--release"

# === Build Artifact Patterns ===
frontend_bin_pattern := frontend_dir + "/*/bin"
frontend_obj_pattern := frontend_dir + "/*/obj"

# === Messages ===
msg_config_created := "Created " + config_file + " from example"
msg_config_exists := config_file + " already exists, skipping"

# ============================================================================
# C# Frontend Commands
# ============================================================================

# === C# Solution-Level Commands ===

# Restore all NuGet packages for frontend
restore-frontend:
    dotnet restore {{frontend_solution}}

# Build entire solution (Debug)
build-frontend:
    dotnet build {{frontend_solution}} -c {{config_debug}} {{dotnet_no_restore}}

# Build entire solution (Release)
build-frontend-release:
    dotnet build {{frontend_solution}} -c {{config_release}} {{dotnet_no_restore}}

# Run all frontend tests
test-frontend:
    dotnet test {{frontend_solution}} {{dotnet_no_restore}}

# Run all tests with detailed output
test-frontend-verbose:
    dotnet test {{frontend_solution}} {{dotnet_no_restore}} {{dotnet_verbosity_normal}}

# Run all tests with code coverage
test-frontend-coverage:
    dotnet test {{frontend_solution}} {{dotnet_no_restore}} --collect:"{{dotnet_coverage_collector}}" --results-directory {{coverage_dir}}

# Clean all frontend projects
clean-frontend:
    @just _clean-frontend-impl

[unix]
_clean-frontend-impl:
    dotnet clean {{frontend_solution}}
    rm -rf {{frontend_bin_pattern}} {{frontend_obj_pattern}}

[windows]
_clean-frontend-impl:
    dotnet clean {{frontend_solution}}
    @if (Test-Path "{{frontend_dir}}") { Get-ChildItem -Path "{{frontend_dir}}" -Include bin,obj -Recurse -Directory | Remove-Item -Recurse -Force }

# Full frontend workflow: restore -> build -> test
check-frontend:
    just restore-frontend
    just build-frontend
    just test-frontend

# === C# Test Commands ===

# Run specific test project
test-cs-core:
    dotnet test {{core_tests}} {{dotnet_no_restore}}

test-cs-services:
    dotnet test {{services_tests}} {{dotnet_no_restore}}

test-cs-components:
    dotnet test {{components_tests}} {{dotnet_no_restore}}

# Run tests matching a filter (namespace, class, or method name)
# Usage: just test-cs-filter "ProjectManagement.Core.Tests.Converters"
test-cs-filter filter:
    dotnet test {{frontend_solution}} {{dotnet_no_restore}} --filter "{{dotnet_filter_prefix}}{{filter}}"

# List all available tests without running them
list-tests-cs:
    dotnet test {{frontend_solution}} --list-tests {{dotnet_no_restore}}

# Watch mode - auto-run tests on file changes
watch-test-cs-core:
    dotnet watch --project {{core_tests}} test

watch-test-cs-services:
    dotnet watch --project {{services_tests}} test

watch-test-cs-components:
    dotnet watch --project {{components_tests}} test

# === C# Build Commands (Individual Projects) ===

# Build specific projects
build-cs-core config=config_debug:
    dotnet build {{core_project}} -c {{config}} {{dotnet_no_restore}}

build-cs-services config=config_debug:
    dotnet build {{services_project}} -c {{config}} {{dotnet_no_restore}}

build-cs-components config=config_debug:
    dotnet build {{components_project}} -c {{config}} {{dotnet_no_restore}}

build-cs-wasm config=config_debug:
    dotnet build {{wasm_project}} -c {{config}} {{dotnet_no_restore}}

# Publish WASM project
publish-wasm config=config_debug:
    dotnet publish {{wasm_project}} -c {{config}} {{dotnet_no_restore}}

# Watch mode - auto-rebuild on file changes
watch-cs-core:
    dotnet watch --project {{core_project}} build

watch-cs-services:
    dotnet watch --project {{services_project}} build

watch-cs-components:
    dotnet watch --project {{components_project}} build

watch-cs-wasm:
    dotnet watch --project {{wasm_project}} build

# ============================================================================
# Rust Backend Commands
# ============================================================================

# === Rust Workspace-Level Commands ===

# Restore backend dependencies
restore-backend:
    cargo fetch

# Check entire workspace (fast compile check without codegen)
check-backend:
    cargo check {{cargo_workspace}} {{cargo_all_targets}}

# Run clippy on entire workspace
clippy-backend:
    cargo clippy {{cargo_workspace}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

# Build entire workspace (debug)
build-backend:
    cargo build {{cargo_workspace}}

# Build entire workspace (release)
build-backend-release:
    cargo build {{cargo_workspace}} {{cargo_release}}

# Run all backend tests
test-backend:
    cargo test {{cargo_workspace}}

# Run all backend tests with output
test-backend-verbose:
    cargo test {{cargo_workspace}} -- --nocapture

# Clean backend build artifacts
clean-backend:
    cargo clean

# Full backend workflow: check -> clippy -> test
check-backend-full:
    just check-backend
    just clippy-backend
    just test-backend

# === Rust Individual Package Commands ===

# Check specific package
check-rs-server:
    cargo check -p {{rust_server}} {{cargo_all_targets}}

check-rs-core:
    cargo check -p {{rust_core}} {{cargo_all_targets}}

check-rs-db:
    cargo check -p {{rust_db}} {{cargo_all_targets}}

check-rs-auth:
    cargo check -p {{rust_auth}} {{cargo_all_targets}}

check-rs-proto:
    cargo check -p {{rust_proto}} {{cargo_all_targets}}

check-rs-ws:
    cargo check -p {{rust_ws}} {{cargo_all_targets}}

check-rs-config:
    cargo check -p {{rust_config}} {{cargo_all_targets}}

# Clippy specific package
clippy-rs-server:
    cargo clippy -p {{rust_server}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rs-core:
    cargo clippy -p {{rust_core}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rs-db:
    cargo clippy -p {{rust_db}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rs-auth:
    cargo clippy -p {{rust_auth}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rs-proto:
    cargo clippy -p {{rust_proto}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rs-ws:
    cargo clippy -p {{rust_ws}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rs-config:
    cargo clippy -p {{rust_config}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

# Build specific package (debug)
build-rs-server:
    cargo build -p {{rust_server}}

build-rs-core:
    cargo build -p {{rust_core}}

build-rs-db:
    cargo build -p {{rust_db}}

build-rs-auth:
    cargo build -p {{rust_auth}}

build-rs-proto:
    cargo build -p {{rust_proto}}

build-rs-ws:
    cargo build -p {{rust_ws}}

build-rs-config:
    cargo build -p {{rust_config}}

# Build specific package (release)
build-rs-server-release:
    cargo build -p {{rust_server}} {{cargo_release}}

build-rs-core-release:
    cargo build -p {{rust_core}} {{cargo_release}}

build-rs-db-release:
    cargo build -p {{rust_db}} {{cargo_release}}

build-rs-auth-release:
    cargo build -p {{rust_auth}} {{cargo_release}}

build-rs-proto-release:
    cargo build -p {{rust_proto}} {{cargo_release}}

build-rs-ws-release:
    cargo build -p {{rust_ws}} {{cargo_release}}

build-rs-config-release:
    cargo build -p {{rust_config}} {{cargo_release}}

# Test specific package
test-rs-server:
    cargo test -p {{rust_server}}

test-rs-core:
    cargo test -p {{rust_core}}

test-rs-db:
    cargo test -p {{rust_db}}

test-rs-auth:
    cargo test -p {{rust_auth}}

test-rs-proto:
    cargo test -p {{rust_proto}}

test-rs-ws:
    cargo test -p {{rust_ws}}

test-rs-config:
    cargo test -p {{rust_config}}

# Watch mode - auto-rebuild on file changes
watch-rs-server:
    cargo watch -x 'check -p {{rust_server}}'

watch-rs-core:
    cargo watch -x 'check -p {{rust_core}}'

watch-rs-db:
    cargo watch -x 'check -p {{rust_db}}'

watch-rs-auth:
    cargo watch -x 'check -p {{rust_auth}}'

watch-rs-proto:
    cargo watch -x 'check -p {{rust_proto}}'

watch-rs-ws:
    cargo watch -x 'check -p {{rust_ws}}'

watch-rs-config:
    cargo watch -x 'check -p {{rust_config}}'

# Watch mode - auto-test on file changes
watch-test-rs-server:
    cargo watch -x 'test -p {{rust_server}}'

watch-test-rs-core:
    cargo watch -x 'test -p {{rust_core}}'

watch-test-rs-db:
    cargo watch -x 'test -p {{rust_db}}'

watch-test-rs-auth:
    cargo watch -x 'test -p {{rust_auth}}'

watch-test-rs-proto:
    cargo watch -x 'test -p {{rust_proto}}'

watch-test-rs-ws:
    cargo watch -x 'test -p {{rust_ws}}'

watch-test-rs-config:
    cargo watch -x 'test -p {{rust_config}}'

# Check CLI
check-rs-cli:
    cargo check -p {{rust_cli}} {{cargo_all_targets}}

# Clippy CLI
clippy-rs-cli:
    cargo clippy -p {{rust_cli}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

# Build CLI (debug)
build-rs-cli:
    cargo build -p {{rust_cli}}

# Build CLI (release)
build-rs-cli-release:
    cargo build -p {{rust_cli}} {{cargo_release}}

# Test CLI
test-rs-cli:
    cargo test -p {{rust_cli}}

# Watch CLI - auto-rebuild on file changes
watch-rs-cli:
    cargo watch -x 'check -p {{rust_cli}}'

# Watch CLI - auto-test on file changes
watch-test-rs-cli:
    cargo watch -x 'test -p {{rust_cli}}'

# Run CLI with arguments (example: just run-cli project list --pretty)
run-cli *ARGS:
    cargo run -p {{rust_cli}} -- {{ARGS}}

# ============================================================================
# Combined Commands
# ============================================================================

# Restore both frontend and backend dependencies
restore:
    just restore-frontend
    just restore-backend

# Check everything (fast compile check)
check-all:
    just check-backend
    just build-frontend

# Lint everything
lint:
    just clippy-backend

# Build both backend and frontend (debug, parallel)
build-dev:
  just setup-config
  just restore
  just _build-dev-impl

[unix]
_build-dev-impl:
  cargo build -p {{rust_server}} & \
  dotnet publish {{wasm_project}} -c {{config_debug}} {{dotnet_no_restore}} & \
  wait

[windows]
_build-dev-impl:
  cargo build -p {{rust_server}}
  dotnet publish {{wasm_project}} -c {{config_debug}} {{dotnet_no_restore}}

# Build both backend and frontend (release, parallel)
build-release:
  just setup-config
  just restore
  just _build-release-impl

[unix]
_build-release-impl:
  cargo build -p {{rust_server}} {{cargo_release}} & \
  dotnet publish {{wasm_project}} -c {{config_release}} {{dotnet_no_restore}} & \
  wait

[windows]
_build-release-impl:
  cargo build -p {{rust_server}} {{cargo_release}}
  dotnet publish {{wasm_project}} -c {{config_release}} {{dotnet_no_restore}}

# Run all tests (backend + frontend)
test:
    just test-backend
    just test-frontend

# Clean all build artifacts
clean:
    just clean-backend
    just clean-frontend

# Full check: restore, check, clippy, build, test everything
check:
    just restore
    just check-all
    just lint
    just test

# ============================================================================
# Tauri Desktop Commands
# ============================================================================

# Run development build then start Tauri
dev:
    just build-dev
    cargo tauri dev

# Full production build with Tauri
build:
    just build-release
    cargo tauri build

# ============================================================================
# Distribution Commands
# ============================================================================

# Build all binaries in release mode for distribution
build-portable:
    @just _build-portable-impl

[unix]
_build-portable-impl:
    @echo "Building portable release ({{target_triple}})..."
    cargo build -p pm-cli {{cargo_release}}
    cargo build -p pm-server {{cargo_release}}
    cargo tauri build

[windows]
_build-portable-impl:
    @echo "Building portable release ({{target_triple}})..."
    cargo build -p pm-cli {{cargo_release}}
    cargo build -p pm-server {{cargo_release}}
    cargo tauri build

# Create distribution archive from release binaries
archive: build-portable
    @just _archive-impl

[unix]
_archive-impl:
    @echo "Creating archive: {{dist_dir}}/{{archive_name}}.tar.gz"
    @test -f target/release/pm || (echo "ERROR: pm binary not found in target/release/"; exit 1)
    @test -f target/release/pm-server || (echo "ERROR: pm-server binary not found in target/release/"; exit 1)
    mkdir -p {{dist_dir}}/{{archive_name}}/bin
    cp target/release/pm {{dist_dir}}/{{archive_name}}/bin/
    cp target/release/pm-server {{dist_dir}}/{{archive_name}}/bin/
    cp pm {{dist_dir}}/{{archive_name}}/
    mkdir -p {{dist_dir}}/{{archive_name}}/skills/pm
    cp .pm/skills/pm/SKILL.md {{dist_dir}}/{{archive_name}}/skills/pm/
    @if [ -d "target/release/bundle/macos/Project Manager.app" ]; then \
        cp -r "target/release/bundle/macos/Project Manager.app" {{dist_dir}}/{{archive_name}}/bin/; \
    elif [ -f target/release/project-manager ]; then \
        cp target/release/project-manager {{dist_dir}}/{{archive_name}}/bin/; \
    fi
    cd {{dist_dir}} && tar czf {{archive_name}}.tar.gz {{archive_name}}/
    @echo ""
    @echo "Archive created: {{dist_dir}}/{{archive_name}}.tar.gz"
    @echo "Contents:"
    @tar tzf {{dist_dir}}/{{archive_name}}.tar.gz

[windows]
_archive-impl:
    @echo "Creating archive: {{dist_dir}}/{{archive_name}}.zip"
    @if (-not (Test-Path "target/release/pm.exe")) { Write-Error "ERROR: pm.exe not found in target/release/"; exit 1 }
    @if (-not (Test-Path "target/release/pm-server.exe")) { Write-Error "ERROR: pm-server.exe not found in target/release/"; exit 1 }
    @New-Item -ItemType Directory -Force -Path "{{dist_dir}}/{{archive_name}}/bin" | Out-Null
    @Copy-Item "target/release/pm.exe" "{{dist_dir}}/{{archive_name}}/bin/"
    @Copy-Item "target/release/pm-server.exe" "{{dist_dir}}/{{archive_name}}/bin/"
    @if (Test-Path "target/release/project-manager.exe") { Copy-Item "target/release/project-manager.exe" "{{dist_dir}}/{{archive_name}}/bin/" }
    @Copy-Item "pm.bat" "{{dist_dir}}/{{archive_name}}/"
    @New-Item -ItemType Directory -Force -Path "{{dist_dir}}/{{archive_name}}/skills/pm" | Out-Null
    @Copy-Item ".pm/skills/pm/SKILL.md" "{{dist_dir}}/{{archive_name}}/skills/pm/"
    @Compress-Archive -Path "{{dist_dir}}/{{archive_name}}" -DestinationPath "{{dist_dir}}/{{archive_name}}.zip" -Force
    @echo ""
    @echo "Archive created: {{dist_dir}}/{{archive_name}}.zip"
    @echo "Contents:"
    @Get-ChildItem -Recurse "{{dist_dir}}/{{archive_name}}" | Select-Object -ExpandProperty FullName

# Build + archive in one step
release-build: archive

# Clean distribution artifacts
clean-dist:
    @just _clean-dist-impl

[unix]
_clean-dist-impl:
    rm -rf {{dist_dir}}

[windows]
_clean-dist-impl:
    @if (Test-Path "{{dist_dir}}") { Remove-Item -Recurse -Force "{{dist_dir}}" }

# Upload archive to an existing GitHub release (requires gh CLI)
release-upload tag:
    gh release upload {{tag}} {{dist_dir}}/{{archive_name}}.{{archive_ext}} --clobber

# Create a new GitHub release and upload the archive
release tag:
    gh release create {{tag}} {{dist_dir}}/{{archive_name}}.{{archive_ext}} --title "v{{version}}" --notes "Release v{{version}} for {{target_triple}}"

# ============================================================================
# Utility Commands
# ============================================================================

# Copy example config to .pm directory if it doesn't exist
setup-config:
  @just _setup-config-impl

[unix]
_setup-config-impl:
  mkdir -p {{config_dir}}
  @if [ ! -f {{config_file}} ]; then \
      cp {{config_example}} {{config_file}} && echo "{{msg_config_created}}"; \
  else \
      echo "{{msg_config_exists}}"; \
  fi

[windows]
_setup-config-impl:
  @if (-not (Test-Path "{{config_dir}}")) { New-Item -ItemType Directory -Force -Path "{{config_dir}}" | Out-Null }
  @if (-not (Test-Path "{{config_file}}")) { Copy-Item "{{config_example}}" "{{config_file}}"; Write-Host "{{msg_config_created}}" } else { Write-Host "{{msg_config_exists}}" }

# Show this help message
@help:
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║           Blazor Agile Board - Development Tasks              ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "📦 Quick Start:"
    echo "  just setup-config      - Copy example config (first time only)"
    echo "  just restore           - Install all dependencies"
    echo "  just check             - Full validation (restore + check + lint + test)"
    echo "  just dev               - Run desktop app in development mode"
    echo ""
    echo "🎨 Frontend (C# / Blazor):"
    echo "  just build-frontend            - Build all C# projects (Debug)"
    echo "  just build-frontend-release    - Build all C# projects (Release)"
    echo "  just publish-wasm              - Publish WASM project"
    echo "  just test-frontend             - Run all frontend tests"
    echo "  just test-cs-filter 'MyTest'   - Run tests matching filter"
    echo "  just list-tests-cs             - List all available tests"
    echo "  just watch-cs-components       - Auto-rebuild on file changes"
    echo ""
    echo "⚙️  Backend (Rust):"
    echo "  just build-backend             - Build all Rust packages (Debug)"
    echo "  just build-backend-release     - Build all Rust packages (Release)"
    echo "  just check-backend             - Fast compile check (no codegen)"
    echo "  just lint                      - Run clippy on all Rust code"
    echo "  just test-backend              - Run all backend tests"
    echo "  just watch-rs-db               - Auto-check on file changes"
    echo ""
    echo "🚀 Production:"
    echo "  just build-release             - Build backend + frontend (Release)"
    echo "  just build                     - Full production Tauri build"
    echo ""
    echo "📦 Distribution:"
    echo "  just build-portable          - Build all binaries (release)"
    echo "  just archive                 - Build + create distribution archive"
    echo "  just release-build           - Alias for archive"
    echo "  just release <tag>           - Create GitHub release with archive"
    echo "  just release-upload <tag>    - Upload to existing GitHub release"
    echo "  just clean-dist              - Remove dist/ directory"
    echo ""
    echo "🧹 Maintenance:"
    echo "  just clean                     - Clean all build artifacts"
    echo ""
    echo "📋 Full command list: just --list"
