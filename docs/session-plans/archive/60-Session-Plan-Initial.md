# Session 60: Time Tracking & Dependencies - Implementation Plan

## Overview

Implements time tracking (running timers + manual entries) and dependency management (blocking relationships with cycle detection).

---

## Existing Infrastructure (Ready to Use)

Already implemented in previous sessions:
- **Database**: `pm_time_entries`, `pm_dependencies` tables with migrations
- **Rust Models**: `TimeEntry`, `Dependency`, `DependencyType` in pm-core
- **Repositories**: `TimeEntryRepository`, `DependencyRepository` in pm-db
- **Proto Entities**: `TimeEntry`, `Dependency` message definitions (but NOT WebSocket commands/events)

---

## Business Rules (Critical)

### Time Tracking Rules
1. **One active timer per user** - Starting a new timer auto-stops any existing timer
2. **Owner-only mutations** - Only the user who created a time entry can edit/delete it
3. **Atomicity** - StartTimer must be atomic (check-stop-create in single transaction)
4. **UTC timestamps** - All timestamps stored and transmitted as UTC Unix seconds
5. **Duration calculated on stop** - `duration_seconds = ended_at - started_at`
6. **Soft deletes** - All queries filter `WHERE deleted_at IS NULL`
7. **Max description** - 1000 characters
8. **Max duration** - 24 hours (86400 seconds) for manual entries
9. **No future timestamps** - started_at/ended_at cannot be in the future (60s tolerance for clock drift)

### Dependency Rules
1. **No self-reference** - Item cannot block itself
2. **No duplicates** - Same (blocking, blocked) pair cannot exist twice
3. **No cycles for Blocks type** - A→B→C→A detected and rejected with path in error
4. **RelatesTo allows bidirectional** - A relates_to B and B relates_to A is valid
5. **Same-project only** - Dependencies can only be created between items in the same project
6. **Edit permission required** - Must have Edit on the project to create/delete dependencies
7. **Soft deletes** - All queries filter `WHERE deleted_at IS NULL`
8. **Max dependencies** - 50 blocking + 50 blocked per item (prevent graph explosion)

---

## Implementation Order (By Dependency)

### Phase 1: Protocol Definition (Required by Everything)

**1.1 Protobuf Messages** (`proto/messages.proto`)

All handlers and frontend depend on these message definitions.

Add new messages (field numbers 110-139 in WebSocketMessage.payload oneof):

```protobuf
// === Time Entry Commands (110-119) ===
message StartTimerRequest {
  string work_item_id = 1;
  optional string description = 2;
}
message StopTimerRequest {
  string time_entry_id = 1;
}
message CreateTimeEntryRequest {
  string work_item_id = 1;
  int64 started_at = 2;      // UTC Unix seconds
  int64 ended_at = 3;        // UTC Unix seconds
  optional string description = 4;
}
message UpdateTimeEntryRequest {
  string time_entry_id = 1;
  optional int64 started_at = 2;
  optional int64 ended_at = 3;
  optional string description = 4;
}
message DeleteTimeEntryRequest {
  string time_entry_id = 1;
}
message GetTimeEntriesRequest {
  string work_item_id = 1;
  optional int32 limit = 2;   // Default 100, max 500
  optional int32 offset = 3;  // For pagination
}
message GetRunningTimerRequest {}

// === Time Entry Events (120-129) ===
message TimerStarted {
  TimeEntry time_entry = 1;
  string user_id = 2;
  optional TimeEntry stopped_entry = 3;  // Previous timer that was auto-stopped
}
message TimerStopped {
  TimeEntry time_entry = 1;
  string user_id = 2;
}
message TimeEntryCreated {
  TimeEntry time_entry = 1;
  string user_id = 2;
}
message TimeEntryUpdated {
  TimeEntry time_entry = 1;
  string user_id = 2;
}
message TimeEntryDeleted {
  string time_entry_id = 1;
  string work_item_id = 2;  // For UI to know which list to update
  string user_id = 3;
}
message TimeEntriesList {
  repeated TimeEntry time_entries = 1;
  int32 total_count = 2;  // For pagination
}
message RunningTimerResponse {
  optional TimeEntry time_entry = 1;
}

// === Dependency Commands (130-134) ===
message CreateDependencyRequest {
  string blocking_item_id = 1;
  string blocked_item_id = 2;
  DependencyType dependency_type = 3;
}
message DeleteDependencyRequest {
  string dependency_id = 1;
}
message GetDependenciesRequest {
  string work_item_id = 1;
}

// === Dependency Events (135-139) ===
message DependencyCreated {
  Dependency dependency = 1;
  string user_id = 2;
}
message DependencyDeleted {
  string dependency_id = 1;
  string blocking_item_id = 2;  // For UI to know which lists to update
  string blocked_item_id = 3;
  string user_id = 4;
}
message DependenciesList {
  repeated Dependency blocking = 1;  // Items blocking this one
  repeated Dependency blocked = 2;   // Items blocked by this one
}
```

---

### Phase 2: Backend Infrastructure (Required by Handlers)

**2.1 Configuration Constants** (`pm-config/src/validation_config.rs`)

Add to existing validation config:

```rust
// Time Entry limits
pub const MAX_TIME_ENTRY_DESCRIPTION_LENGTH: usize = 1000;
pub const MAX_TIME_ENTRY_DURATION_SECONDS: i64 = 86400; // 24 hours
pub const MAX_FUTURE_TIMESTAMP_TOLERANCE_SECONDS: i64 = 60;
pub const DEFAULT_TIME_ENTRIES_LIMIT: i32 = 100;
pub const MAX_TIME_ENTRIES_LIMIT: i32 = 500;

// Dependency limits
pub const MAX_BLOCKING_DEPENDENCIES_PER_ITEM: usize = 50;
pub const MAX_BLOCKED_DEPENDENCIES_PER_ITEM: usize = 50;
```

**2.2 Message Validator** (`pm-ws/src/handlers/message_validator.rs`)

Add validation methods:

```rust
/// Validate time entry description (optional, max 1000 chars)
#[track_caller]
pub fn validate_time_entry_description(description: Option<&str>) -> WsErrorResult<()> {
    if let Some(desc) = description {
        Self::validate_string(desc, "description", 0, MAX_TIME_ENTRY_DESCRIPTION_LENGTH)?;
    }
    Ok(())
}

/// Validate time entry timestamps for manual entry
#[track_caller]
pub fn validate_time_entry_timestamps(started_at: i64, ended_at: i64) -> WsErrorResult<()> {
    let now = Utc::now().timestamp();

    // Cannot be in future (with tolerance)
    if started_at > now + MAX_FUTURE_TIMESTAMP_TOLERANCE_SECONDS {
        return Err(WsError::ValidationError {
            message: "started_at cannot be in the future".into(),
            field: Some("started_at".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }
    if ended_at > now + MAX_FUTURE_TIMESTAMP_TOLERANCE_SECONDS {
        return Err(WsError::ValidationError {
            message: "ended_at cannot be in the future".into(),
            field: Some("ended_at".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Start must be before end
    if started_at >= ended_at {
        return Err(WsError::ValidationError {
            message: "started_at must be before ended_at".into(),
            field: Some("started_at".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Max duration check
    let duration = ended_at - started_at;
    if duration > MAX_TIME_ENTRY_DURATION_SECONDS {
        return Err(WsError::ValidationError {
            message: format!("Duration cannot exceed {} hours", MAX_TIME_ENTRY_DURATION_SECONDS / 3600),
            field: Some("ended_at".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    Ok(())
}

/// Validate dependency type enum
#[track_caller]
pub fn validate_dependency_type(value: i32) -> WsErrorResult<DependencyType> {
    match value {
        x if x == ProtoDependencyType::Blocks as i32 => Ok(DependencyType::Blocks),
        x if x == ProtoDependencyType::RelatesTo as i32 => Ok(DependencyType::RelatesTo),
        _ => Err(WsError::ValidationError {
            message: "Invalid dependency_type. Must be BLOCKS or RELATES_TO".into(),
            field: Some("dependency_type".into()),
            location: ErrorLocation::from(Location::caller()),
        }),
    }
}
```

**2.3 Response Builders** (`pm-ws/src/handlers/response_builder.rs`)

Add converters and builders:

