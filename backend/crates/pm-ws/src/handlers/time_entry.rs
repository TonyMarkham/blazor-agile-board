#![allow(dead_code)]

use crate::{
    HandlerContext, MessageValidator, Result as WsErrorResult, WsError,
    build_running_timer_response, build_time_entries_list_response,
    build_time_entry_created_response, build_time_entry_deleted_response,
    build_time_entry_updated_response, build_timer_started_response, build_timer_stopped_response,
    check_idempotency, check_permission, db_read, db_write, decode_cached_response,
    sanitize_string, store_idempotency_non_fatal,
};

use pm_config::{DEFAULT_TIME_ENTRIES_LIMIT, MAX_TIME_ENTRIES_LIMIT};
use pm_core::{ActivityLog, Permission, TimeEntry};
use pm_db::{ActivityLogRepository, TimeEntryRepository, WorkItemRepository};
use pm_proto::{
    CreateTimeEntryRequest, DeleteTimeEntryRequest, FieldChange, GetRunningTimerRequest,
    GetTimeEntriesRequest, StartTimerRequest, StopTimerRequest, UpdateTimeEntryRequest,
    WebSocketMessage,
};

use std::panic::Location;

use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use log::{debug, info};
use uuid::Uuid;

fn parse_uuid(s: &str, field: &str) -> WsErrorResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| WsError::ValidationError {
        message: format!("Invalid UUID format for {}", field),
        field: Some(field.to_string()),
        location: ErrorLocation::from(Location::caller()),
    })
}

/// Start a timer on a work item.
///
/// # Atomicity
///
/// This operation is atomic: if the user has an existing running timer,
/// it is stopped in the same transaction as creating the new timer.
/// This prevents race conditions where a user could have multiple timers.
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
    })
    .await?;
    if let Some(cached_response) = cached {
        info!(
            "{} Returning cached idempotent response for StartTimer",
            ctx.log_prefix()
        );
        return decode_cached_response(&cached_response);
    }

    // 4. Verify work item exists and get project_id
    let work_item = db_read(&ctx, "find_work_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", work_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 5. Check Edit permission on project
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::Edit).await
    })
    .await?;

    // 6. ATOMIC TRANSACTION: Stop existing timer + Create new timer
    let (new_entry, stopped_entry) = db_write(&ctx, "start_timer_atomic", || async {
        let repo = TimeEntryRepository::new(ctx.pool.clone());

        // Find and stop any running timer for this user
        let running_timers = repo.find_running(ctx.user_id).await?;
        let stopped = if let Some(mut running) = running_timers.into_iter().next() {
            let now = Utc::now();
            running.ended_at = Some(now);
            running.duration_seconds =
                Some((now.timestamp() - running.started_at.timestamp()) as i32);
            running.updated_at = now;
            repo.update(&running).await?;

            // Activity log for auto-stopped timer
            let activity = ActivityLog::updated(
                "time_entry",
                running.id,
                ctx.user_id,
                &[FieldChange {
                    field_name: "ended_at".to_string(),
                    old_value: None,
                    new_value: Some("auto-stopped by new timer".to_string()),
                }],
            );
            ActivityLogRepository::create(&ctx.pool, &activity).await?;

            Some(running)
        } else {
            None
        };

        // Create new timer (TimeEntry::new creates a running timer by default)
        let new_timer = TimeEntry::new(
            work_item_id,
            ctx.user_id,
            req.description.as_ref().map(|d| sanitize_string(d)),
        );
        repo.create(&new_timer).await?;

        // Activity log for new timer
        let activity = ActivityLog::created("time_entry", new_timer.id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>((new_timer, stopped))
    })
    .await?;

    // 7. Build response
    let response = build_timer_started_response(
        &ctx.message_id,
        &new_entry,
        stopped_entry.as_ref(),
        ctx.user_id,
    );

    // 8. Store idempotency (non-fatal if fails)
    store_idempotency_non_fatal(&ctx.pool, &ctx.message_id, "start_timer", &response).await;

    info!(
        "{} Started timer {} on work item {}, stopped previous: {}",
        ctx.log_prefix(),
        new_entry.id,
        work_item_id,
        stopped_entry.is_some()
    );

    Ok(response)
}

