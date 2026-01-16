# Session 30.1: Schema & Repository Infrastructure

**STATUS: ✅ COMPLETED**

> **⚠️ CRITICAL**: The contents of [`CRITICAL_OPERATING_CONSTRAINTS.md`](../CRITICAL_OPERATING_CONSTRAINTS.md) apply to this implementation session.
> - **Teaching Mode**: Do NOT write/edit files unless explicitly asked to "implement"
> - **Production-Grade**: No shortcuts, no TODOs, comprehensive error handling
> - **Plan First**: Read entire step, identify sub-tasks, present approach before coding

---

## Overview

Add database schema changes for optimistic locking, authorization, and idempotency. Create corresponding repositories.

**Estimated Files**: 7
**Dependencies**: Session 20 complete (WebSocket infrastructure exists)

---

## Phase 1: Database Migrations

### 1.1 Add version column for optimistic locking

**File**: `pm-db/migrations/003_add_version_column.sql`

```sql
-- Migration: add_version_column
-- Enables optimistic locking to prevent silent data loss from concurrent updates

ALTER TABLE pm_work_items ADD COLUMN version INTEGER NOT NULL DEFAULT 0;
```

**Why**: Client sends `expected_version`, server rejects if stale. Prevents concurrent update conflicts.

---

### 1.2 Add project_members table for authorization

**File**: `pm-db/migrations/004_add_project_members.sql`

```sql
-- Migration: add_project_members
-- Enforces project-level access control

CREATE TABLE pm_project_members (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('viewer', 'editor', 'admin')),
    created_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id),
    UNIQUE(project_id, user_id)
);

CREATE INDEX idx_pm_project_members_project ON pm_project_members(project_id);
CREATE INDEX idx_pm_project_members_user ON pm_project_members(user_id);
```

**Roles**:
- `viewer`: Read-only access
- `editor`: CRUD operations on work items
- `admin`: Manage members + all editor permissions

---

### 1.3 Add idempotency tracking table

**File**: `pm-db/migrations/005_add_idempotency_keys.sql`

```sql
-- Migration: add_idempotency_keys
-- Prevents duplicate creates on network retries

CREATE TABLE pm_idempotency_keys (
    message_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Cleanup old entries (run periodically via background job)
-- DELETE FROM pm_idempotency_keys WHERE created_at < (unixepoch() - 3600);
```

**Why**: Store result of create operations, return cached on replay with same `message_id`.

---

## Phase 2: New Repositories

### 2.1 Project Member Repository

**File**: `pm-db/src/repositories/project_member_repository.rs`

```rust
use sqlx::SqlitePool;
use uuid::Uuid;
use crate::models::ProjectMember;
use crate::error::DbError;

pub struct ProjectMemberRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ProjectMemberRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_user_and_project(
        &self,
        user_id: Uuid,
        project_id: Uuid,
    ) -> Result<Option<ProjectMember>, DbError> {
        let user_id_str = user_id.to_string();
        let project_id_str = project_id.to_string();

        let result = sqlx::query_as!(
            ProjectMember,
            r#"
            SELECT id, project_id, user_id, role, created_at
            FROM pm_project_members
            WHERE user_id = ? AND project_id = ?
            "#,
            user_id_str,
            project_id_str
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(result)
    }

    pub async fn find_by_project(&self, project_id: Uuid) -> Result<Vec<ProjectMember>, DbError> {
        let project_id_str = project_id.to_string();

        let results = sqlx::query_as!(
            ProjectMember,
            r#"
            SELECT id, project_id, user_id, role, created_at
            FROM pm_project_members
            WHERE project_id = ?
            "#,
            project_id_str
        )
        .fetch_all(self.pool)
        .await?;

        Ok(results)
    }

    pub async fn create(&self, member: &ProjectMember) -> Result<(), DbError> {
        let id_str = member.id.to_string();
        let project_id_str = member.project_id.to_string();
        let user_id_str = member.user_id.to_string();

        sqlx::query!(
            r#"
            INSERT INTO pm_project_members (id, project_id, user_id, role, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
            id_str,
            project_id_str,
            user_id_str,
            member.role,
            member.created_at
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool, DbError> {
        let id_str = id.to_string();

        let result = sqlx::query!(
            "DELETE FROM pm_project_members WHERE id = ?",
            id_str
        )
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
```