```rust
// === Converters ===

fn time_entry_to_proto(entry: &TimeEntry) -> ProtoTimeEntry {
    ProtoTimeEntry {
        id: entry.id.to_string(),
        work_item_id: entry.work_item_id.to_string(),
        user_id: entry.user_id.to_string(),
        started_at: entry.started_at.timestamp(),
        ended_at: entry.ended_at.map(|dt| dt.timestamp()),
        duration_seconds: entry.duration_seconds,
        description: entry.description.clone(),
        created_at: entry.created_at.timestamp(),
        updated_at: entry.updated_at.timestamp(),
        deleted_at: entry.deleted_at.map(|dt| dt.timestamp()),
    }
}

fn dependency_to_proto(dep: &Dependency) -> ProtoDependency {
    ProtoDependency {
        id: dep.id.to_string(),
        blocking_item_id: dep.blocking_item_id.to_string(),
        blocked_item_id: dep.blocked_item_id.to_string(),
        dependency_type: match dep.dependency_type {
            DependencyType::Blocks => ProtoDependencyType::Blocks as i32,
            DependencyType::RelatesTo => ProtoDependencyType::RelatesTo as i32,
        },
        created_at: dep.created_at.timestamp(),
        created_by: dep.created_by.to_string(),
        deleted_at: dep.deleted_at.map(|dt| dt.timestamp()),
    }
}

// === Response Builders ===

pub fn build_timer_started_response(
    message_id: &str,
    entry: &TimeEntry,
    stopped_entry: Option<&TimeEntry>,
    user_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::TimerStarted(TimerStarted {
            time_entry: Some(time_entry_to_proto(entry)),
            user_id: user_id.to_string(),
            stopped_entry: stopped_entry.map(time_entry_to_proto),
        })),
    }
}

pub fn build_timer_stopped_response(
    message_id: &str,
    entry: &TimeEntry,
    user_id: Uuid,
) -> WebSocketMessage;

pub fn build_time_entry_created_response(...) -> WebSocketMessage;
pub fn build_time_entry_updated_response(...) -> WebSocketMessage;

pub fn build_time_entry_deleted_response(
    message_id: &str,
    time_entry_id: Uuid,
    work_item_id: Uuid,
    user_id: Uuid,
) -> WebSocketMessage;

pub fn build_time_entries_list_response(
    message_id: &str,
    entries: &[TimeEntry],
    total_count: i32,
) -> WebSocketMessage;

pub fn build_running_timer_response(
    message_id: &str,
    entry: Option<&TimeEntry>,
) -> WebSocketMessage;

pub fn build_dependency_created_response(
    message_id: &str,
    dependency: &Dependency,
    user_id: Uuid,
) -> WebSocketMessage;

pub fn build_dependency_deleted_response(
    message_id: &str,
    dependency_id: Uuid,
    blocking_item_id: Uuid,
    blocked_item_id: Uuid,
    user_id: Uuid,
) -> WebSocketMessage;

pub fn build_dependencies_list_response(
    message_id: &str,
    blocking: &[Dependency],
    blocked: &[Dependency],
) -> WebSocketMessage;
```

**2.4 Repository Additions**

`pm-db/src/repositories/time_entry_repository.rs` - Add pagination:

```rust
/// Find time entries for a work item with pagination
/// Always filters deleted_at IS NULL
pub async fn find_by_work_item_paginated(
    &self,
    work_item_id: Uuid,
    limit: i32,
    offset: i32,
) -> DbErrorResult<(Vec<TimeEntry>, i32)> {
    let work_item_str = work_item_id.to_string();

    // Get total count
    let count_row = sqlx::query!(
        r#"SELECT COUNT(*) as count FROM pm_time_entries
           WHERE work_item_id = ? AND deleted_at IS NULL"#,
        work_item_str
    )
    .fetch_one(&self.pool)
    .await?;
    let total_count = count_row.count as i32;

    // Get paginated entries
    let rows = sqlx::query!(
        r#"SELECT id, work_item_id, user_id, started_at, ended_at,
                  duration_seconds, description, created_at, updated_at, deleted_at
           FROM pm_time_entries
           WHERE work_item_id = ? AND deleted_at IS NULL
           ORDER BY started_at DESC
           LIMIT ? OFFSET ?"#,
        work_item_str,
        limit,
        offset
    )
    .fetch_all(&self.pool)
    .await?;

    let entries = rows.into_iter().map(|r| /* convert */).collect();
    Ok((entries, total_count))
}
```

`pm-db/src/repositories/dependency_repository.rs` - Add methods:

```rust
/// Find existing dependency by pair (for duplicate check)
/// Always filters deleted_at IS NULL
pub async fn find_by_pair(
    &self,
    blocking_item_id: Uuid,
    blocked_item_id: Uuid,
) -> DbErrorResult<Option<Dependency>> {
    let blocking_str = blocking_item_id.to_string();
    let blocked_str = blocked_item_id.to_string();

    let row = sqlx::query!(
        r#"SELECT id, blocking_item_id, blocked_item_id, dependency_type,
                  created_at, created_by, deleted_at
           FROM pm_dependencies
           WHERE blocking_item_id = ? AND blocked_item_id = ? AND deleted_at IS NULL"#,
        blocking_str,
        blocked_str
    )
    .fetch_optional(&self.pool)
    .await?;

    Ok(row.map(|r| /* convert */))
}

/// Count dependencies for limit enforcement
pub async fn count_blocking(&self, blocked_item_id: Uuid) -> DbErrorResult<usize> {
    let id_str = blocked_item_id.to_string();
    let row = sqlx::query!(
        r#"SELECT COUNT(*) as count FROM pm_dependencies
           WHERE blocked_item_id = ? AND deleted_at IS NULL"#,
        id_str
    )
    .fetch_one(&self.pool)
    .await?;
    Ok(row.count as usize)
}

pub async fn count_blocked(&self, blocking_item_id: Uuid) -> DbErrorResult<usize> {
    let id_str = blocking_item_id.to_string();
    let row = sqlx::query!(
        r#"SELECT COUNT(*) as count FROM pm_dependencies
           WHERE blocking_item_id = ? AND deleted_at IS NULL"#,
        id_str
    )
    .fetch_one(&self.pool)
    .await?;
    Ok(row.count as usize)
}
```

---

### Phase 3: Backend Handlers (Required by Dispatcher)

**3.1 Time Entry Handler** (`pm-ws/src/handlers/time_entry.rs`)

