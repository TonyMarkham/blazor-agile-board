# Notes

## Quick Reference

### Install sqlx-cli
```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

### Run Tests (no database setup needed)
```bash
cargo test --workspace
```
Tests use in-memory SQLite - migrations run automatically via `create_test_pool()`.

---

## Schema Changes Workflow

All commands run from repo root (where `Cargo.toml` workspace is).

### 1. Create Migration
```bash
cd backend/crates/pm-db
sqlx migrate add <descriptive_name>
# Edit the generated migrations/TIMESTAMP_<name>.sql file
cd ../../..  # back to repo root
```

### 2. Run Migration on Test DB
```bash
sqlx migrate run \
  --source backend/crates/pm-db/migrations \
  --database-url sqlite:backend/crates/pm-db/.sqlx-test/test.db
```

### 3. Update Query Cache
```bash
DATABASE_URL=sqlite:backend/crates/pm-db/.sqlx-test/test.db cargo sqlx prepare --workspace
```
This updates `.sqlx/` at the repo root.

### 4. Verify
```bash
cargo build --workspace
cargo test --workspace
```

---

## Useful Commands

```bash
# Check migration status
sqlx migrate info \
  --source backend/crates/pm-db/migrations \
  --database-url sqlite:backend/crates/pm-db/.sqlx-test/test.db

# Inspect schema
sqlite3 backend/crates/pm-db/.sqlx-test/test.db ".schema pm_work_items"

# Reset test database
rm backend/crates/pm-db/.sqlx-test/test.db
sqlx migrate run \
  --source backend/crates/pm-db/migrations \
  --database-url sqlite:backend/crates/pm-db/.sqlx-test/test.db
```

---

## Directory Layout

```
blazor-agile-board/          # Workspace root
├── Cargo.toml               # Workspace manifest
├── .sqlx/                   # Query cache (at workspace level)
└── backend/crates/pm-db/
    ├── migrations/          # SQL migrations
    └── .sqlx-test/test.db   # Test database
```