---

### 2.2 Idempotency Repository

**File**: `pm-db/src/repositories/idempotency_repository.rs`

```rust
use sqlx::SqlitePool;
use chrono::Utc;
use crate::error::DbError;

pub struct IdempotencyRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> IdempotencyRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_message_id(&self, message_id: &str) -> Result<Option<String>, DbError> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT result_json
            FROM pm_idempotency_keys
            WHERE message_id = ?
            "#,
            message_id
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(result)
    }

    pub async fn create(
        &self,
        message_id: &str,
        operation: &str,
        result_json: &str,
    ) -> Result<(), DbError> {
        let created_at = Utc::now().timestamp();

        sqlx::query!(
            r#"
            INSERT INTO pm_idempotency_keys (message_id, operation, result_json, created_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(message_id) DO NOTHING
            "#,
            message_id,
            operation,
            result_json,
            created_at
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_old_entries(&self, max_age_seconds: i64) -> Result<u64, DbError> {
        let cutoff = Utc::now().timestamp() - max_age_seconds;

        let result = sqlx::query!(
            "DELETE FROM pm_idempotency_keys WHERE created_at < ?",
            cutoff
        )
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
```

---

### 2.3 ProjectMember Model

**File**: `pm-db/src/models/project_member.rs`

```rust
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProjectMember {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub role: String,
    pub created_at: i64,
}

impl ProjectMember {
    pub fn new(project_id: Uuid, user_id: Uuid, role: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            user_id: user_id.to_string(),
            role: role.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn has_permission(&self, required: Permission) -> bool {
        match (self.role.as_str(), required) {
            ("admin", _) => true,
            ("editor", Permission::View | Permission::Edit) => true,
            ("viewer", Permission::View) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Permission {
    View,
    Edit,
    Admin,
}
```

---

## Phase 3: Infrastructure Updates

### 3.1 Database Timeout Configuration

**File**: `pm-db/src/connection/tenant_connection_manager.rs` (modify)

Add acquire timeout to pool options:

```rust
use std::time::Duration;

// In create_pool or equivalent function:
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .acquire_timeout(Duration::from_secs(5))  // ADD: Fail fast if pool exhausted
    .connect_with(options)
    .await?;
```

---

### 3.2 Update Repository Module Exports

**File**: `pm-db/src/repositories/mod.rs` (modify)

```rust
// Add new exports:
mod project_member_repository;
mod idempotency_repository;

pub use project_member_repository::ProjectMemberRepository;
pub use idempotency_repository::IdempotencyRepository;
```

---

## File Summary

| Action | Path |
|--------|------|
| Create | `pm-db/migrations/003_add_version_column.sql` |
| Create | `pm-db/migrations/004_add_project_members.sql` |
| Create | `pm-db/migrations/005_add_idempotency_keys.sql` |
| Create | `pm-db/src/models/project_member.rs` |
| Create | `pm-db/src/repositories/project_member_repository.rs` |
| Create | `pm-db/src/repositories/idempotency_repository.rs` |
| Modify | `pm-db/src/repositories/mod.rs` |
| Modify | `pm-db/src/models/mod.rs` |
| Modify | `pm-db/src/connection/tenant_connection_manager.rs` |

---

## Verification

```bash
cd backend
cargo build --workspace
cargo test -p pm-db

# Run migrations on test DB
sqlx migrate run --database-url sqlite:test.db
```

---

## Tests to Add

```rust
#[tokio::test]
async fn test_project_member_find_by_user_and_project() {
    // Setup test DB with member
    // Verify find returns member
    // Verify find for non-member returns None
}

#[tokio::test]
async fn test_idempotency_create_and_find() {
    // Create idempotency entry
    // Find should return cached result
    // Second create with same ID should be no-op
}

#[tokio::test]
async fn test_idempotency_cleanup() {
    // Create old entry
    // Run cleanup
    // Verify old entry removed
}
```