```rust
//! Time entry handlers with atomic timer operations.
//!
//! Business rules:
//! - Only ONE running timer per user at any time
//! - StartTimer is ATOMIC: check-stop-create in single transaction
//! - Owner-only for edit/delete operations
//! - All timestamps in UTC

use crate::{
    HandlerContext, MessageValidator, WsError, WsErrorResult,
    build_timer_started_response, build_timer_stopped_response,
    build_time_entry_created_response, build_time_entry_updated_response,
    build_time_entry_deleted_response, build_time_entries_list_response,
    build_running_timer_response,
    check_idempotency, store_idempotency, check_permission,
    db_read, db_write, parse_uuid, sanitize_string,
};
use pm_config::{DEFAULT_TIME_ENTRIES_LIMIT, MAX_TIME_ENTRIES_LIMIT};
use pm_core::{ActivityLog, Permission, TimeEntry};
use pm_db::{ActivityLogRepository, TimeEntryRepository, WorkItemRepository};
use pm_proto::{
    StartTimerRequest, StopTimerRequest, CreateTimeEntryRequest,
    UpdateTimeEntryRequest, DeleteTimeEntryRequest, GetTimeEntriesRequest,
    GetRunningTimerRequest, WebSocketMessage,
};

/// Start a timer on a work item.
/// ATOMIC: Stops any existing running timer in the same transaction.
pub async fn handle_start_timer(
    req: StartTimerRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} StartTimer starting", ctx.log_prefix());

    // 1. Validate description if provided
    MessageValidator::validate_time_entry_description(req.description.as_deref())?;

    // 2. Parse work_item_id
    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // 3. Check idempotency
    let cached = db_read(&ctx, "check_idempotency", || async {
        check_idempotency(&ctx.pool, &ctx.message_id).await
    }).await?;
    if let Some(cached_response) = cached {
        info!("{} Returning cached idempotent response for StartTimer", ctx.log_prefix());
        return decode_cached_response(&cached_response);
    }

    // 4. Verify work item exists and get project_id
    let work_item = db_read(&ctx, "find_work_item", || async {
        WorkItemRepository::new(ctx.pool.clone()).find_by_id(work_item_id).await
    }).await?.ok_or_else(|| WsError::NotFound {
        entity: "WorkItem".into(),
        id: work_item_id.to_string(),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 5. Check Edit permission on project
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::Edit).await
    }).await?;

    // 6. ATOMIC TRANSACTION: Stop existing timer + Create new timer
    let (new_entry, stopped_entry) = db_write(&ctx, "start_timer_atomic", || async {
        let repo = TimeEntryRepository::new(ctx.pool.clone());

        // Find and stop any running timer for this user
        let running_timers = repo.find_running(ctx.user_id).await?;
        let stopped = if let Some(mut running) = running_timers.into_iter().next() {
            running.stop(); // Sets ended_at and calculates duration
            repo.update(&running).await?;

            // Activity log for stopped timer
            let activity = ActivityLog::updated(
                "time_entry",
                running.id,
                ctx.user_id,
                &[("ended_at", "auto-stopped by new timer")],
            );
            ActivityLogRepository::new(ctx.pool.clone()).create(&activity).await?;

            Some(running)
        } else {
            None
        };

        // Create new timer
        let new_timer = TimeEntry::new(
            work_item_id,
            ctx.user_id,
            req.description.as_ref().map(|d| sanitize_string(d)),
        );
        repo.create(&new_timer).await?;

        // Activity log for new timer
        let activity = ActivityLog::created("time_entry", new_timer.id, ctx.user_id);
        ActivityLogRepository::new(ctx.pool.clone()).create(&activity).await?;

        Ok::<_, WsError>((new_timer, stopped))
    }).await?;

    // 7. Build response
    let response = build_timer_started_response(
        &ctx.message_id,
        &new_entry,
        stopped_entry.as_ref(),
        ctx.user_id,
    );

    // 8. Store idempotency (non-fatal if fails)
    store_idempotency_non_fatal(&ctx, "start_timer", &response).await;

    info!(
        "{} Started timer {} on work item {}, stopped previous: {}",
        ctx.log_prefix(),
        new_entry.id,
        work_item_id,
        stopped_entry.is_some()
    );

    Ok(response)
}

/// Stop the currently running timer.
pub async fn handle_stop_timer(
    req: StopTimerRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} StopTimer starting", ctx.log_prefix());

    // 1. Parse time_entry_id
    let time_entry_id = parse_uuid(&req.time_entry_id, "time_entry_id")?;

    // 2. Check idempotency
    // ... standard pattern ...

    // 3. Find the time entry
    let repo = TimeEntryRepository::new(ctx.pool.clone());
    let mut entry = db_read(&ctx, "find_time_entry", || async {
        repo.find_by_id(time_entry_id).await
    }).await?.ok_or_else(|| WsError::NotFound {
        entity: "TimeEntry".into(),
        id: time_entry_id.to_string(),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 4. OWNER-ONLY: Verify ownership
    if entry.user_id != ctx.user_id {
        return Err(WsError::Unauthorized {
            message: "Cannot stop another user's timer".into(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Check if already stopped
    if !entry.is_running() {
        return Err(WsError::ValidationError {
            message: "Timer is not running".into(),
            field: Some("time_entry_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 6. Stop the timer
    entry.stop(); // Sets ended_at = now, calculates duration_seconds

    db_write(&ctx, "stop_timer", || async {
        repo.update(&entry).await?;

        let activity = ActivityLog::updated(
            "time_entry",
            entry.id,
            ctx.user_id,
            &[("ended_at", &entry.ended_at.unwrap().to_rfc3339())],
        );
        ActivityLogRepository::new(ctx.pool.clone()).create(&activity).await?;

        Ok::<_, WsError>(())
    }).await?;

    // 7. Build response
    let response = build_timer_stopped_response(&ctx.message_id, &entry, ctx.user_id);

    // 8. Store idempotency
    store_idempotency_non_fatal(&ctx, "stop_timer", &response).await;

    info!("{} Stopped timer {}, duration: {}s", ctx.log_prefix(), entry.id, entry.duration_seconds.unwrap_or(0));

    Ok(response)
}

/// Create a manual time entry (already completed).
pub async fn handle_create_time_entry(
    req: CreateTimeEntryRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    // 1. Validate timestamps
    MessageValidator::validate_time_entry_timestamps(req.started_at, req.ended_at)?;
    MessageValidator::validate_time_entry_description(req.description.as_deref())?;

    // 2. Parse work_item_id, check idempotency
    // ... standard pattern ...

    // 3. Verify work item exists, check Edit permission
    // ... standard pattern ...

    // 4. Create entry with explicit timestamps
    let started_at = DateTime::from_timestamp(req.started_at, 0)
        .ok_or_else(|| WsError::ValidationError {
            message: "Invalid started_at timestamp".into(),
            field: Some("started_at".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;
    let ended_at = DateTime::from_timestamp(req.ended_at, 0)
        .ok_or_else(|| WsError::ValidationError {
            message: "Invalid ended_at timestamp".into(),
            field: Some("ended_at".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let duration_seconds = (req.ended_at - req.started_at) as i32;

    let entry = TimeEntry {
        id: Uuid::new_v4(),
        work_item_id,
        user_id: ctx.user_id,
        started_at,
        ended_at: Some(ended_at),
        duration_seconds: Some(duration_seconds),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
    };

    // 5. Save with activity log
    db_write(&ctx, "create_time_entry", || async {
        TimeEntryRepository::new(ctx.pool.clone()).create(&entry).await?;
        let activity = ActivityLog::created("time_entry", entry.id, ctx.user_id);
        ActivityLogRepository::new(ctx.pool.clone()).create(&activity).await?;
        Ok::<_, WsError>(())
    }).await?;

    // 6. Build response, store idempotency
    // ... standard pattern ...
}

/// Update a time entry (owner-only).
pub async fn handle_update_time_entry(
    req: UpdateTimeEntryRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    // 1. Find entry, verify ownership
    // 2. Validate any changed timestamps
    // 3. Track field changes with FieldChangeBuilder
    // 4. Update and log
    // ... follows update pattern from sprint.rs ...
}

/// Delete a time entry (owner-only, soft delete).
pub async fn handle_delete_time_entry(
    req: DeleteTimeEntryRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    // 1. Find entry, verify ownership
    // 2. Soft delete
    // 3. Activity log
    // ... follows delete pattern ...
}

/// Get time entries for a work item (paginated).
pub async fn handle_get_time_entries(
    req: GetTimeEntriesRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // Validate and apply pagination limits
    let limit = req.limit
        .map(|l| l.clamp(1, MAX_TIME_ENTRIES_LIMIT))
        .unwrap_or(DEFAULT_TIME_ENTRIES_LIMIT);
    let offset = req.offset.unwrap_or(0).max(0);

    // Verify work item exists, check View permission
    let work_item = /* ... */;
    check_permission(&ctx, work_item.project_id, Permission::View).await?;

    // Get paginated entries
    let (entries, total_count) = db_read(&ctx, "get_time_entries", || async {
        TimeEntryRepository::new(ctx.pool.clone())
            .find_by_work_item_paginated(work_item_id, limit, offset)
            .await
    }).await?;

    Ok(build_time_entries_list_response(&ctx.message_id, &entries, total_count))
}

/// Get the current user's running timer (if any).
pub async fn handle_get_running_timer(
    _req: GetRunningTimerRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    let repo = TimeEntryRepository::new(ctx.pool.clone());
    let running = db_read(&ctx, "find_running_timer", || async {
        repo.find_running(ctx.user_id).await
    }).await?;

    let entry = running.into_iter().next();
    Ok(build_running_timer_response(&ctx.message_id, entry.as_ref()))
}
```

**3.2 Dependency Handler** (`pm-ws/src/handlers/dependency.rs`)

