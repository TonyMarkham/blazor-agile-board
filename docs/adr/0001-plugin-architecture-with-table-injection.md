# ADR-0001: Plugin Architecture with Table Injection

## Status
Accepted

## Context
The project management application needs to work both as a standalone application and as a plugin for an existing multi-tenant SaaS coaching platform. We need to decide how to structure the data storage to support both deployment modes while maintaining clean integration with the host platform.

Two primary options were considered:
1. Separate database per plugin (e.g., `project_management.db` alongside `main.db`)
2. Inject plugin tables into the tenant's primary database with namespaced prefixes

## Decision
We will inject plugin tables into each tenant's primary SQLite database using a `pm_` prefix for all table names.

All tables will be prefixed with `pm_`:
- `pm_projects`
- `pm_tasks`
- `pm_sprints`
- `pm_comments`
- etc.

## Consequences

### Positive
- **Single source of truth**: One database file contains all tenant data, simplifying LLM context access
- **Native foreign keys**: Can directly reference platform tables (users, teams, clients) without synchronization
- **Simplified connection management**: No need to manage multiple database connections per tenant
- **Better data cohesion**: Project management data lives alongside the business data it relates to
- **LLM-friendly**: AI assistants can analyze complete tenant context from a single SQLite file

### Negative
- **Tighter coupling**: Plugin schema is integrated with platform schema
- **Migration coordination**: Plugin migrations must be coordinated with platform upgrades
- **Namespace management**: Must maintain consistent `pm_` prefixing to avoid collisions
- **Uninstall complexity**: Removing plugin requires dropping all `pm_*` tables (though this is scriptable)

### Mitigation Strategies
- Strict naming convention enforcement for all plugin tables
- Plugin manages its own migration lifecycle independently
- Clear documentation of foreign key relationships to platform tables
- Uninstall script to cleanly remove all plugin data if needed
