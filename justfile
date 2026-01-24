# justfile - task runner for Blazor Agile Board

# === Directory Structure ===
frontend_dir := "frontend"
backend_dir := "backend"
desktop_dir := "desktop"
config_dir := ".pm"
coverage_dir := "./coverage"

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
rust_tauri := "desktop"

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
    dotnet clean {{frontend_solution}}
    rm -rf {{frontend_bin_pattern}} {{frontend_obj_pattern}}

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
check-rust-server:
    cargo check -p {{rust_server}} {{cargo_all_targets}}

check-rust-core:
    cargo check -p {{rust_core}} {{cargo_all_targets}}

check-rust-db:
    cargo check -p {{rust_db}} {{cargo_all_targets}}

check-rust-auth:
    cargo check -p {{rust_auth}} {{cargo_all_targets}}

check-rust-proto:
    cargo check -p {{rust_proto}} {{cargo_all_targets}}

check-rust-ws:
    cargo check -p {{rust_ws}} {{cargo_all_targets}}

check-rust-config:
    cargo check -p {{rust_config}} {{cargo_all_targets}}

# Clippy specific package
clippy-rust-server:
    cargo clippy -p {{rust_server}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rust-core:
    cargo clippy -p {{rust_core}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rust-db:
    cargo clippy -p {{rust_db}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rust-auth:
    cargo clippy -p {{rust_auth}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rust-proto:
    cargo clippy -p {{rust_proto}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rust-ws:
    cargo clippy -p {{rust_ws}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

clippy-rust-config:
    cargo clippy -p {{rust_config}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings

# Build specific package (debug)
build-rust-server:
    cargo build -p {{rust_server}}

build-rust-core:
    cargo build -p {{rust_core}}

build-rust-db:
    cargo build -p {{rust_db}}

build-rust-auth:
    cargo build -p {{rust_auth}}

build-rust-proto:
    cargo build -p {{rust_proto}}

build-rust-ws:
    cargo build -p {{rust_ws}}

build-rust-config:
    cargo build -p {{rust_config}}

# Build specific package (release)
build-rust-server-release:
    cargo build -p {{rust_server}} {{cargo_release}}

build-rust-core-release:
    cargo build -p {{rust_core}} {{cargo_release}}

build-rust-db-release:
    cargo build -p {{rust_db}} {{cargo_release}}

build-rust-auth-release:
    cargo build -p {{rust_auth}} {{cargo_release}}

build-rust-proto-release:
    cargo build -p {{rust_proto}} {{cargo_release}}

build-rust-ws-release:
    cargo build -p {{rust_ws}} {{cargo_release}}

build-rust-config-release:
    cargo build -p {{rust_config}} {{cargo_release}}

# Test specific package
test-rust-server:
    cargo test -p {{rust_server}}

test-rust-core:
    cargo test -p {{rust_core}}

test-rust-db:
    cargo test -p {{rust_db}}

test-rust-auth:
    cargo test -p {{rust_auth}}

test-rust-proto:
    cargo test -p {{rust_proto}}

test-rust-ws:
    cargo test -p {{rust_ws}}

test-rust-config:
    cargo test -p {{rust_config}}

# Watch mode - auto-rebuild on file changes
watch-rust-server:
    cargo watch -x 'check -p {{rust_server}}'

watch-rust-core:
    cargo watch -x 'check -p {{rust_core}}'

watch-rust-db:
    cargo watch -x 'check -p {{rust_db}}'

watch-rust-auth:
    cargo watch -x 'check -p {{rust_auth}}'

watch-rust-proto:
    cargo watch -x 'check -p {{rust_proto}}'

watch-rust-ws:
    cargo watch -x 'check -p {{rust_ws}}'

watch-rust-config:
    cargo watch -x 'check -p {{rust_config}}'

# Watch mode - auto-test on file changes
watch-test-rust-server:
    cargo watch -x 'test -p {{rust_server}}'

watch-test-rust-core:
    cargo watch -x 'test -p {{rust_core}}'

watch-test-rust-db:
    cargo watch -x 'test -p {{rust_db}}'

watch-test-rust-auth:
    cargo watch -x 'test -p {{rust_auth}}'

watch-test-rust-proto:
    cargo watch -x 'test -p {{rust_proto}}'

watch-test-rust-ws:
    cargo watch -x 'test -p {{rust_ws}}'

watch-test-rust-config:
    cargo watch -x 'test -p {{rust_config}}'

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
    cargo build -p {{rust_server}} & \
    dotnet publish {{wasm_project}} -c {{config_debug}} {{dotnet_no_restore}} & \
    wait

# Build both backend and frontend (release, parallel)
build-release:
    just setup-config
    just restore
    cargo build -p {{rust_server}} {{cargo_release}} & \
    dotnet publish {{wasm_project}} -c {{config_release}} {{dotnet_no_restore}} & \
    wait

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
    cd {{desktop_dir}} && cargo tauri dev

# Full production build with Tauri
build:
    just build-release
    cd {{desktop_dir}} && cargo tauri build

# ============================================================================
# Utility Commands
# ============================================================================

# Copy example config to .pm directory if it doesn't exist
setup-config:
    mkdir -p {{config_dir}}
    @if [ ! -f {{config_file}} ]; then \
        cp {{config_example}} {{config_file}} && \
        echo "{{msg_config_created}}"; \
    else \
        echo "{{msg_config_exists}}"; \
    fi

# List all available commands
help:
    @just --list