```rust
//! Dependency handlers with cycle detection.
//!
//! Business rules:
//! - No self-reference (A cannot block A)
//! - No duplicates (same pair cannot exist twice)
//! - No cycles for Blocks type (detected via BFS)
//! - Same-project only
//! - Max 50 blocking + 50 blocked per item

use crate::{
    HandlerContext, MessageValidator, WsError, WsErrorResult,
    build_dependency_created_response, build_dependency_deleted_response,
    build_dependencies_list_response,
    check_idempotency, store_idempotency, check_permission,
    db_read, db_write, parse_uuid,
};
use pm_config::{MAX_BLOCKING_DEPENDENCIES_PER_ITEM, MAX_BLOCKED_DEPENDENCIES_PER_ITEM};
use pm_core::{ActivityLog, Dependency, DependencyType, Permission};
use pm_db::{ActivityLogRepository, DependencyRepository, WorkItemRepository};
use pm_proto::{
    CreateDependencyRequest, DeleteDependencyRequest, GetDependenciesRequest,
    WebSocketMessage,
};
use std::collections::{HashSet, VecDeque};

/// Detect circular dependencies using BFS.
/// Returns Ok(()) if no cycle, Err with cycle path if cycle detected.
async fn detect_circular_dependency(
    repo: &DependencyRepository,
    blocking_id: Uuid,
    blocked_id: Uuid,
) -> WsErrorResult<()> {
    // If we add blocking_id -> blocked_id,
    // check if blocked_id can eventually reach blocking_id

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut parent_map: HashMap<Uuid, Uuid> = HashMap::new();

    queue.push_back(blocked_id);
    visited.insert(blocked_id);

    while let Some(current) = queue.pop_front() {
        // Get all items that `current` blocks
        let blocked_by_current = repo.find_blocked(current).await
            .map_err(|e| WsError::DatabaseError {
                message: e.to_string(),
                location: ErrorLocation::from(Location::caller()),
            })?;

        for dep in blocked_by_current {
            if dep.dependency_type != DependencyType::Blocks {
                continue; // RelatesTo doesn't create cycles
            }

            if dep.blocked_item_id == blocking_id {
                // Found a cycle! Build the path for error message
                let mut path = vec![blocking_id, blocked_id];
                let mut node = current;
                while let Some(&parent) = parent_map.get(&node) {
                    path.push(node);
                    node = parent;
                }
                path.push(dep.blocked_item_id);

                let path_str = path.iter()
                    .map(|id| id.to_string()[..8].to_string()) // Short IDs
                    .collect::<Vec<_>>()
                    .join(" → ");

                return Err(WsError::ValidationError {
                    message: format!(
                        "Circular dependency detected: {}. This would create a cycle.",
                        path_str
                    ),
                    field: Some("blocking_item_id".into()),
                    location: ErrorLocation::from(Location::caller()),
                });
            }

            if !visited.contains(&dep.blocked_item_id) {
                visited.insert(dep.blocked_item_id);
                parent_map.insert(dep.blocked_item_id, current);
                queue.push_back(dep.blocked_item_id);
            }
        }
    }

    Ok(())
}

/// Create a dependency between two work items.
pub async fn handle_create_dependency(
    req: CreateDependencyRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} CreateDependency starting", ctx.log_prefix());

    // 1. Parse IDs
    let blocking_id = parse_uuid(&req.blocking_item_id, "blocking_item_id")?;
    let blocked_id = parse_uuid(&req.blocked_item_id, "blocked_item_id")?;

    // 2. Self-reference check
    if blocking_id == blocked_id {
        return Err(WsError::ValidationError {
            message: "Work item cannot block itself".into(),
            field: Some("blocked_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 3. Parse and validate dependency type
    let dep_type = MessageValidator::validate_dependency_type(req.dependency_type)?;

    // 4. Check idempotency
    // ... standard pattern ...

    // 5. Verify both work items exist
    let work_item_repo = WorkItemRepository::new(ctx.pool.clone());

    let blocking_item = db_read(&ctx, "find_blocking_item", || async {
        work_item_repo.find_by_id(blocking_id).await
    }).await?.ok_or_else(|| WsError::NotFound {
        entity: "WorkItem (blocking)".into(),
        id: blocking_id.to_string(),
        location: ErrorLocation::from(Location::caller()),
    })?;

    let blocked_item = db_read(&ctx, "find_blocked_item", || async {
        work_item_repo.find_by_id(blocked_id).await
    }).await?.ok_or_else(|| WsError::NotFound {
        entity: "WorkItem (blocked)".into(),
        id: blocked_id.to_string(),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 6. SAME-PROJECT CHECK
    if blocking_item.project_id != blocked_item.project_id {
        return Err(WsError::ValidationError {
            message: "Dependencies can only be created between items in the same project".into(),
            field: Some("blocking_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 7. Check Edit permission on the project
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, blocking_item.project_id, Permission::Edit).await
    }).await?;

    let dep_repo = DependencyRepository::new(ctx.pool.clone());

    // 8. Check for duplicate
    let existing = db_read(&ctx, "check_duplicate", || async {
        dep_repo.find_by_pair(blocking_id, blocked_id).await
    }).await?;

    if existing.is_some() {
        return Err(WsError::ConflictError {
            message: "Dependency already exists between these items".into(),
            current_version: 0,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 9. Check dependency limits
    let blocking_count = db_read(&ctx, "count_blocking", || async {
        dep_repo.count_blocking(blocked_id).await
    }).await?;

    if blocking_count >= MAX_BLOCKING_DEPENDENCIES_PER_ITEM {
        return Err(WsError::ValidationError {
            message: format!(
                "Item already has {} blocking dependencies (max {})",
                blocking_count, MAX_BLOCKING_DEPENDENCIES_PER_ITEM
            ),
            field: Some("blocked_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    let blocked_count = db_read(&ctx, "count_blocked", || async {
        dep_repo.count_blocked(blocking_id).await
    }).await?;

    if blocked_count >= MAX_BLOCKED_DEPENDENCIES_PER_ITEM {
        return Err(WsError::ValidationError {
            message: format!(
                "Item already blocks {} items (max {})",
                blocked_count, MAX_BLOCKED_DEPENDENCIES_PER_ITEM
            ),
            field: Some("blocking_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 10. CIRCULAR DEPENDENCY CHECK (only for Blocks type)
    if dep_type == DependencyType::Blocks {
        detect_circular_dependency(&dep_repo, blocking_id, blocked_id).await?;
    }

    // 11. Create dependency
    let dependency = Dependency::new(blocking_id, blocked_id, dep_type, ctx.user_id);

    db_write(&ctx, "create_dependency", || async {
        dep_repo.create(&dependency).await?;

        let activity = ActivityLog::created("dependency", dependency.id, ctx.user_id);
        ActivityLogRepository::new(ctx.pool.clone()).create(&activity).await?;

        Ok::<_, WsError>(())
    }).await?;

    // 12. Build response
    let response = build_dependency_created_response(
        &ctx.message_id,
        &dependency,
        ctx.user_id,
    );

    // 13. Store idempotency
    store_idempotency_non_fatal(&ctx, "create_dependency", &response).await;

    info!(
        "{} Created dependency: {} {} {}",
        ctx.log_prefix(),
        blocking_id,
        dep_type.as_str(),
        blocked_id
    );

    Ok(response)
}

/// Delete a dependency.
pub async fn handle_delete_dependency(
    req: DeleteDependencyRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    let dependency_id = parse_uuid(&req.dependency_id, "dependency_id")?;

    // Find dependency
    let dep_repo = DependencyRepository::new(ctx.pool.clone());
    let dependency = db_read(&ctx, "find_dependency", || async {
        dep_repo.find_by_id(dependency_id).await
    }).await?.ok_or_else(|| WsError::NotFound {
        entity: "Dependency".into(),
        id: dependency_id.to_string(),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Get work item to check project permission
    let work_item = WorkItemRepository::new(ctx.pool.clone())
        .find_by_id(dependency.blocking_item_id)
        .await?
        .ok_or_else(|| WsError::NotFound {
            entity: "WorkItem".into(),
            id: dependency.blocking_item_id.to_string(),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // Check Edit permission
    check_permission(&ctx, work_item.project_id, Permission::Edit).await?;

    // Soft delete
    db_write(&ctx, "delete_dependency", || async {
        dep_repo.delete(dependency_id, Utc::now().timestamp()).await?;

        let activity = ActivityLog::deleted("dependency", dependency_id, ctx.user_id);
        ActivityLogRepository::new(ctx.pool.clone()).create(&activity).await?;

        Ok::<_, WsError>(())
    }).await?;

    let response = build_dependency_deleted_response(
        &ctx.message_id,
        dependency_id,
        dependency.blocking_item_id,
        dependency.blocked_item_id,
        ctx.user_id,
    );

    info!("{} Deleted dependency {}", ctx.log_prefix(), dependency_id);

    Ok(response)
}

/// Get dependencies for a work item.
pub async fn handle_get_dependencies(
    req: GetDependenciesRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // Verify work item exists, check View permission
    let work_item = WorkItemRepository::new(ctx.pool.clone())
        .find_by_id(work_item_id)
        .await?
        .ok_or_else(|| WsError::NotFound { /* ... */ })?;

    check_permission(&ctx, work_item.project_id, Permission::View).await?;

    let dep_repo = DependencyRepository::new(ctx.pool.clone());

    // Get both directions
    let blocking = db_read(&ctx, "find_blocking", || async {
        dep_repo.find_blocking(work_item_id).await
    }).await?;

    let blocked = db_read(&ctx, "find_blocked", || async {
        dep_repo.find_blocked(work_item_id).await
    }).await?;

    Ok(build_dependencies_list_response(&ctx.message_id, &blocking, &blocked))
}
```

**3.3 Dispatcher Wiring** (`pm-ws/src/handlers/dispatcher.rs`)

Add to `dispatch_inner` match:
```rust
// Time Entry handlers
Some(Payload::StartTimerRequest(req)) => time_entry::handle_start_timer(req, ctx).await,
Some(Payload::StopTimerRequest(req)) => time_entry::handle_stop_timer(req, ctx).await,
Some(Payload::CreateTimeEntryRequest(req)) => time_entry::handle_create_time_entry(req, ctx).await,
Some(Payload::UpdateTimeEntryRequest(req)) => time_entry::handle_update_time_entry(req, ctx).await,
Some(Payload::DeleteTimeEntryRequest(req)) => time_entry::handle_delete_time_entry(req, ctx).await,
Some(Payload::GetTimeEntriesRequest(req)) => time_entry::handle_get_time_entries(req, ctx).await,
Some(Payload::GetRunningTimerRequest(req)) => time_entry::handle_get_running_timer(req, ctx).await,

// Dependency handlers
Some(Payload::CreateDependencyRequest(req)) => dependency::handle_create_dependency(req, ctx).await,
Some(Payload::DeleteDependencyRequest(req)) => dependency::handle_delete_dependency(req, ctx).await,
Some(Payload::GetDependenciesRequest(req)) => dependency::handle_get_dependencies(req, ctx).await,
```

