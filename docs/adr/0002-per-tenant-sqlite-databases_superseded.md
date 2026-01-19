# ADR-0002: Per-Tenant SQLite Databases

## Status
**Superseded** by [ADR-0006](0006-single-tenant-desktop-first.md)

> **Note (2026-01-19)**: This ADR described the original multi-tenant SaaS architecture. The project has pivoted to a desktop-first, single-tenant approach. See ADR-0006 for the current architecture. This ADR is retained for historical context.

## Context
The application is designed for a multi-tenant SaaS environment. We need to determine the database architecture for tenant isolation. The host coaching platform uses a dedicated SQLite database per tenant.

Options considered:
1. Single shared database with tenant_id columns
2. Dedicated SQLite database per tenant
3. Hybrid approach with shared schema database and per-tenant data stores

## Decision
We will use dedicated SQLite database files per tenant, matching the existing architecture of the host coaching platform.

Database file structure:
```
/data/tenants/{tenant_id}/main.db
```

The plugin tables (prefixed with `pm_`) will be injected into each tenant's `main.db` file.

## Consequences

### Positive
- **Complete data isolation**: Each tenant's data is physically separated at the filesystem level
- **LLM accessibility**: Entire tenant context (platform + plugin data) is in a single file that can be easily read by LLMs
- **Simplified queries**: No need for tenant_id in WHERE clauses; database file itself provides isolation
- **Data portability**: Easy tenant export, backup, and migration (just copy the file)
- **Performance**: No cross-tenant query concerns or index bloat from multi-tenant columns
- **Security**: Filesystem permissions provide an additional security layer
- **Platform consistency**: Matches existing SaaS architecture

### Negative
- **Connection management**: Need to maintain connection pools per active tenant
- **Schema migrations**: Must apply migrations to each tenant database individually
- **Cross-tenant queries**: Impossible to query across tenants (though this is rarely needed and is a security feature)
- **Storage overhead**: Each database has some filesystem overhead

### Implementation Notes
- Rust backend will use a connection pool manager that caches pools per tenant
- Tenant resolution happens via middleware extracting tenant_id from JWT claims
- Lazy loading of database connections (only open when needed)
- Migration tooling will iterate over all tenant databases
- Consider connection pool eviction strategy for inactive tenants
