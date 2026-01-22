# justfile - task runner for Blazor Agile Board

# Build backend and frontend for development (parallel)
build-dev:
    just setup-config
    cargo build -p pm-server & \
    dotnet publish frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj -c Debug & \
    wait

# Build backend and frontend for release (parallel)
build-release:
    just setup-config
    cargo build -p pm-server --release & \
    dotnet publish frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj -c Release & \
    wait

# Build only backend (debug)
build-backend:
    cargo build -p pm-server

# Build only backend (release)
build-backend-release:
    cargo build -p pm-server --release

# Build only frontend (debug)
build-frontend:
    dotnet publish frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj -c Debug

# Build only frontend (release)
build-frontend-release:
    dotnet publish frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj -c Release

# Clean all build artifacts
clean:
    cargo clean
    dotnet clean frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj

# Run development build then start Tauri
dev:
    just build-dev
    cd desktop && cargo tauri dev

# Full production build
build:
    just build-release
    cd desktop && cargo tauri build

# Copy example config to .pm directory if it doesn't exist
setup-config:
  mkdir -p .pm
  @if [ ! -f .pm/config.toml ]; then \
      cp backend/config.example.toml .pm/config.toml && \
      echo "Created .pm/config.toml from example"; \
  else \
      echo ".pm/config.toml already exists, skipping"; \
  fi