Add to `payload_to_handler_name`:
```rust
Some(Payload::StartTimerRequest(_)) => "StartTimer",
Some(Payload::StopTimerRequest(_)) => "StopTimer",
Some(Payload::CreateTimeEntryRequest(_)) => "CreateTimeEntry",
Some(Payload::UpdateTimeEntryRequest(_)) => "UpdateTimeEntry",
Some(Payload::DeleteTimeEntryRequest(_)) => "DeleteTimeEntry",
Some(Payload::GetTimeEntriesRequest(_)) => "GetTimeEntries",
Some(Payload::GetRunningTimerRequest(_)) => "GetRunningTimer",
Some(Payload::CreateDependencyRequest(_)) => "CreateDependency",
Some(Payload::DeleteDependencyRequest(_)) => "DeleteDependency",
Some(Payload::GetDependenciesRequest(_)) => "GetDependencies",
```

**3.4 Module Export** (`pm-ws/src/handlers/mod.rs`)

```rust
pub(crate) mod time_entry;
pub(crate) mod dependency;
```

---

### Phase 4: Frontend Models (Required by Stores)

**4.1 Domain Models**

`frontend/ProjectManagement.Core/Models/TimeEntry.cs`:
```csharp
namespace ProjectManagement.Core.Models;

/// <summary>
/// A time tracking entry on a work item.
/// Can be a running timer (EndedAt = null) or completed entry.
/// </summary>
public sealed record TimeEntry
{
    public Guid Id { get; init; }
    public Guid WorkItemId { get; init; }
    public Guid UserId { get; init; }

    public DateTime StartedAt { get; init; }
    public DateTime? EndedAt { get; init; }
    public int? DurationSeconds { get; init; }

    public string? Description { get; init; }

    public DateTime CreatedAt { get; init; }
    public DateTime UpdatedAt { get; init; }
    public DateTime? DeletedAt { get; init; }

    /// <summary>True if timer is still running (EndedAt is null).</summary>
    public bool IsRunning => EndedAt == null && DeletedAt == null;

    /// <summary>
    /// Elapsed time. If running, calculates from StartedAt to now.
    /// If stopped, uses DurationSeconds or calculates from EndedAt.
    /// </summary>
    public TimeSpan Elapsed => DurationSeconds.HasValue
        ? TimeSpan.FromSeconds(DurationSeconds.Value)
        : EndedAt.HasValue
            ? EndedAt.Value - StartedAt
            : DateTime.UtcNow - StartedAt;
}
```

`frontend/ProjectManagement.Core/Models/Dependency.cs`:
```csharp
namespace ProjectManagement.Core.Models;

public sealed record Dependency
{
    public Guid Id { get; init; }
    public Guid BlockingItemId { get; init; }
    public Guid BlockedItemId { get; init; }
    public DependencyType Type { get; init; }

    public DateTime CreatedAt { get; init; }
    public Guid CreatedBy { get; init; }
    public DateTime? DeletedAt { get; init; }
}

public enum DependencyType
{
    Blocks = 1,
    RelatesTo = 2
}
```

**4.2 Request DTOs**

`frontend/ProjectManagement.Core/Models/TimeEntryRequests.cs`:
```csharp
namespace ProjectManagement.Core.Models;

public sealed record StartTimerRequest
{
    public Guid WorkItemId { get; init; }
    public string? Description { get; init; }
}

public sealed record CreateTimeEntryRequest
{
    public Guid WorkItemId { get; init; }
    public DateTime StartedAt { get; init; }
    public DateTime EndedAt { get; init; }
    public string? Description { get; init; }
}

public sealed record UpdateTimeEntryRequest
{
    public Guid TimeEntryId { get; init; }
    public DateTime? StartedAt { get; init; }
    public DateTime? EndedAt { get; init; }
    public string? Description { get; init; }
}
```

`frontend/ProjectManagement.Core/Models/DependencyRequests.cs`:
```csharp
namespace ProjectManagement.Core.Models;

public sealed record CreateDependencyRequest
{
    public Guid BlockingItemId { get; init; }
    public Guid BlockedItemId { get; init; }
    public DependencyType Type { get; init; }
}
```

**4.3 Proto Converters** (`ProtoConverter.cs`)

Add to existing file:
```csharp
// === Time Entry ===

public static TimeEntry ToDomain(Proto.TimeEntry proto)
{
    ArgumentNullException.ThrowIfNull(proto);

    return new TimeEntry
    {
        Id = Guid.Parse(proto.Id),
        WorkItemId = Guid.Parse(proto.WorkItemId),
        UserId = Guid.Parse(proto.UserId),
        StartedAt = DateTimeOffset.FromUnixTimeSeconds(proto.StartedAt).UtcDateTime,
        EndedAt = proto.HasEndedAt
            ? DateTimeOffset.FromUnixTimeSeconds(proto.EndedAt).UtcDateTime
            : null,
        DurationSeconds = proto.HasDurationSeconds ? proto.DurationSeconds : null,
        Description = proto.HasDescription ? proto.Description : null,
        CreatedAt = DateTimeOffset.FromUnixTimeSeconds(proto.CreatedAt).UtcDateTime,
        UpdatedAt = DateTimeOffset.FromUnixTimeSeconds(proto.UpdatedAt).UtcDateTime,
        DeletedAt = proto.HasDeletedAt
            ? DateTimeOffset.FromUnixTimeSeconds(proto.DeletedAt).UtcDateTime
            : null,
    };
}

public static Proto.TimeEntry ToProto(TimeEntry entry)
{
    ArgumentNullException.ThrowIfNull(entry);

    var proto = new Proto.TimeEntry
    {
        Id = entry.Id.ToString(),
        WorkItemId = entry.WorkItemId.ToString(),
        UserId = entry.UserId.ToString(),
        StartedAt = new DateTimeOffset(entry.StartedAt).ToUnixTimeSeconds(),
        CreatedAt = new DateTimeOffset(entry.CreatedAt).ToUnixTimeSeconds(),
        UpdatedAt = new DateTimeOffset(entry.UpdatedAt).ToUnixTimeSeconds(),
    };

    if (entry.EndedAt.HasValue)
        proto.EndedAt = new DateTimeOffset(entry.EndedAt.Value).ToUnixTimeSeconds();
    if (entry.DurationSeconds.HasValue)
        proto.DurationSeconds = entry.DurationSeconds.Value;
    if (entry.Description != null)
        proto.Description = entry.Description;
    if (entry.DeletedAt.HasValue)
        proto.DeletedAt = new DateTimeOffset(entry.DeletedAt.Value).ToUnixTimeSeconds();

    return proto;
}

// === Dependency ===

public static Dependency ToDomain(Proto.Dependency proto)
{
    ArgumentNullException.ThrowIfNull(proto);

    return new Dependency
    {
        Id = Guid.Parse(proto.Id),
        BlockingItemId = Guid.Parse(proto.BlockingItemId),
        BlockedItemId = Guid.Parse(proto.BlockedItemId),
        Type = (DependencyType)proto.DependencyType,
        CreatedAt = DateTimeOffset.FromUnixTimeSeconds(proto.CreatedAt).UtcDateTime,
        CreatedBy = Guid.Parse(proto.CreatedBy),
        DeletedAt = proto.HasDeletedAt
            ? DateTimeOffset.FromUnixTimeSeconds(proto.DeletedAt).UtcDateTime
            : null,
    };
}

public static Proto.Dependency ToProto(Dependency dep)
{
    ArgumentNullException.ThrowIfNull(dep);

    var proto = new Proto.Dependency
    {
        Id = dep.Id.ToString(),
        BlockingItemId = dep.BlockingItemId.ToString(),
        BlockedItemId = dep.BlockedItemId.ToString(),
        DependencyType = (Proto.DependencyType)dep.Type,
        CreatedAt = new DateTimeOffset(dep.CreatedAt).ToUnixTimeSeconds(),
        CreatedBy = dep.CreatedBy.ToString(),
    };

    if (dep.DeletedAt.HasValue)
        proto.DeletedAt = new DateTimeOffset(dep.DeletedAt.Value).ToUnixTimeSeconds();

    return proto;
}
```

---

### Phase 5: Frontend WebSocket (Required by Stores)

**5.1 Interface** (`IWebSocketClient.cs`)

