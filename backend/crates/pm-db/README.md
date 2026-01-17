# pm-db: Database Layer

SQLite-based repository layer with SQLx for compile-time query verification.

---

## Database Workflow

### 1. Create a Migration

**When:** Schema changes needed (add table, add column, modify constraints, etc.)

**Command:**
```bash
# From backend/crates/pm-db/
sqlx migrate add <descriptive_name>

# Example:
sqlx migrate add add_priority_column
```

**Result:** Creates `migrations/TIMESTAMP_<descriptive_name>.sql`

**Edit the file** and add your SQL:
```sql
-- Migration: add_priority_column
-- Description of what this migration does

ALTER TABLE pm_work_items ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium';
```

---

### 2. Run Migrations (Apply Schema Changes)

**When:** After creating a migration, or when pulling new migrations from git

**Command:**
```bash
# From backend/crates/pm-db/
sqlx migrate run --database-url sqlite:.sqlx-test/test.db
```

**What this does:**
- Executes all pending migrations on the test database
- Updates schema to match your SQL files
- Records which migrations have been applied

---

### 3. Regenerate Query Cache (Update SQLx Metadata)

**When:** After running migrations that change schema

**Command:**
```bash
# From backend/ (workspace root)
DATABASE_URL=sqlite:crates/pm-db/.sqlx-test/test.db cargo sqlx prepare --workspace
```

**What this does:**
- Connects to database
- Validates all `sqlx::query!()` macros in the workspace
- Generates `.sqlx/` cache files for offline compilation
- Enables CI builds without a live database

---

## Complete Workflow Example

**Scenario:** Add `priority` column to `pm_work_items` table

```bash
# Step 1: Create migration
cd backend/crates/pm-db
sqlx migrate add add_priority_column

# Step 2: Edit migration file
# (Add your ALTER TABLE statement)

# Step 3: Run migration
sqlx migrate run --database-url sqlite:.sqlx-test/test.db

# Step 4: Update Rust code
# (Update WorkItem struct, repository queries, etc.)

# Step 5: Regenerate query cache
cd ../..  # back to backend/
DATABASE_URL=sqlite:crates/pm-db/.sqlx-test/test.db cargo sqlx prepare --workspace

# Step 6: Verify build
cargo build --workspace
```

---

## Troubleshooting

### "SQLX_OFFLINE=true but there is no cached data"

**Cause:** Query cache is stale (schema changed but cache not regenerated)

**Fix:** Run step 3 above (regenerate query cache)

### "set DATABASE_URL to use query macros online"

**Cause:** `cargo sqlx prepare` needs database connection

**Fix:** Ensure `DATABASE_URL` is set when running `prepare`:
```bash
DATABASE_URL=sqlite:crates/pm-db/.sqlx-test/test.db cargo sqlx prepare --workspace
```

### "no such column: priority"

**Cause:** Migration exists but hasn't been applied to test database

**Fix:** Run step 2 above (run migrations)

---

## Database Location

**Test Database:** `crates/pm-db/.sqlx-test/test.db`

**Production:** Per-tenant SQLite files at `/data/tenants/{tenant_id}/main.db`

---

## Common Tasks

**Check migration status:**
```bash
sqlx migrate info --database-url sqlite:.sqlx-test/test.db
```

**Inspect test database schema:**
```bash
sqlite3 .sqlx-test/test.db ".schema pm_work_items"
```

**Reset test database (reapply all migrations):**
```bash
rm .sqlx-test/test.db
sqlx migrate run --database-url sqlite:.sqlx-test/test.db
```

---

## CI/CD Note

The `.sqlx/` directory is checked into git. This allows CI builds to run without a database connection by setting `SQLX_OFFLINE=true`. Always regenerate the cache before committing schema changes.
