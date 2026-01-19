# ADR-0004: Rust Axum Backend

## Status
Accepted

> **Updated (2026-01-19)**: pm-server now runs as a Tauri sidecar for desktop deployment (see [ADR-0006](0006-single-tenant-desktop-first.md)). The same binary works for both desktop (sidecar) and future SaaS (orchestrated) deployments.

## Context
The project management application requires a backend API server. For desktop deployment, the server runs as a sidecar process managed by Tauri. We need a backend technology that is efficient, compiles to a single binary, and can work both as an embedded sidecar and as a standalone service.

Options considered:
1. ASP.NET Core (matches frontend technology)
2. Node.js/Express (popular, easy to integrate)
3. Rust with Axum (matches host platform)
4. Go with standard library or framework

## Decision
We will build the backend using Rust with the Axum web framework.

Technology stack:
- **Axum**: Web framework built on tokio and tower
- **SQLx**: Async SQL with compile-time query verification
- **Tokio**: Async runtime
- **Tower**: Middleware ecosystem
- **Prost**: Protocol Buffers implementation

## Consequences

### Positive
- **Platform alignment**: Identical tech stack to the coaching SaaS platform
- **Seamless integration**: Plugin backend can share infrastructure, middleware, and patterns
- **Performance**: Rust's performance characteristics suitable for real-time WebSocket connections
- **Type safety**: Strong typing catches errors at compile time
- **Memory safety**: No garbage collection pauses, predictable performance
- **Async-first**: Native async/await with tokio for WebSocket and database operations
- **SQLx compile-time verification**: SQL queries checked against database schema at compile time
- **Single deployment**: Can deploy as standalone or integrate into platform monolith

### Negative
- **Learning curve**: Rust has steeper learning curve than dynamic languages
- **Compile times**: Rust compilation can be slow for large projects
- **Ecosystem maturity**: Some libraries less mature than Node.js/Python equivalents
- **Development velocity**: May be slower to prototype than with dynamic languages

### Architecture Benefits
- **Middleware reuse**: Can use same auth, tenant context, logging middleware as host platform
- **Connection pooling**: Integrate with platform's existing database connection management
- **Error handling**: Consistent error types and handling across platform and plugin
- **Observability**: Same tracing, metrics, and logging infrastructure

### Deployment Patterns
- **Desktop (Tauri sidecar)**: pm-server binary bundled with app, started/stopped by Tauri lifecycle
- **Standalone**: Axum server runs independently for development or headless deployment
- **SaaS (future)**: Orchestrator spawns pm-server instances per tenant
- All modes share identical business logic and data access code