Add to existing interface:
```csharp
// === Time Entry Events ===
event Action<TimeEntry, TimeEntry?>? OnTimerStarted;  // (started, optionally stopped)
event Action<TimeEntry>? OnTimerStopped;
event Action<TimeEntry>? OnTimeEntryCreated;
event Action<TimeEntry>? OnTimeEntryUpdated;
event Action<Guid, Guid>? OnTimeEntryDeleted;  // (timeEntryId, workItemId)

// === Time Entry Operations ===
Task<(TimeEntry Started, TimeEntry? Stopped)> StartTimerAsync(
    StartTimerRequest request, CancellationToken ct = default);
Task<TimeEntry> StopTimerAsync(Guid timeEntryId, CancellationToken ct = default);
Task<TimeEntry> CreateTimeEntryAsync(
    CreateTimeEntryRequest request, CancellationToken ct = default);
Task<TimeEntry> UpdateTimeEntryAsync(
    UpdateTimeEntryRequest request, CancellationToken ct = default);
Task DeleteTimeEntryAsync(Guid timeEntryId, CancellationToken ct = default);
Task<(IReadOnlyList<TimeEntry> Entries, int TotalCount)> GetTimeEntriesAsync(
    Guid workItemId, int? limit = null, int? offset = null, CancellationToken ct = default);
Task<TimeEntry?> GetRunningTimerAsync(CancellationToken ct = default);

// === Dependency Events ===
event Action<Dependency>? OnDependencyCreated;
event Action<Guid, Guid, Guid>? OnDependencyDeleted;  // (depId, blockingId, blockedId)

// === Dependency Operations ===
Task<Dependency> CreateDependencyAsync(
    CreateDependencyRequest request, CancellationToken ct = default);
Task DeleteDependencyAsync(Guid dependencyId, CancellationToken ct = default);
Task<(IReadOnlyList<Dependency> Blocking, IReadOnlyList<Dependency> Blocked)> GetDependenciesAsync(
    Guid workItemId, CancellationToken ct = default);
```

**5.2 Implementation** (`WebSocketClient.cs`)

Add event declarations and implementations following existing Sprint/Comment pattern.

---

### Phase 6: Frontend State Management (Required by UI)

**6.1 Store Interfaces**

`frontend/ProjectManagement.Core/Interfaces/ITimeEntryStore.cs`:
```csharp
namespace ProjectManagement.Core.Interfaces;

public interface ITimeEntryStore : IDisposable
{
    /// <summary>Fired when any time entry changes.</summary>
    event Action? OnChanged;

    /// <summary>Get all time entries for a work item (non-deleted only).</summary>
    IReadOnlyList<TimeEntry> GetByWorkItem(Guid workItemId);

    /// <summary>Get the currently running timer for the current user.</summary>
    TimeEntry? GetRunningTimer();

    /// <summary>Check if an entry has a pending server operation.</summary>
    bool IsPending(Guid timeEntryId);

    /// <summary>Start a timer on a work item. Auto-stops any running timer.</summary>
    Task<TimeEntry> StartTimerAsync(StartTimerRequest request, CancellationToken ct = default);

    /// <summary>Stop the specified timer.</summary>
    Task<TimeEntry> StopTimerAsync(Guid timeEntryId, CancellationToken ct = default);

    /// <summary>Create a manual (already completed) time entry.</summary>
    Task<TimeEntry> CreateAsync(CreateTimeEntryRequest request, CancellationToken ct = default);

    /// <summary>Update a time entry (owner-only).</summary>
    Task<TimeEntry> UpdateAsync(UpdateTimeEntryRequest request, CancellationToken ct = default);

    /// <summary>Delete a time entry (owner-only).</summary>
    Task DeleteAsync(Guid timeEntryId, CancellationToken ct = default);

    /// <summary>Refresh entries for a work item from server.</summary>
    Task RefreshAsync(Guid workItemId, CancellationToken ct = default);

    /// <summary>Fetch current running timer from server.</summary>
    Task RefreshRunningTimerAsync(CancellationToken ct = default);
}
```

`frontend/ProjectManagement.Core/Interfaces/IDependencyStore.cs`:
```csharp
namespace ProjectManagement.Core.Interfaces;

public interface IDependencyStore : IDisposable
{
    event Action? OnChanged;

    /// <summary>Get items that are blocking this work item.</summary>
    IReadOnlyList<Dependency> GetBlocking(Guid workItemId);

    /// <summary>Get items that this work item blocks.</summary>
    IReadOnlyList<Dependency> GetBlocked(Guid workItemId);

    /// <summary>Check if work item has any blocking dependencies.</summary>
    bool IsBlocked(Guid workItemId);

    bool IsPending(Guid dependencyId);

    Task<Dependency> CreateAsync(CreateDependencyRequest request, CancellationToken ct = default);
    Task DeleteAsync(Guid dependencyId, CancellationToken ct = default);
    Task RefreshAsync(Guid workItemId, CancellationToken ct = default);
}
```

**6.2 Store Implementations**

`frontend/ProjectManagement.Services/State/TimeEntryStore.cs`:
```csharp
public sealed class TimeEntryStore : ITimeEntryStore
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<TimeEntryStore> _logger;
    private readonly Guid _currentUserId;

    private readonly ConcurrentDictionary<Guid, TimeEntry> _entries = new();
    private readonly ConcurrentDictionary<Guid, TimeEntry> _rollbackState = new();
    private readonly ConcurrentDictionary<Guid, bool> _pendingUpdates = new();
    private TimeEntry? _runningTimer;
    private bool _disposed;

    public event Action? OnChanged;

    public TimeEntryStore(
        IWebSocketClient client,
        IAppState appState,
        ILogger<TimeEntryStore> logger)
    {
        _client = client;
        _currentUserId = appState.CurrentUserId;
        _logger = logger;

        // Subscribe to WebSocket events
        _client.OnTimerStarted += HandleTimerStarted;
        _client.OnTimerStopped += HandleTimerStopped;
        _client.OnTimeEntryCreated += HandleTimeEntryCreated;
        _client.OnTimeEntryUpdated += HandleTimeEntryUpdated;
        _client.OnTimeEntryDeleted += HandleTimeEntryDeleted;
    }

    public IReadOnlyList<TimeEntry> GetByWorkItem(Guid workItemId)
    {
        return _entries.Values
            .Where(e => e.WorkItemId == workItemId && e.DeletedAt == null)
            .OrderByDescending(e => e.StartedAt)
            .ToList();
    }

    public TimeEntry? GetRunningTimer() => _runningTimer;

    public bool IsPending(Guid timeEntryId) => _pendingUpdates.ContainsKey(timeEntryId);

    public async Task<TimeEntry> StartTimerAsync(StartTimerRequest request, CancellationToken ct)
    {
        ThrowIfDisposed();

        // Optimistic: Create temp entry
        var tempId = Guid.NewGuid();
        var optimistic = new TimeEntry
        {
            Id = tempId,
            WorkItemId = request.WorkItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow,
            Description = request.Description,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
        };

        // Track previous running timer for potential rollback
        var previousRunning = _runningTimer;

        _entries[tempId] = optimistic;
        _runningTimer = optimistic;
        _pendingUpdates[tempId] = true;
        NotifyChanged();

        try
        {
            var (started, stopped) = await _client.StartTimerAsync(request, ct);

            // Remove temp, add confirmed
            _entries.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);

            _entries[started.Id] = started;
            _runningTimer = started;

            // Update stopped entry if any
            if (stopped != null && _entries.ContainsKey(stopped.Id))
            {
                _entries[stopped.Id] = stopped;
            }

            NotifyChanged();
            _logger.LogInformation("Started timer {TimerId} on {WorkItemId}", started.Id, request.WorkItemId);
            return started;
        }
        catch (Exception ex)
        {
            // Rollback
            _entries.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            _runningTimer = previousRunning;
            NotifyChanged();

            _logger.LogWarning(ex, "Failed to start timer on {WorkItemId}", request.WorkItemId);
            throw;
        }
    }

    // ... other methods following same optimistic update pattern ...

    private void HandleTimerStarted(TimeEntry started, TimeEntry? stopped)
    {
        // Skip if this is from our own pending operation
        if (_pendingUpdates.ContainsKey(started.Id))
            return;

        _entries[started.Id] = started;

        // Only update running timer if it's the current user's
        if (started.UserId == _currentUserId)
        {
            _runningTimer = started;
        }

        if (stopped != null)
        {
            _entries[stopped.Id] = stopped;
            if (_runningTimer?.Id == stopped.Id)
            {
                _runningTimer = null;
            }
        }

        NotifyChanged();
    }

    // ... other event handlers ...

    private void NotifyChanged() => OnChanged?.Invoke();

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(TimeEntryStore));
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnTimerStarted -= HandleTimerStarted;
        _client.OnTimerStopped -= HandleTimerStopped;
        _client.OnTimeEntryCreated -= HandleTimeEntryCreated;
        _client.OnTimeEntryUpdated -= HandleTimeEntryUpdated;
        _client.OnTimeEntryDeleted -= HandleTimeEntryDeleted;
    }
}
```

