# pm-db: Database Layer

SQLite-based repository layer with SQLx for compile-time query verification.

---

## Database Workflow

> **Important:** All commands below are run from the **repo root** directory unless explicitly stated otherwise.
> The workspace `Cargo.toml` is at the repo root, and the `.sqlx/` cache lives there too.

---

## Standard Workflow (After Schema Changes)

**When to use:** After pulling schema changes from git or creating a new migration.

**Working directory:** Repo root (`blazor-agile-board/`)

### Step 1: Run all migrations

**Why:** Applies all migrations in order, establishing the complete schema.

```bash
sqlx migrate run --source backend/crates/pm-db/migrations --database-url sqlite:backend/crates/pm-db/.sqlx-test/test.db
```

**Expected output:** "Applied 20260110000000/migrate...", "Applied 20260110000001/migrate...", etc. for all migrations.

### Step 2: Regenerate SQLx query cache

**Why:** Validates all `sqlx::query!()` macros against the actual database schema and generates `.sqlx/*.json` files for offline compilation.

```bash
DATABASE_URL=sqlite:backend/crates/pm-db/.sqlx-test/test.db cargo sqlx prepare --workspace
```

**Expected output:** "Checking pm-proto", "Checking pm-db", etc. Creates `.sqlx/` directory with JSON files.

### Step 3: Build workspace

**Why:** Verifies all code compiles correctly with the new schema using offline mode.

```bash
cargo build --workspace
```

### Step 4: Run tests

**Why:** Verifies all repository logic works correctly. Tests use in-memory databases with migrations applied automatically.

```bash
cargo test --workspace
```

---

## Adding a New Migration

**When:** You need to change the database schema (add table, add column, modify constraints, etc.)

**Working directory:** `backend/crates/pm-db/`

### Step 1: Create migration file

**Why:** Generates a timestamped migration file to track schema changes.

```bash
cd backend/crates/pm-db
```

```bash
sqlx migrate add <descriptive_name>
```

**Example:**
```bash
sqlx migrate add add_priority_column
```

**Result:** Creates `migrations/TIMESTAMP_<descriptive_name>.sql`

### Step 2: Edit the migration file

Add your SQL to the generated file:

```sql
-- Migration: add_priority_column
-- Description of what this migration does

ALTER TABLE pm_work_items ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium';
```

### Step 3: Return to repo root and apply the standard workflow

**Why:** After schema changes, you must regenerate the query cache.

```bash
cd ../../..
```

Then follow the **Standard Workflow** steps above.

---

## First-Time Setup

**When to use:** Setting up the database for the first time on a new machine.

**Working directory:** Repo root (`blazor-agile-board/`)

### Step 1: Create test database directory

```bash
mkdir -p backend/crates/pm-db/.sqlx-test
```

### Step 2: Create the database file

```bash
sqlx database create --database-url sqlite:backend/crates/pm-db/.sqlx-test/test.db
```

### Step 3: Follow the standard workflow

Run Steps 1-4 from the **Standard Workflow** section above to apply migrations, regenerate cache, build, and test.

---

## Troubleshooting

### "SQLX_OFFLINE=true but there is no cached data"

**Cause:** Query cache doesn't exist or is stale.

**Fix:** Run Step 2 of the Standard Workflow (regenerate query cache).

### "set DATABASE_URL to use query macros online"

**Cause:** Trying to build without the query cache and without DATABASE_URL set.

**Fix:** Run Step 2 of the Standard Workflow (regenerate query cache).

### "no such column: priority" or "no such table: pm_work_items_old"

**Cause:** Query cache was generated mid-migration or with wrong schema.

**Fix:** Delete `.sqlx/` directory and run Step 2 of the Standard Workflow:
```bash
rm -rf .sqlx
DATABASE_URL=sqlite:backend/crates/pm-db/.sqlx-test/test.db cargo sqlx prepare --workspace
```

### "error returned from database: (code: 1) no such table"

**Cause:** Test database is out of sync with migrations or doesn't exist.

**Fix:** Run the First-Time Setup, then the Standard Workflow.

### Starting completely fresh (nuclear option)

If you want to completely reset everything:

```bash
rm -rf .sqlx
rm -rf backend/crates/pm-db/.sqlx-test
```

Then follow the **First-Time Setup** and **Standard Workflow** procedures.

---

## Directory Layout

```
blazor-agile-board/                    # Repo root (workspace Cargo.toml)
├── Cargo.toml                         # Workspace manifest
├── .sqlx/                             # Query cache (workspace level)
│   └── query-*.json                   # Generated cache files
└── backend/crates/pm-db/
    ├── migrations/                    # SQL migration files
    │   └── TIMESTAMP_*.sql
    ├── .sqlx-test/                    # Test database directory
    │   └── test.db                    # SQLite test database
    └── src/                           # Repository code
```

---

## Common Inspection Commands

**Working directory:** Repo root

**Check which migrations have been applied:**

```bash
sqlx migrate info --source backend/crates/pm-db/migrations --database-url sqlite:backend/crates/pm-db/.sqlx-test/test.db
```

**Inspect table schema:**

```bash
sqlite3 backend/crates/pm-db/.sqlx-test/test.db ".schema pm_work_items"
```

**List all tables:**

```bash
sqlite3 backend/crates/pm-db/.sqlx-test/test.db ".tables"
```

**Check foreign keys on a table:**

```bash
sqlite3 backend/crates/pm-db/.sqlx-test/test.db "PRAGMA foreign_key_list(pm_work_items);"
```

---

## CI/CD Note

The `.sqlx/` directory at the repo root is checked into git. This allows CI builds to run without a database connection by setting `SQLX_OFFLINE=true`.

**Before committing schema changes:**
1. Run the Standard Workflow
2. Verify tests pass
3. Commit both the migration file AND the updated `.sqlx/` directory

---

## Why This Process?

**Q: When do I need to regenerate the query cache?**

A: After any migration that changes table structure, columns, or constraints. The cache contains metadata about every `sqlx::query!()` macro in the codebase and must match the actual database schema.

**Q: Why not use `cargo sqlx prepare --check`?**

A: That command validates existing cache but doesn't regenerate it. After schema changes, you need full regeneration with `cargo sqlx prepare --workspace`.

**Q: Why is `.sqlx-test/` not in `.gitignore`?**

A: It's a local development database that can be regenerated. Only `.sqlx/` (the cache) is checked into git for CI/CD offline builds.

**Q: What if the database gets corrupted or out of sync?**

A: The database file is disposable. Delete `backend/crates/pm-db/.sqlx-test/` and follow the First-Time Setup procedure.