/// Stop a running timer.
///
/// # Owner-Only
///
/// Only the user who created the timer can stop it.
pub async fn handle_stop_timer(
    req: StopTimerRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} StopTimer starting", ctx.log_prefix());

    // 1. Parse time_entry_id
    let time_entry_id = parse_uuid(&req.time_entry_id, "time_entry_id")?;

    // 2. Check idempotency
    let cached = db_read(&ctx, "check_idempotency", || async {
        check_idempotency(&ctx.pool, &ctx.message_id).await
    })
    .await?;
    if let Some(cached_response) = cached {
        info!(
            "{} Returning cached idempotent response for StopTimer",
            ctx.log_prefix()
        );
        return decode_cached_response(&cached_response);
    }

    // 3. Find the time entry
    let repo = TimeEntryRepository::new(ctx.pool.clone());
    let mut entry = db_read(&ctx, "find_time_entry", || async {
        repo.find_by_id(time_entry_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Time entry {} not found", time_entry_id),
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
    if entry.ended_at.is_some() {
        return Err(WsError::ValidationError {
            message: "Timer is not running".into(),
            field: Some("time_entry_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 6. Stop the timer
    let now = Utc::now();
    entry.ended_at = Some(now);
    entry.duration_seconds = Some((now.timestamp() - entry.started_at.timestamp()) as i32);
    entry.updated_at = now;

    db_write(&ctx, "stop_timer", || async {
        let repo = TimeEntryRepository::new(ctx.pool.clone());
        repo.update(&entry).await?;

        let activity = ActivityLog::updated(
            "time_entry",
            entry.id,
            ctx.user_id,
            &[FieldChange {
                field_name: "ended_at".to_string(),
                old_value: None,
                new_value: Some(entry.ended_at.unwrap().to_rfc3339()),
            }],
        );
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    // 7. Build response
    let response = build_timer_stopped_response(&ctx.message_id, &entry, ctx.user_id);

    // 8. Store idempotency
    store_idempotency_non_fatal(&ctx.pool, &ctx.message_id, "stop_timer", &response).await;

    info!(
        "{} Stopped timer {}, duration: {}s",
        ctx.log_prefix(),
        entry.id,
        entry.duration_seconds.unwrap_or(0)
    );

    Ok(response)
}

/// Create a manual time entry (already completed).
///
/// Use this for logging time after the fact, not for running timers.
pub async fn handle_create_time_entry(
    req: CreateTimeEntryRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} CreateTimeEntry starting", ctx.log_prefix());

    // 1. Validate timestamps
    MessageValidator::validate_time_entry_timestamps(req.started_at, req.ended_at)?;
    MessageValidator::validate_time_entry_description(req.description.as_deref())?;

    // 2. Parse work_item_id
    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // 3. Check idempotency
    let cached = db_read(&ctx, "check_idempotency", || async {
        check_idempotency(&ctx.pool, &ctx.message_id).await
    })
    .await?;
    if let Some(cached_response) = cached {
        info!("{} Returning cached idempotent response", ctx.log_prefix());
        return decode_cached_response(&cached_response);
    }

    // 4. Verify work item exists, check Edit permission
    let work_item = db_read(&ctx, "find_work_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", work_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::Edit).await
    })
    .await?;

    // 5. Create entry with explicit timestamps
    let started_at =
        DateTime::from_timestamp(req.started_at, 0).ok_or_else(|| WsError::ValidationError {
            message: "Invalid started_at timestamp".into(),
            field: Some("started_at".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;
    let ended_at =
        DateTime::from_timestamp(req.ended_at, 0).ok_or_else(|| WsError::ValidationError {
            message: "Invalid ended_at timestamp".into(),
            field: Some("ended_at".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let duration_seconds = (req.ended_at - req.started_at) as i32;
    let now = Utc::now();

    let entry = TimeEntry {
        id: Uuid::new_v4(),
        work_item_id,
        user_id: ctx.user_id,
        started_at,
        ended_at: Some(ended_at),
        duration_seconds: Some(duration_seconds),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    // 6. Save with activity log
    let entry_clone = entry.clone();
    db_write(&ctx, "create_time_entry", || async {
        TimeEntryRepository::new(ctx.pool.clone())
            .create(&entry_clone)
            .await?;
        let activity = ActivityLog::created("time_entry", entry_clone.id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // 7. Build response
    let response = build_time_entry_created_response(&ctx.message_id, &entry, ctx.user_id);

    // 8. Store idempotency
    store_idempotency_non_fatal(&ctx.pool, &ctx.message_id, "create_time_entry", &response).await;

    info!(
        "{} Created manual time entry {} for {} ({}s)",
        ctx.log_prefix(),
        entry.id,
        work_item_id,
        duration_seconds
    );

    Ok(response)
}

/// Update a time entry (owner-only).
pub async fn handle_update_time_entry(
    req: UpdateTimeEntryRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} UpdateTimeEntry starting", ctx.log_prefix());

    let time_entry_id = parse_uuid(&req.time_entry_id, "time_entry_id")?;

    // Find entry
    let repo = TimeEntryRepository::new(ctx.pool.clone());
    let mut entry = db_read(&ctx, "find_time_entry", || async {
        repo.find_by_id(time_entry_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Time entry {} not found", time_entry_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Owner-only check
    if entry.user_id != ctx.user_id {
        return Err(WsError::Unauthorized {
            message: "Cannot update another user's time entry".into(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Validate description if provided
    MessageValidator::validate_time_entry_description(req.description.as_deref())?;

    // Apply updates
    let mut field_changes: Vec<FieldChange> = Vec::new();

    if let Some(started_at) = req.started_at {
        let new_started =
            DateTime::from_timestamp(started_at, 0).ok_or_else(|| WsError::ValidationError {
                message: "Invalid started_at timestamp".into(),
                field: Some("started_at".into()),
                location: ErrorLocation::from(Location::caller()),
            })?;
        field_changes.push(FieldChange {
            field_name: "started_at".to_string(),
            old_value: Some(entry.started_at.to_rfc3339()),
            new_value: Some(new_started.to_rfc3339()),
        });
        entry.started_at = new_started;
    }

    if let Some(ended_at) = req.ended_at {
        let new_ended =
            DateTime::from_timestamp(ended_at, 0).ok_or_else(|| WsError::ValidationError {
                message: "Invalid ended_at timestamp".into(),
                field: Some("ended_at".into()),
                location: ErrorLocation::from(Location::caller()),
            })?;
        field_changes.push(FieldChange {
            field_name: "ended_at".to_string(),
            old_value: entry.ended_at.map(|e| e.to_rfc3339()),
            new_value: Some(new_ended.to_rfc3339()),
        });
        entry.ended_at = Some(new_ended);
    }

    if let Some(ref desc) = req.description {
        let sanitized = sanitize_string(desc);
        field_changes.push(FieldChange {
            field_name: "description".to_string(),
            old_value: entry.description.clone(),
            new_value: Some(sanitized.clone()),
        });
        entry.description = Some(sanitized);
    }

    // Validate final timestamps if both present
    if let Some(ended_at) = entry.ended_at {
        MessageValidator::validate_time_entry_timestamps(
            entry.started_at.timestamp(),
            ended_at.timestamp(),
        )?;
        entry.duration_seconds = Some((ended_at.timestamp() - entry.started_at.timestamp()) as i32);
    }

    entry.updated_at = Utc::now();

    // Save
    if !field_changes.is_empty() {
        db_write(&ctx, "update_time_entry", || async {
            repo.update(&entry).await?;
            let activity =
                ActivityLog::updated("time_entry", entry.id, ctx.user_id, &field_changes);
            ActivityLogRepository::create(&ctx.pool, &activity).await?;
            Ok::<_, WsError>(())
        })
        .await?;
    }

    info!("{} Updated time entry {}", ctx.log_prefix(), entry.id);

    Ok(build_time_entry_updated_response(
        &ctx.message_id,
        &entry,
        ctx.user_id,
    ))
}

/// Delete a time entry (owner-only, soft delete).
pub async fn handle_delete_time_entry(
    req: DeleteTimeEntryRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} DeleteTimeEntry starting", ctx.log_prefix());

    let time_entry_id = parse_uuid(&req.time_entry_id, "time_entry_id")?;

    // Find entry
    let repo = TimeEntryRepository::new(ctx.pool.clone());
    let entry = db_read(&ctx, "find_time_entry", || async {
        repo.find_by_id(time_entry_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Time entry {} not found", time_entry_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Owner-only check
    if entry.user_id != ctx.user_id {
        return Err(WsError::Unauthorized {
            message: "Cannot delete another user's time entry".into(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Soft delete
    let now = Utc::now().timestamp();
    db_write(&ctx, "delete_time_entry", || async {
        repo.delete(time_entry_id, now).await?;
        let activity = ActivityLog::deleted("time_entry", time_entry_id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    info!("{} Deleted time entry {}", ctx.log_prefix(), time_entry_id);

    Ok(build_time_entry_deleted_response(
        &ctx.message_id,
        time_entry_id,
        entry.work_item_id,
        ctx.user_id,
    ))
}

/// Get time entries for a work item (paginated).
pub async fn handle_get_time_entries(
    req: GetTimeEntriesRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} GetTimeEntries starting", ctx.log_prefix());

    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // Apply pagination limits
    let limit = req
        .limit
        .map(|l| l.clamp(1, MAX_TIME_ENTRIES_LIMIT))
        .unwrap_or(DEFAULT_TIME_ENTRIES_LIMIT);
    let offset = req.offset.unwrap_or(0).max(0);

    // Verify work item exists, check View permission
    let work_item = db_read(&ctx, "find_work_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", work_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::View).await
    })
    .await?;

    // Get paginated entries
    let (entries, total_count) = db_read(&ctx, "get_time_entries", || async {
        TimeEntryRepository::new(ctx.pool.clone())
            .find_by_work_item_paginated(work_item_id, limit, offset)
            .await
            .map_err(WsError::from)
    })
    .await?;

    debug!(
        "{} Found {} time entries (total: {}) for work item {}",
        ctx.log_prefix(),
        entries.len(),
        total_count,
        work_item_id
    );

    Ok(build_time_entries_list_response(
        &ctx.message_id,
        &entries,
        total_count,
    ))
}

/// Get the current user's running timer (if any).
pub async fn handle_get_running_timer(
    _req: GetRunningTimerRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} GetRunningTimer starting", ctx.log_prefix());

    let repo = TimeEntryRepository::new(ctx.pool.clone());
    let running = db_read(&ctx, "find_running_timer", || async {
        repo.find_running(ctx.user_id).await.map_err(WsError::from)
    })
    .await?;

    let entry = running.into_iter().next();

    if entry.is_some() {
        debug!("{} Found running timer", ctx.log_prefix());
    } else {
        debug!("{} No running timer", ctx.log_prefix());
    }

    Ok(build_running_timer_response(
        &ctx.message_id,
        entry.as_ref(),
    ))
}