`frontend/ProjectManagement.Services/State/DependencyStore.cs`:
Following same pattern with blocking/blocked tracking.

**6.3 Service Registration** (`Program.cs`)

Add to existing service registration:
```csharp
// Time Entry and Dependency stores
builder.Services.AddSingleton<ITimeEntryStore, TimeEntryStore>();
builder.Services.AddSingleton<IDependencyStore, DependencyStore>();
```

---

### Phase 7: Frontend UI Components

**7.1 CSS Files**

`frontend/ProjectManagement.Components/TimeTracking/time-tracking.css`:
```css
.timer-widget {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm);
    border-radius: var(--radius-md);
    background: var(--surface-secondary);
}

.timer-widget.running {
    background: var(--success-surface);
    animation: pulse 2s infinite;
}

@keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.8; }
}

.timer-display {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
}

.timer-display .elapsed {
    font-family: var(--font-mono);
    font-size: var(--text-lg);
    font-weight: 600;
}

.time-entry-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
}

.time-entry-list .entry {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm);
    border-radius: var(--radius-sm);
    background: var(--surface-primary);
}

.time-entry-list .entry.pending {
    opacity: 0.6;
}

.time-entry-list .entry-time {
    text-align: right;
    min-width: 100px;
}

.time-entry-list .total {
    padding-top: var(--spacing-sm);
    border-top: 1px solid var(--border-color);
    text-align: right;
}
```

`frontend/ProjectManagement.Components/Dependencies/dependencies.css`:
```css
.dependency-manager {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
}

.dependency-manager .section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
}

.dependency-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-xs) var(--spacing-sm);
    border-radius: var(--radius-sm);
    background: var(--surface-secondary);
}

.dependency-item.pending {
    opacity: 0.6;
}

.blocked-indicator {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    background: var(--warning-surface);
    color: var(--warning-text);
    font-size: var(--text-xs);
}

.blocked-indicator .rz-icon {
    font-size: 12px;
}
```

**7.2 Timer Components**

`frontend/ProjectManagement.Components/TimeTracking/TimerWidget.razor`:
```razor
@using ProjectManagement.Core.Interfaces
@using ProjectManagement.Core.Models
@inject ITimeEntryStore TimeEntryStore
@implements IDisposable

<div class="timer-widget @(IsRunningOnThis ? "running" : "")">
    @if (IsRunningOnThis)
    {
        <div class="timer-display">
            <RadzenIcon Icon="timer" />
            <span class="elapsed">@FormatElapsed()</span>
        </div>
        @if (!string.IsNullOrEmpty(RunningTimer?.Description))
        {
            <RadzenText TextStyle="TextStyle.Caption" class="description">
                @RunningTimer.Description
            </RadzenText>
        }
        <RadzenButton Icon="stop"
                      ButtonStyle="ButtonStyle.Danger"
                      Size="ButtonSize.Small"
                      Click="HandleStopTimer"
                      IsBusy="@_isBusy"
                      title="Stop timer" />
    }
    else if (HasRunningTimerElsewhere)
    {
        <RadzenButton Icon="swap_horiz"
                      Text="Switch here"
                      ButtonStyle="ButtonStyle.Warning"
                      Size="ButtonSize.Small"
                      Click="HandleStartTimer"
                      IsBusy="@_isBusy"
                      title="Stop current timer and start here" />
    }
    else
    {
        <RadzenButton Icon="play_arrow"
                      Text="Start Timer"
                      ButtonStyle="ButtonStyle.Primary"
                      Size="ButtonSize.Small"
                      Click="HandleStartTimer"
                      IsBusy="@_isBusy" />
    }
</div>

@code {
    [Parameter, EditorRequired] public Guid WorkItemId { get; set; }

    private TimeEntry? RunningTimer => TimeEntryStore.GetRunningTimer();
    private bool IsRunningOnThis => RunningTimer?.WorkItemId == WorkItemId;
    private bool HasRunningTimerElsewhere => RunningTimer != null && !IsRunningOnThis;

    private bool _isBusy;
    private Timer? _refreshTimer;

    protected override void OnInitialized()
    {
        TimeEntryStore.OnChanged += HandleStoreChanged;
        // Refresh display every second when any timer is running
        _refreshTimer = new Timer(_ =>
        {
            if (RunningTimer != null)
                InvokeAsync(StateHasChanged);
        }, null, 0, 1000);
    }

    private async Task HandleStartTimer()
    {
        _isBusy = true;
        try
        {
            await TimeEntryStore.StartTimerAsync(new StartTimerRequest
            {
                WorkItemId = WorkItemId,
                Description = null
            });
        }
        catch (Exception ex)
        {
            // Error handling - show toast
        }
        finally
        {
            _isBusy = false;
        }
    }

    private async Task HandleStopTimer()
    {
        if (RunningTimer == null) return;

        _isBusy = true;
        try
        {
            await TimeEntryStore.StopTimerAsync(RunningTimer.Id);
        }
        catch (Exception ex)
        {
            // Error handling
        }
        finally
        {
            _isBusy = false;
        }
    }

    private string FormatElapsed()
    {
        if (RunningTimer == null) return "00:00";
        var elapsed = RunningTimer.Elapsed;
        return elapsed.TotalHours >= 1
            ? $"{(int)elapsed.TotalHours}:{elapsed.Minutes:D2}:{elapsed.Seconds:D2}"
            : $"{elapsed.Minutes:D2}:{elapsed.Seconds:D2}";
    }

    private void HandleStoreChanged() => InvokeAsync(StateHasChanged);

    public void Dispose()
    {
        TimeEntryStore.OnChanged -= HandleStoreChanged;
        _refreshTimer?.Dispose();
    }
}
```

`frontend/ProjectManagement.Components/TimeTracking/TimeEntryList.razor`:
List component with pagination, edit/delete actions.

`frontend/ProjectManagement.Components/TimeTracking/TimeEntryDialog.razor`:
Dialog for manual time entry creation/editing with validation.

**7.3 Dependency Components**

`frontend/ProjectManagement.Components/Dependencies/DependencyManager.razor`:
Shows blocking/blocked lists with add/remove actions.

`frontend/ProjectManagement.Components/Dependencies/BlockedIndicator.razor`:
```razor
@using ProjectManagement.Core.Interfaces
@inject IDependencyStore DependencyStore
@implements IDisposable

@if (IsBlocked)
{
    <div class="blocked-indicator" title="@BlockedByText">
        <RadzenIcon Icon="block" />
        <span>Blocked</span>
    </div>
}

@code {
    [Parameter, EditorRequired] public Guid WorkItemId { get; set; }

    private bool IsBlocked => DependencyStore.IsBlocked(WorkItemId);
    private int BlockedByCount => DependencyStore.GetBlocking(WorkItemId).Count;
    private string BlockedByText => $"Blocked by {BlockedByCount} item(s)";

    protected override void OnInitialized()
    {
        DependencyStore.OnChanged += HandleChanged;
    }

    private void HandleChanged() => InvokeAsync(StateHasChanged);

    public void Dispose()
    {
        DependencyStore.OnChanged -= HandleChanged;
    }
}
```

`frontend/ProjectManagement.Components/Dependencies/AddDependencyDialog.razor`:
Dialog with work item search for adding dependencies.

---

### Phase 8: Tests

**8.1 Backend Tests**

`backend/crates/pm-ws/tests/time_entry_handler_tests.rs`:
```rust
#[tokio::test]
async fn test_start_timer_creates_running_entry() { ... }

#[tokio::test]
async fn test_start_timer_stops_previous_timer_atomically() { ... }

#[tokio::test]
async fn test_start_timer_idempotent() { ... }

#[tokio::test]
async fn test_stop_timer_calculates_duration() { ... }

#[tokio::test]
async fn test_stop_timer_owner_only() { ... }

#[tokio::test]
async fn test_create_manual_entry_validates_timestamps() { ... }

#[tokio::test]
async fn test_create_manual_entry_rejects_future_dates() { ... }

#[tokio::test]
async fn test_create_manual_entry_rejects_excessive_duration() { ... }

#[tokio::test]
async fn test_get_time_entries_pagination() { ... }

#[tokio::test]
async fn test_get_running_timer_returns_current_user_only() { ... }
```

`backend/crates/pm-ws/tests/dependency_handler_tests.rs`:
```rust
#[tokio::test]
async fn test_create_dependency_success() { ... }

#[tokio::test]
async fn test_create_dependency_self_reference_rejected() { ... }

#[tokio::test]
async fn test_create_dependency_duplicate_rejected() { ... }

#[tokio::test]
async fn test_create_dependency_circular_direct_rejected() {
    // A blocks B, B blocks A -> rejected
}

#[tokio::test]
async fn test_create_dependency_circular_indirect_rejected() {
    // A blocks B, B blocks C, C blocks A -> rejected with path
}

#[tokio::test]
async fn test_create_dependency_relates_to_allows_bidirectional() { ... }

#[tokio::test]
async fn test_create_dependency_cross_project_rejected() { ... }

#[tokio::test]
async fn test_create_dependency_limit_enforced() { ... }

#[tokio::test]
async fn test_delete_dependency_success() { ... }

#[tokio::test]
async fn test_get_dependencies_returns_both_directions() { ... }
```

**8.2 Frontend Tests**

`frontend/ProjectManagement.Core.Tests/Converters/TimeEntryConverterTests.cs`:
```csharp
public class TimeEntryConverterTests
{
    [Fact]
    public void ToDomain_ConvertsAllFields() { ... }

    [Fact]
    public void ToDomain_HandlesNullOptionalFields() { ... }

    [Fact]
    public void ToProto_ConvertsAllFields() { ... }

    [Fact]
    public void RoundTrip_PreservesData() { ... }

    [Fact]
    public void ToDomain_ThrowsOnNull() { ... }
}
```

`frontend/ProjectManagement.Core.Tests/Converters/DependencyConverterTests.cs`:
Similar pattern.

`frontend/ProjectManagement.Services.Tests/State/TimeEntryStoreTests.cs`:
```csharp
public class TimeEntryStoreTests
{
    [Fact]
    public async Task StartTimerAsync_CreatesOptimisticEntry() { ... }

    [Fact]
    public async Task StartTimerAsync_UpdatesRunningTimer() { ... }

    [Fact]
    public async Task StartTimerAsync_ServerFailure_RollsBack() { ... }

    [Fact]
    public async Task StopTimerAsync_ClearsRunningTimer() { ... }

    [Fact]
    public void GetByWorkItem_FiltersDeleted() { ... }

    [Fact]
    public void GetByWorkItem_OrdersByStartedAtDesc() { ... }

    [Fact]
    public void IsPending_ReflectsPendingState() { ... }

    [Fact]
    public void HandleTimerStarted_SkipsPendingUpdates() { ... }

    [Fact]
    public void Dispose_UnsubscribesFromEvents() { ... }
}
```

`frontend/ProjectManagement.Services.Tests/State/DependencyStoreTests.cs`:
```csharp
public class DependencyStoreTests
{
    [Fact]
    public async Task CreateAsync_AddsToStore() { ... }

    [Fact]
    public async Task CreateAsync_ServerRejects_RollsBack() { ... }

    [Fact]
    public void GetBlocking_ReturnsCorrectItems() { ... }

    [Fact]
    public void GetBlocked_ReturnsCorrectItems() { ... }

    [Fact]
    public void IsBlocked_ReturnsTrueWhenHasBlockingDeps() { ... }

    [Fact]
    public void IsBlocked_ReturnsFalseWhenNoBlockingDeps() { ... }
}
```

---

## File Summary by Phase

| Phase | Files to Create | Files to Modify |
|-------|-----------------|-----------------|
| 1 | - | `proto/messages.proto` |
| 2 | - | `validation_config.rs`, `message_validator.rs`, `response_builder.rs`, `time_entry_repository.rs`, `dependency_repository.rs` |
| 3 | `time_entry.rs`, `dependency.rs` | `dispatcher.rs`, `mod.rs` |
| 4 | `TimeEntry.cs`, `Dependency.cs`, `TimeEntryRequests.cs`, `DependencyRequests.cs` | `ProtoConverter.cs` |
| 5 | - | `IWebSocketClient.cs`, `WebSocketClient.cs` |
| 6 | `ITimeEntryStore.cs`, `IDependencyStore.cs`, `TimeEntryStore.cs`, `DependencyStore.cs` | `Program.cs` |
| 7 | `time-tracking.css`, `dependencies.css`, `TimerWidget.razor`, `TimeEntryList.razor`, `TimeEntryDialog.razor`, `DependencyManager.razor`, `BlockedIndicator.razor`, `AddDependencyDialog.razor` | - |
| 8 | `time_entry_handler_tests.rs`, `dependency_handler_tests.rs`, `TimeEntryConverterTests.cs`, `DependencyConverterTests.cs`, `TimeEntryStoreTests.cs`, `DependencyStoreTests.cs` | - |

**Total: ~25 files to create, ~12 files to modify**

---

## Verification Plan

1. **After Phase 2**: `just check-backend` - infrastructure compiles
2. **After Phase 3**: `just test-backend` - 20+ handler tests pass
3. **After Phase 4**: `just build-frontend` - models compile
4. **After Phase 5**: `just build-frontend` - WebSocket compiles
5. **After Phase 6**: `just test-frontend` - 15+ store tests pass
6. **After Phase 7**: `just build-frontend` - components compile
7. **After Phase 8**: `just test` - all 650+ tests pass

**Manual Integration Test** (`just dev`):
1. Start timer on work item A → timer widget shows elapsed time
2. Start timer on work item B → A's timer auto-stops, B starts
3. Stop B's timer → duration calculated and displayed
4. Create manual time entry → appears in list
5. Create dependency: A blocks B → appears in dependency manager
6. Try B blocks A → circular dependency error with path
7. BlockedIndicator appears on B in Kanban board
8. Delete dependency → indicator disappears

---

## Real-Time Broadcast & Reconnection

### Event Broadcasting

All mutation handlers broadcast events to other connected clients:

```rust
// In handle_start_timer, after successful creation:
ctx.broadcast_tx.send(BroadcastEvent::TimerStarted {
    time_entry: new_entry.clone(),
    stopped_entry: stopped_entry.clone(),
    user_id: ctx.user_id,
})?;
```

**Broadcast Events:**
- `TimerStarted` → Other clients see user's timer activity
- `TimerStopped` → Other clients update their view
- `TimeEntryCreated/Updated/Deleted` → Work item time totals refresh
- `DependencyCreated/Deleted` → BlockedIndicator updates across all views

### Reconnection State Recovery

When WebSocket reconnects, the frontend must recover state:

```csharp
// In AppState.OnReconnected():
public async Task OnReconnectedAsync()
{
    // 1. Refresh running timer (may have changed while disconnected)
    await TimeEntryStore.RefreshRunningTimerAsync();

    // 2. Refresh dependencies for currently visible work items
    foreach (var workItemId in _visibleWorkItemIds)
    {
        await DependencyStore.RefreshAsync(workItemId);
    }

    // 3. Time entries refresh lazily when user views a work item
}
```

**State Recovery Priority:**
1. **Running timer** - Critical for accurate time tracking
2. **Dependencies** - Critical for blocked indicator accuracy
3. **Time entry lists** - Can be lazy-loaded on demand

### Orphaned Timer Handling

If the server crashes while a timer is running, the timer remains in "running" state with no `ended_at`. On next app launch:

```csharp
// In TimeEntryStore initialization:
public async Task InitializeAsync()
{
    var running = await _client.GetRunningTimerAsync();
    if (running != null)
    {
        // Check if timer has been running unreasonably long (>24 hours)
        if (running.Elapsed > TimeSpan.FromHours(24))
        {
            _logger.LogWarning(
                "Orphaned timer detected: {TimerId} running for {Hours}h",
                running.Id, running.Elapsed.TotalHours);

            // Prompt user to either continue or stop with estimated time
            // This is handled in UI layer via OnOrphanedTimerDetected event
        }
        _runningTimer = running;
    }
}
```

---

## Success Criteria

### Time Tracking
- [ ] Only ONE running timer per user (atomic check-stop-create)
- [ ] Starting new timer auto-stops previous with notification
- [ ] Manual time entry creation with timestamp validation
- [ ] Owner-only edit/delete for time entries
- [ ] Pagination for time entries list (default 100, max 500)
- [ ] Max duration validation (24 hours)
- [ ] No future timestamps (60s tolerance)

### Dependencies
- [ ] Self-referential dependency rejected
- [ ] Circular dependency detected with path in error message
- [ ] Duplicate dependency rejected
- [ ] Same-project only for dependencies
- [ ] Max 50 blocking + 50 blocked per item enforced
- [ ] BlockedIndicator shows on blocked items

### Infrastructure
- [ ] Activity logging for all mutations
- [ ] Soft delete filtering (`deleted_at IS NULL`) in all queries
- [ ] UTC timestamps throughout
- [ ] Broadcast events to other connected clients
- [ ] Running timer state recovery on reconnect
- [ ] Orphaned timer detection (>24h running)

### Quality
- [ ] CSS styling for all new components
- [ ] All 615+ existing tests still pass
- [ ] 35+ new tests passing (20 backend, 15 frontend)
