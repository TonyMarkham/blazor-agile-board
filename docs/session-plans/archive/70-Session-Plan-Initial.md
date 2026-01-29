# Session 70: Activity Logging & Polish - Implementation Plan

**Goal**: Production-ready application with activity feed UI, UX polish, LLM context seeding, and documentation

**Production Grade Target**: 9.25/10 - pagination, error handling, accessibility, performance, security, resilience

---

## Current State Summary

### Already Complete
- **Activity Logging Backend**: `pm_activity_log` table, `ActivityLog` model, `ActivityLogRepository`, all mutation handlers log activity
- **Error Handling**: `AppErrorBoundary`, `ConnectionStatus`, `OfflineBanner`, `LoadingButton`, `CircuitBreaker`, `ReconnectionService`
- **LLM Context Schema**: `pm_llm_context` table exists (empty)
- **Architecture Docs**: Complete, ADRs complete

### Needs Implementation
1. Activity Log query handler with pagination + frontend UI
2. Toast notification service with queue management
3. LLM context seed data + query endpoint
4. User documentation

---

## Dependency Graph

```
                    ┌─────────────────┐
                    │  Phase 1 (root) │
                    │  No dependencies│
                    └────────┬────────┘
           ┌─────────────────┼─────────────────┐
           ▼                 ▼                 ▼
    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
    │ Activity Log │  │ Toast/Notif  │  │ LLM Context  │
    │ Proto + BE   │  │ Service      │  │ Seed + Query │
    │ Handler      │  │              │  │              │
    └──────┬───────┘  └──────┬───────┘  └──────────────┘
           │                 │
           ▼                 ▼
    ┌──────────────┐  ┌──────────────┐
    │ Activity Log │  │ Store Toast  │
    │ Frontend UI  │  │ Integration  │
    └──────────────┘  └──────────────┘
                    │
                    ▼
           ┌─────────────────┐
           │  Documentation  │
           │  (depends on    │
           │  final features)│
           └─────────────────┘
```

---

## Phase 1: Independent Tasks (No Dependencies)

### 1A. Activity Log Backend Handler (Production Grade)

**Proto Messages** (`proto/messages.proto` at field 140+):
```protobuf
message GetActivityLogRequest {
  string entity_type = 1;  // "work_item", "sprint", "comment", "time_entry", "dependency", "project"
  string entity_id = 2;
  int32 limit = 3;         // Default 50, max 100, validated
  int32 offset = 4;        // For pagination, default 0
}

message ActivityLogEntry {
  string id = 1;
  string entity_type = 2;
  string entity_id = 3;
  string action = 4;       // "created", "updated", "deleted"
  optional string field_name = 5;
  optional string old_value = 6;
  optional string new_value = 7;
  string user_id = 8;
  int64 timestamp = 9;
  optional string comment = 10;
}

message ActivityLogList {
  repeated ActivityLogEntry entries = 1;
  int32 total_count = 2;   // For pagination UI
  bool has_more = 3;       // Quick check for "load more"
}

// Real-time activity broadcast (sent to subscribed clients)
message ActivityLogCreated {
  ActivityLogEntry entry = 1;
}
```

**Repository Enhancement** (`activity_log_repository.rs`):
```rust
// Add new method with pagination
pub async fn find_by_entity_paginated(
    executor: E,
    entity_type: &str,
    entity_id: Uuid,
    limit: i64,
    offset: i64,
) -> DbErrorResult<(Vec<ActivityLog>, i64)>  // Returns (entries, total_count)

// Retention policy - called by background task
pub async fn delete_older_than(
    executor: E,
    cutoff: DateTime<Utc>,
) -> DbErrorResult<u64>  // Returns deleted count
```

**Handler** (`backend/crates/pm-ws/src/handlers/activity_log.rs`):
```rust
pub async fn handle_get_activity_log(
    req: GetActivityLogRequest,
    ctx: HandlerContext,
) -> Result<WebSocketMessage, WsError> {
    // 1. Validate entity_type is in allowed set
    const VALID_TYPES: &[&str] = &["work_item", "sprint", "comment", "time_entry", "dependency", "project"];
    if !VALID_TYPES.contains(&req.entity_type.as_str()) {
        return Err(WsError::Validation {
            field: "entity_type".into(),
            message: format!("Invalid entity type. Must be one of: {:?}", VALID_TYPES),
        });
    }

    // 2. Parse and validate entity_id
    let entity_id = Uuid::parse_str(&req.entity_id)
        .map_err(|_| WsError::Validation {
            field: "entity_id".into(),
            message: "Invalid UUID format".into(),
        })?;

    // 3. SECURITY: Verify user has access to the entity's project
    //    (uses existing project membership check pattern)
    let project_id = get_entity_project_id(&ctx.pool, &req.entity_type, entity_id).await?;
    if !ctx.user_can_view_project(project_id).await? {
        return Err(WsError::Forbidden {
            message: "You don't have access to this entity".into(),
        });
    }

    // 4. Validate and clamp pagination params
    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as i64;

    // 5. Query with pagination (uses circuit breaker via db_ops wrapper)
    let (entries, total) = db_ops::execute_with_resilience(
        || ActivityLogRepository::find_by_entity_paginated(
            &ctx.pool, &req.entity_type, entity_id, limit, offset
        ),
        &ctx.circuit_breaker,
    ).await?;

    // 6. Build response with correlation ID
    build_activity_log_list_response(ctx.message_id, ctx.correlation_id, entries, total, limit, offset)
}

// Helper to resolve entity -> project for access control
async fn get_entity_project_id(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<Uuid, WsError> {
    match entity_type {
        "work_item" => WorkItemRepository::find_by_id(pool, entity_id).await?.map(|wi| wi.project_id),
        "sprint" => SprintRepository::find_by_id(pool, entity_id).await?.map(|s| s.project_id),
        "comment" => {
            let comment = CommentRepository::find_by_id(pool, entity_id).await?;
            let work_item = WorkItemRepository::find_by_id(pool, comment.work_item_id).await?;
            work_item.map(|wi| wi.project_id)
        },
        // ... similar for other types
        _ => None,
    }.ok_or_else(|| WsError::NotFound { entity: entity_type.into(), id: entity_id })
}
```

**Real-time Activity Broadcasts:**
Update existing mutation handlers to broadcast `ActivityLogCreated` after logging:
```rust
// In work_item.rs handle_create, after ActivityLogRepository::create():
ctx.broadcast_to_project(project_id, WebSocketMessage {
    payload: Some(Payload::ActivityLogCreated(ActivityLogCreated {
        entry: activity_to_proto(&activity_log),
    })),
    ..
}).await;
```

**Retention Policy Configuration** (`pm-config`):
```toml
[activity_log]
retention_days = 90  # Delete activity older than 90 days
cleanup_interval_hours = 24  # Run cleanup daily
```

**Files to Create:**
- `backend/crates/pm-ws/src/handlers/activity_log.rs`

**Files to Modify:**
- `proto/messages.proto` - Add messages + payload fields 140-142
- `backend/crates/pm-db/src/repositories/activity_log_repository.rs` - Add paginated + cleanup methods
- `backend/crates/pm-ws/src/handlers/mod.rs` - Export module
- `backend/crates/pm-ws/src/handlers/dispatcher.rs` - Add match arm
- `backend/crates/pm-ws/src/handlers/response_builder.rs` - Add builder
- `backend/crates/pm-ws/src/handlers/work_item.rs` - Add broadcast after activity log
- `backend/crates/pm-ws/src/handlers/sprint.rs` - Add broadcast after activity log
- `backend/crates/pm-ws/src/handlers/comment.rs` - Add broadcast after activity log
- `backend/crates/pm-config/src/config.rs` - Add activity_log retention config

**Error Handling:**
- Invalid entity_type → ValidationError with allowed values
- Invalid UUID → ValidationError with field context
- Entity not found → NotFoundError with entity type and ID
- Access denied → ForbiddenError (don't leak entity existence)
- Empty result → Success with empty list (not error)
- Database error → Internal error with correlation ID

**Tests:**
- `get_activity_log_returns_paginated_entries`
- `get_activity_log_invalid_entity_type_returns_validation_error`
- `get_activity_log_clamps_limit_to_max_100`
- `get_activity_log_empty_entity_returns_empty_list`
- `get_activity_log_returns_total_count_and_has_more`
- `get_activity_log_requires_project_access` (security)
- `get_activity_log_forbidden_returns_403` (security)
- `activity_broadcast_sent_on_work_item_create`
- `retention_cleanup_deletes_old_entries`

**Property-based tests:**
- Fuzz entity_id with random strings (should return validation error, not crash)
- Fuzz limit/offset with negative/extreme values (should clamp safely)

---

### 1B. Toast Notification Service (Production Grade)

**Interface** (`frontend/ProjectManagement.Services/Notifications/IToastService.cs`):
```csharp
public interface IToastService
{
    void ShowSuccess(string message, string? title = null, int? durationMs = null);
    void ShowError(string message, string? title = null, int? durationMs = null);
    void ShowWarning(string message, string? title = null, int? durationMs = null);
    void ShowInfo(string message, string? title = null, int? durationMs = null);

    // For operations that might need undo
    void ShowWithAction(string message, string actionText, Func<Task> onAction, int durationMs = 5000);

    // Queue management
    void Clear();

    // Observable for testing
    int ActiveCount { get; }
}
```

**Implementation** (`frontend/ProjectManagement.Services/Notifications/ToastService.cs`):
```csharp
public sealed class ToastService : IToastService, IDisposable
{
    private readonly NotificationService _radzen;
    private readonly ILogger<ToastService> _logger;
    private readonly object _lock = new();

    // Default durations by severity (configurable)
    public static class Defaults
    {
        public const int SuccessDurationMs = 3000;
        public const int ErrorDurationMs = 5000;
        public const int WarningDurationMs = 4000;
        public const int InfoDurationMs = 3000;
        public const int MaxConcurrentToasts = 3;
    }

    private int _activeCount;
    public int ActiveCount => _activeCount;

    public void ShowSuccess(string message, string? title = null, int? durationMs = null)
    {
        Show(NotificationSeverity.Success, message, title, durationMs ?? Defaults.SuccessDurationMs);
    }

    public void ShowError(string message, string? title = null, int? durationMs = null)
    {
        // Errors always show, don't count against limit (important feedback)
        _logger.LogWarning("User error displayed: {Message}", message);
        _radzen.Notify(new NotificationMessage
        {
            Severity = NotificationSeverity.Error,
            Summary = title ?? "Error",
            Detail = message,
            Duration = durationMs ?? Defaults.ErrorDurationMs,
            CloseOnClick = true,
        });
    }

    private void Show(NotificationSeverity severity, string message, string? title, int durationMs)
    {
        lock (_lock)
        {
            if (_activeCount >= Defaults.MaxConcurrentToasts)
            {
                _logger.LogDebug("Toast suppressed (queue full): {Message}", message);
                return;
            }
            _activeCount++;
        }

        _radzen.Notify(new NotificationMessage
        {
            Severity = severity,
            Summary = title ?? SeverityToDefaultTitle(severity),
            Detail = message,
            Duration = durationMs,
            CloseOnClick = true,
        });

        // Schedule decrement after duration
        _ = DecrementAfterDelay(durationMs);
    }

    private async Task DecrementAfterDelay(int delayMs)
    {
        await Task.Delay(delayMs);
        lock (_lock)
        {
            _activeCount = Math.Max(0, _activeCount - 1);
        }
    }

    public void ShowWithAction(string message, string actionText, Func<Task> onAction, int durationMs = 5000)
    {
        // Radzen doesn't support action buttons natively, use custom component
        // This would render a special toast with a button
        throw new NotImplementedException("Use ActionToast component directly");
    }

    public void Clear()
    {
        // Radzen doesn't expose clear, but we reset our counter
        lock (_lock)
        {
            _activeCount = 0;
        }
    }

    private static string SeverityToDefaultTitle(NotificationSeverity severity) => severity switch
    {
        NotificationSeverity.Success => "Success",
        NotificationSeverity.Warning => "Warning",
        NotificationSeverity.Info => "Info",
        _ => ""
    };

    public void Dispose()
    {
        Clear();
    }
}
```

**Spinner Component** (`frontend/ProjectManagement.Components/Shared/OperationSpinner.razor`):
```razor
@implements IDisposable

@* Overlay for blocking operations *@
<div class="operation-spinner @(IsVisible ? "visible" : "")"
     role="progressbar"
     aria-busy="@IsVisible"
     aria-valuetext="@(Message ?? "Loading")"
     aria-label="@(Message ?? "Loading")">
    <div class="spinner-content">
        <RadzenProgressBarCircular ShowValue="false" Mode="ProgressBarMode.Indeterminate" />
        @if (!string.IsNullOrEmpty(Message))
        {
            <span class="spinner-message" aria-live="polite">@Message</span>
        }
    </div>
</div>

@code {
    [Parameter] public bool IsVisible { get; set; }
    [Parameter] public string? Message { get; set; }
    [Parameter] public EventCallback OnCancel { get; set; }

    private bool _previousVisible;

    protected override void OnParametersSet()
    {
        // Announce state changes to screen readers
        if (IsVisible != _previousVisible)
        {
            _previousVisible = IsVisible;
            // State change handled by aria-live
        }
    }

    public void Dispose() { }
}
```

**Files to Create:**
- `frontend/ProjectManagement.Services/Notifications/IToastService.cs`
- `frontend/ProjectManagement.Services/Notifications/ToastService.cs`
- `frontend/ProjectManagement.Components/Shared/OperationSpinner.razor`

**Files to Modify:**
- `frontend/ProjectManagement.Wasm/Program.cs` - Register services
- `frontend/ProjectManagement.Components/wwwroot/css/app.css` - Toast/spinner styles

**Accessibility:**
- Toast has `role="status"` and `aria-live="polite"`
- Error toasts have `aria-live="assertive"` (more urgent)
- Spinner has `role="progressbar"` with `aria-busy`
- Keyboard dismissible (Escape key - handled by Radzen)

**Tests:**
- `ToastService_ShowSuccess_RendersCorrectSeverity`
- `ToastService_QueueLimit_PreventsTooManyToasts`
- `ToastService_ErrorsAlwaysShow_BypassQueue`
- `ToastService_ActiveCount_DecrementsAfterDuration`
- `ToastService_Clear_ResetsActiveCount`
- `OperationSpinner_SetsAriaBusy_WhenVisible`
- `OperationSpinner_AnnouncesMessage_ViaAriaLive`

---

### 1C. LLM Context Seeding + Query Endpoint (Production Grade)

**Model** (`backend/crates/pm-core/src/models/llm_context.rs`):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmContext {
    pub id: Uuid,
    pub context_type: LlmContextType,
    pub category: String,
    pub title: String,
    pub content: String,
    pub example_sql: Option<String>,
    pub example_description: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum LlmContextType {
    SchemaDoc,
    QueryPattern,
    BusinessRule,
    Instruction,
    Example,
}

impl std::fmt::Display for LlmContextType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemaDoc => write!(f, "schema_doc"),
            Self::QueryPattern => write!(f, "query_pattern"),
            Self::BusinessRule => write!(f, "business_rule"),
            Self::Instruction => write!(f, "instruction"),
            Self::Example => write!(f, "example"),
        }
    }
}
```

**Repository** (`backend/crates/pm-db/src/repositories/llm_context_repository.rs`):
```rust
impl LlmContextRepository {
    pub async fn find_all<'e, E>(executor: E) -> DbErrorResult<Vec<LlmContext>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>;

    pub async fn find_by_category<'e, E>(executor: E, category: &str) -> DbErrorResult<Vec<LlmContext>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>;

    pub async fn find_by_type<'e, E>(executor: E, context_type: &str) -> DbErrorResult<Vec<LlmContext>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>;

    pub async fn find_by_priority_above<'e, E>(executor: E, min_priority: i32) -> DbErrorResult<Vec<LlmContext>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>;
}
```

**Proto Messages** (`proto/messages.proto` at field 143-145):
```protobuf
message GetLlmContextRequest {
  optional string category = 1;      // Filter by category
  optional string context_type = 2;  // Filter by type (schema_doc, query_pattern, business_rule, instruction, example)
  optional int32 min_priority = 3;   // Filter by minimum priority
}

message LlmContextEntry {
  string id = 1;
  string context_type = 2;
  string category = 3;
  string title = 4;
  string content = 5;
  optional string example_sql = 6;
  optional string example_description = 7;
  int32 priority = 8;
}

message LlmContextList {
  repeated LlmContextEntry entries = 1;
}
```

**Seed Migration** (`backend/crates/pm-db/migrations/20260127000001_seed_llm_context.sql`):

**Schema Documentation (10 entries, priority 100):**

| ID | Category | Title | Content Summary |
|----|----------|-------|-----------------|
| 1 | work_items | Work Item Hierarchy | Single-table polymorphic design with item_type (epic/story/task), parent_id for hierarchy, project_id for filtering |
| 2 | sprints | Sprint Lifecycle | Status transitions: planned → active → completed/cancelled, version field for optimistic locking |
| 3 | comments | Comment Threading | work_item_id FK, author_id for ownership checks, parent_id for threaded replies |
| 4 | time_entries | Time Entry Structure | Running timers have NULL end_at, duration computed on stop, one active timer per user |
| 5 | dependencies | Dependency Graph | from_item_id → to_item_id, dependency_type (blocks/relates), same project constraint |
| 6 | activity_log | Activity Audit Trail | entity_type + entity_id + action + field changes, immutable append-only |
| 7 | general | Soft Delete Pattern | All tables have deleted_at column, ALWAYS filter WHERE deleted_at IS NULL |
| 8 | general | Audit Columns | created_at/created_by, updated_at/updated_by on all mutable tables |
| 9 | general | UUID Primary Keys | All IDs are TEXT storing UUID v4, compare as strings |
| 10 | swim_lanes | Swim Lanes | Custom Kanban columns per project, position for ordering |

**Query Patterns (8 entries, priority 90):**

| ID | Title | Example SQL |
|----|-------|-------------|
| 11 | Project Hierarchy | WITH RECURSIVE hierarchy AS (...) |
| 12 | Find Blocked Items | SELECT wi.* FROM pm_work_items wi JOIN pm_dependencies d ON d.to_item_id = wi.id |
| 13 | Sprint Velocity | SELECT SUM(story_points) FROM pm_work_items WHERE sprint_id = ? AND status = 'done' |
| 14 | Time Summary | SELECT work_item_id, SUM(duration) FROM pm_time_entries GROUP BY work_item_id |
| 15 | Recent Activity | SELECT * FROM pm_activity_log ORDER BY timestamp DESC LIMIT 50 |
| 16 | User Workload | SELECT assignee_id, COUNT(*) FROM pm_work_items WHERE status != 'done' GROUP BY assignee_id |
| 17 | Dependency Chain | WITH RECURSIVE deps AS (...) to find transitive blockers |
| 18 | Backlog Items | SELECT * FROM pm_work_items WHERE sprint_id IS NULL ORDER BY position |

**Business Rules (5 entries, priority 80):**

| ID | Title | Rule |
|----|-------|------|
| 19 | Sprint Assignment | Only stories and tasks can be assigned to sprints, not epics |
| 20 | Timer Exclusivity | Maximum one running timer per user (end_at IS NULL) |
| 21 | Comment Ownership | Only author can edit/delete comments (author_id check) |
| 22 | Same-Project Deps | Both work items in a dependency must belong to same project |
| 23 | Status Machine | Valid transitions: todo→in_progress→review→done (customizable per project) |

**Instructions (5 entries, priority 70):**

| ID | Title | Instruction |
|----|-------|-------------|
| 24 | Filter Deleted | Always include WHERE deleted_at IS NULL in queries |
| 25 | Use Position | ORDER BY position for display ordering in lists |
| 26 | UUID Comparison | Compare UUIDs as TEXT (string equality) |
| 27 | Limit Results | Always use LIMIT to prevent unbounded queries |
| 28 | History Lookup | Use pm_activity_log for change history, never query snapshots |

**Handler** (`backend/crates/pm-ws/src/handlers/llm_context.rs`):
```rust
pub async fn handle_get_llm_context(
    req: GetLlmContextRequest,
    ctx: HandlerContext,
) -> Result<WebSocketMessage, WsError> {
    // No auth required - LLM context is public documentation

    // Apply filters
    let entries = if let Some(category) = req.category {
        LlmContextRepository::find_by_category(&ctx.pool, &category).await?
    } else if let Some(context_type) = req.context_type {
        LlmContextRepository::find_by_type(&ctx.pool, &context_type).await?
    } else if let Some(min_priority) = req.min_priority {
        LlmContextRepository::find_by_priority_above(&ctx.pool, min_priority).await?
    } else {
        LlmContextRepository::find_all(&ctx.pool).await?
    };

    build_llm_context_list_response(ctx.message_id, entries)
}
```

**Files to Create:**
- `backend/crates/pm-core/src/models/llm_context.rs`
- `backend/crates/pm-db/src/repositories/llm_context_repository.rs`
- `backend/crates/pm-db/migrations/20260127000001_seed_llm_context.sql`
- `backend/crates/pm-ws/src/handlers/llm_context.rs`

**Files to Modify:**
- `proto/messages.proto` - Add LLM context messages at 143-145
- `backend/crates/pm-core/src/models/mod.rs` - Export
- `backend/crates/pm-db/src/repositories/mod.rs` - Export
- `backend/crates/pm-ws/src/handlers/mod.rs` - Export
- `backend/crates/pm-ws/src/handlers/dispatcher.rs` - Add match arm

**Tests:**
- `find_all_returns_28_seeded_entries`
- `find_by_category_filters_correctly`
- `find_by_type_filters_correctly`
- `find_by_priority_above_filters_correctly`
- `handler_returns_filtered_results`
- `handler_returns_all_when_no_filters`

---

## Phase 2: Dependent Tasks

### 2A. Activity Log Frontend UI (depends on 1A)

**Model** (`frontend/ProjectManagement.Core/Models/ActivityLog.cs`):
```csharp
public sealed record ActivityLog : IEntity
{
    public Guid Id { get; init; }
    public string EntityType { get; init; } = string.Empty;
    public Guid EntityId { get; init; }
    public string Action { get; init; } = string.Empty;
    public string? FieldName { get; init; }
    public string? OldValue { get; init; }
    public string? NewValue { get; init; }
    public Guid UserId { get; init; }
    public DateTime Timestamp { get; init; }
    public string? Comment { get; init; }
}

public sealed record ActivityLogPage
{
    public IReadOnlyList<ActivityLog> Entries { get; init; } = [];
    public int TotalCount { get; init; }
    public bool HasMore { get; init; }
}
```

**WebSocket Client** - Add to interface and implementation:
```csharp
Task<ActivityLogPage> GetActivityLogAsync(
    string entityType,
    Guid entityId,
    int limit = 50,
    int offset = 0,
    CancellationToken ct = default);

// Subscribe to real-time activity updates
event Action<ActivityLog>? OnActivityLogCreated;
```

**ActivityFeed Component** (`frontend/ProjectManagement.Components/Activity/ActivityFeed.razor`):
```razor
@implements IAsyncDisposable
@inject IWebSocketClient Client
@inject ILogger<ActivityFeed> Logger

@* Activity history with infinite scroll and real-time updates *@
<div class="activity-feed"
     role="feed"
     aria-label="Activity history"
     aria-busy="@_loading"
     tabindex="0"
     @onkeydown="HandleKeyDown">

    @if (_error != null)
    {
        <div class="activity-error" role="alert">
            <RadzenIcon Icon="error" />
            <span>@_error</span>
            <RadzenButton Text="Retry" Click="@LoadInitial" Size="ButtonSize.Small" />
        </div>
    }
    else if (_loading && _entries.Count == 0)
    {
        <ActivityFeedSkeleton Rows="@PageSize" />
    }
    else if (_entries.Count == 0)
    {
        <EmptyState Icon="history" Title="No activity yet" Description="Changes will appear here" />
    }
    else
    {
        <div class="activity-list" role="list">
            @foreach (var entry in _entries)
            {
                <ActivityItem Entry="@entry" @key="entry.Id" />
            }
        </div>

        @if (_hasMore)
        {
            <LoadingButton Text="Load more"
                           LoadingText="Loading..."
                           OnClick="@LoadMore"
                           IsBusy="@_loadingMore"
                           Disabled="@_loadingMore" />
        }
    }
</div>

@code {
    [Parameter, EditorRequired] public string EntityType { get; set; } = default!;
    [Parameter, EditorRequired] public Guid EntityId { get; set; }
    [Parameter] public int PageSize { get; set; } = 20;

    private readonly List<ActivityLog> _entries = new();
    private bool _loading, _loadingMore, _hasMore;
    private string? _error;
    private int _offset;
    private Guid _currentEntityId;
    private bool _disposed;

    protected override async Task OnParametersSetAsync()
    {
        // Reset when entity changes
        if (_currentEntityId != EntityId)
        {
            _currentEntityId = EntityId;
            _entries.Clear();
            _offset = 0;
            _error = null;
            await LoadInitial();
        }
    }

    protected override void OnInitialized()
    {
        // Subscribe to real-time updates
        Client.OnActivityLogCreated += HandleActivityCreated;
    }

    private async Task LoadInitial()
    {
        _loading = true;
        _error = null;
        StateHasChanged();

        try
        {
            var page = await Client.GetActivityLogAsync(EntityType, EntityId, PageSize, 0);
            _entries.Clear();
            _entries.AddRange(page.Entries);
            _hasMore = page.HasMore;
            _offset = PageSize;
        }
        catch (Exception ex)
        {
            Logger.LogError(ex, "Failed to load activity for {EntityType} {EntityId}", EntityType, EntityId);
            _error = "Failed to load activity. Click to retry.";
        }
        finally
        {
            _loading = false;
            StateHasChanged();
        }
    }

    private async Task LoadMore()
    {
        if (_loadingMore) return; // Debounce

        _loadingMore = true;
        StateHasChanged();

        try
        {
            var page = await Client.GetActivityLogAsync(EntityType, EntityId, PageSize, _offset);
            _entries.AddRange(page.Entries);
            _hasMore = page.HasMore;
            _offset += PageSize;
        }
        catch (Exception ex)
        {
            Logger.LogError(ex, "Failed to load more activity");
            // Don't set error - user can retry
        }
        finally
        {
            _loadingMore = false;
            StateHasChanged();
        }
    }

    private void HandleActivityCreated(ActivityLog activity)
    {
        // Real-time update: prepend if matches current entity
        if (activity.EntityType == EntityType && activity.EntityId == EntityId)
        {
            _entries.Insert(0, activity);
            InvokeAsync(StateHasChanged);
        }
    }

    private void HandleKeyDown(KeyboardEventArgs e)
    {
        // Keyboard navigation: R to refresh
        if (e.Key == "r" || e.Key == "R")
        {
            _ = LoadInitial();
        }
    }

    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;
        Client.OnActivityLogCreated -= HandleActivityCreated;
    }
}
```

**ActivityItem Component** (`frontend/ProjectManagement.Components/Activity/ActivityItem.razor`):
```razor
<article class="activity-item"
         role="listitem"
         aria-label="@AriaLabel"
         tabindex="0">
    <div class="activity-icon" aria-hidden="true">
        @switch (Entry.Action)
        {
            case "created":
                <RadzenIcon Icon="add_circle" Style="color: var(--rz-success)" />
                break;
            case "updated":
                <RadzenIcon Icon="edit" Style="color: var(--rz-info)" />
                break;
            case "deleted":
                <RadzenIcon Icon="delete" Style="color: var(--rz-danger)" />
                break;
            default:
                <RadzenIcon Icon="history" />
                break;
        }
    </div>

    <div class="activity-content">
        <span class="activity-action">@FormatAction()</span>

        @if (!string.IsNullOrEmpty(Entry.FieldName))
        {
            <div class="activity-change" aria-label="@ChangeAriaLabel">
                <span class="field-name">@Entry.FieldName:</span>
                <span class="old-value" aria-label="Old value">@(Entry.OldValue ?? "(empty)")</span>
                <RadzenIcon Icon="arrow_forward" aria-hidden="true" />
                <span class="new-value" aria-label="New value">@(Entry.NewValue ?? "(empty)")</span>
            </div>
        }

        <time class="activity-timestamp"
              datetime="@Entry.Timestamp.ToString("O")"
              title="@Entry.Timestamp.ToLocalTime().ToString("f")">
            @FormatRelativeTime(Entry.Timestamp)
        </time>
    </div>
</article>

@code {
    [Parameter, EditorRequired] public ActivityLog Entry { get; set; } = default!;

    private string AriaLabel => $"{Entry.Action} {Entry.FieldName ?? "item"} {FormatRelativeTime(Entry.Timestamp)}";
    private string ChangeAriaLabel => $"Changed {Entry.FieldName} from {Entry.OldValue ?? "empty"} to {Entry.NewValue ?? "empty"}";

    private string FormatAction() => Entry.Action switch
    {
        "created" => "Created",
        "updated" => $"Updated {Entry.FieldName}",
        "deleted" => "Deleted",
        _ => Entry.Action
    };

    private string FormatRelativeTime(DateTime timestamp)
    {
        var span = DateTime.UtcNow - timestamp;
        return span.TotalSeconds switch
        {
            < 60 => "just now",
            < 3600 => $"{(int)span.TotalMinutes}m ago",
            < 86400 => $"{(int)span.TotalHours}h ago",
            < 604800 => $"{(int)span.TotalDays}d ago",
            _ => timestamp.ToLocalTime().ToString("MMM d", CultureInfo.CurrentCulture)
        };
    }
}
```

**CSS** (add to `app.css`):
```css
.activity-feed {
    padding: var(--spacing-md);
    max-height: 400px;
    overflow-y: auto;
}

.activity-feed:focus {
    outline: 2px solid var(--rz-primary);
    outline-offset: 2px;
}

.activity-error {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-md);
    background: var(--rz-danger-lighter);
    border-radius: var(--rz-border-radius);
    color: var(--rz-danger);
}

.activity-item {
    display: flex;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) 0;
    border-bottom: 1px solid var(--rz-base-200);
}

.activity-item:focus {
    background: var(--rz-base-100);
    outline: none;
}

.activity-item:last-child {
    border-bottom: none;
}

.activity-icon {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
}

.activity-content {
    flex: 1;
    min-width: 0;
}

.activity-action {
    font-weight: 500;
}

.activity-change {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--rz-body-font-size-sm);
    margin-top: var(--spacing-xs);
}

.field-name {
    color: var(--rz-text-secondary);
}

.old-value {
    text-decoration: line-through;
    color: var(--rz-danger);
    word-break: break-word;
}

.new-value {
    color: var(--rz-success);
    word-break: break-word;
}

.activity-timestamp {
    display: block;
    font-size: var(--rz-body-font-size-xs);
    color: var(--rz-text-tertiary);
    margin-top: var(--spacing-xs);
}
```

**Integration** - Add to `WorkItemDetail.razor`:
```razor
<div class="work-item-detail-layout">
    <div class="work-item-main">
        @* existing content *@
    </div>
    <aside class="work-item-sidebar" aria-label="Work item activity">
        <h3>Activity</h3>
        <ActivityFeed EntityType="work_item" EntityId="@WorkItemId" />
    </aside>
</div>
```

**Files to Create:**
- `frontend/ProjectManagement.Core/Models/ActivityLog.cs`
- `frontend/ProjectManagement.Core/Models/ActivityLogPage.cs`
- `frontend/ProjectManagement.Core/Models/GetActivityLogRequest.cs`
- `frontend/ProjectManagement.Components/Activity/ActivityFeed.razor`
- `frontend/ProjectManagement.Components/Activity/ActivityItem.razor`
- `frontend/ProjectManagement.Components/Activity/ActivityFeedSkeleton.razor`

**Files to Modify:**
- `frontend/ProjectManagement.Core/Converters/ProtoConverter.cs`
- `frontend/ProjectManagement.Core/Interfaces/IWebSocketClient.cs`
- `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs`
- `frontend/ProjectManagement.Components/Pages/WorkItemDetail.razor`
- `frontend/ProjectManagement.Components/wwwroot/css/app.css`

**Accessibility:**
- `role="feed"` on container
- `role="listitem"` on each item
- `aria-busy` during loading
- `aria-label` on actions and changes
- Keyboard navigation (focus, R to refresh)
- Semantic `<time>` element with `datetime` attribute
- Focus visible indicators

**Tests:**
- `ActivityLog_ProtoConverter_RoundTrip`
- `ActivityFeed_ShowsSkeleton_WhenLoading`
- `ActivityFeed_ShowsEmptyState_WhenNoEntries`
- `ActivityFeed_ShowsError_WithRetryButton`
- `ActivityFeed_LoadMore_AppendsPaginated`
- `ActivityFeed_LoadMore_Debounces`
- `ActivityFeed_RealTime_PrependsNewActivity`
- `ActivityFeed_KeyboardR_Refreshes`
- `ActivityItem_FormatsRelativeTime_Correctly`
- `ActivityItem_ShowsFieldChange_WithOldAndNew`
- `ActivityItem_HasProperAriaLabel`

---

### 2B. Store Toast Integration (depends on 1B)

**Inject IToastService into stores:**

`WorkItemStore.cs`:
```csharp
private readonly IToastService _toast;

public WorkItemStore(IWebSocketClient client, IToastService toast, ILogger<WorkItemStore> logger)
{
    _client = client;
    _toast = toast;
    _logger = logger;
}

public async Task CreateAsync(CreateWorkItemRequest request)
{
    // ... existing optimistic update logic ...

    try
    {
        var result = await _client.CreateWorkItemAsync(request);
        _toast.ShowSuccess($"Created \"{result.WorkItem.Title}\"");
    }
    catch (ValidationException ex)
    {
        // ... rollback ...
        _toast.ShowError(ex.Message, "Validation Error");
    }
    catch (Exception ex)
    {
        // ... rollback ...
        _logger.LogError(ex, "Failed to create work item");
        _toast.ShowError("Failed to create work item. Please try again.");
    }
}
```

**Toast messages for each store:**

| Store | Action | Success Message | Error Message |
|-------|--------|-----------------|---------------|
| WorkItemStore | Create | "Created \"{title}\"" | "Failed to create work item" |
| WorkItemStore | Update | "Saved changes" | "Failed to save changes" |
| WorkItemStore | Delete | "Deleted \"{title}\"" | "Failed to delete" |
| WorkItemStore | Move | "Moved to {status}" | "Failed to move" |
| SprintStore | Create | "Created sprint \"{name}\"" | "Failed to create sprint" |
| SprintStore | Start | "Sprint started" | "Failed to start sprint" |
| SprintStore | Complete | "Sprint completed" | "Failed to complete sprint" |
| CommentStore | Create | "Comment added" | "Failed to add comment" |
| CommentStore | Update | "Comment updated" | "Failed to update comment" |
| CommentStore | Delete | "Comment deleted" | "Failed to delete comment" |
| TimeEntryStore | Start | "Timer started" | "Failed to start timer" |
| TimeEntryStore | Stop | "Timer stopped ({duration})" | "Failed to stop timer" |
| DependencyStore | Create | "Dependency added" | "Failed to add dependency" |
| DependencyStore | Delete | "Dependency removed" | "Failed to remove dependency" |

**Connection Quality Enhancement** (`ConnectionStatus.razor`):
```razor
@inject IConnectionHealth HealthTracker
@implements IAsyncDisposable

<div class="connection-status @StatusClass @QualityClass"
     title="@TooltipText"
     role="status"
     aria-live="polite">
    <RadzenIcon Icon="@StatusIcon" />
    <span class="status-text">@StatusText</span>
    @if (ShowLatency && State == ConnectionState.Connected && _latencyMs > 0)
    {
        <span class="latency" aria-label="Latency @_latencyMs milliseconds">@_latencyMs ms</span>
    }
</div>

@code {
    [Parameter] public bool ShowLatency { get; set; } = true;
    [Parameter] public ConnectionState State { get; set; }

    private int _latencyMs;
    private Timer? _refreshTimer;
    private bool _disposed;

    protected override void OnInitialized()
    {
        // Poll health metrics every 500ms for UI updates
        _refreshTimer = new Timer(_ =>
        {
            if (!_disposed)
            {
                _latencyMs = (int)(HealthTracker.Latency?.TotalMilliseconds ?? 0);
                InvokeAsync(StateHasChanged);
            }
        }, null, TimeSpan.Zero, TimeSpan.FromMilliseconds(500));
    }

    private string QualityClass => HealthTracker.Quality switch
    {
        ConnectionQuality.Excellent => "quality-excellent",
        ConnectionQuality.Good => "quality-good",
        ConnectionQuality.Fair => "quality-fair",
        ConnectionQuality.Poor => "quality-poor",
        _ => ""
    };

    private string TooltipText => State switch
    {
        ConnectionState.Connected => $"Connected ({HealthTracker.Quality}, {_latencyMs}ms latency)",
        ConnectionState.Connecting => "Connecting to server...",
        ConnectionState.Reconnecting => $"Reconnecting (attempt {HealthTracker.ReconnectAttempts})...",
        ConnectionState.Disconnected => "Disconnected",
        _ => ""
    };

    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;

        if (_refreshTimer != null)
        {
            await _refreshTimer.DisposeAsync();
        }
    }
}
```

**Optimistic Update CSS** (add to `app.css`):
```css
/* Optimistic update visual feedback */
.is-pending {
    position: relative;
    opacity: 0.8;
}

.is-pending::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, rgba(var(--rz-primary-rgb), 0.1), transparent);
    animation: pending-sweep 1.5s ease-in-out infinite;
}

@keyframes pending-sweep {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
}

/* Connection quality colors */
.quality-excellent .latency { color: var(--rz-success); }
.quality-good .latency { color: var(--rz-success); }
.quality-fair .latency { color: var(--rz-warning); }
.quality-poor .latency { color: var(--rz-danger); }

.connection-status {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-xs) var(--spacing-sm);
    border-radius: var(--rz-border-radius);
    font-size: var(--rz-body-font-size-sm);
}

.connection-status.status-connected { color: var(--rz-success); }
.connection-status.status-connecting { color: var(--rz-warning); }
.connection-status.status-reconnecting { color: var(--rz-warning); }
.connection-status.status-disconnected { color: var(--rz-danger); }
```

**Files to Create:**
- `frontend/ProjectManagement.Components/Shared/ConnectionStatus.razor`

**Files to Modify:**
- `frontend/ProjectManagement.Services/State/WorkItemStore.cs`
- `frontend/ProjectManagement.Services/State/SprintStore.cs`
- `frontend/ProjectManagement.Services/State/CommentStore.cs`
- `frontend/ProjectManagement.Services/State/TimeEntryStore.cs`
- `frontend/ProjectManagement.Services/State/DependencyStore.cs`
- `frontend/ProjectManagement.Components/wwwroot/css/app.css`
- `frontend/ProjectManagement.Wasm/Program.cs` - Inject IToastService into stores

**Tests:**
- `WorkItemStore_Create_ShowsSuccessToast`
- `WorkItemStore_Create_ShowsErrorToast_OnFailure`
- `WorkItemStore_Create_ShowsValidationError_OnValidationException`
- `SprintStore_Start_ShowsSuccessToast`
- `TimeEntryStore_Stop_ShowsDuration_InToast`
- `ConnectionStatus_ShowsLatency_WhenEnabled`
- `ConnectionStatus_QualityClass_MatchesHealth`
- `ConnectionStatus_ReconnectAttempts_ShownInTooltip`
- `ConnectionStatus_PollsHealthMetrics_Periodically`
- `ConnectionStatus_DisposesTimer_OnDisposal`

---

## Phase 3: Final Tasks

### 3A. Documentation

**Files to Create:**

| File | Sections |
|------|----------|
| `docs/GETTING_STARTED.md` | Prerequisites (Rust 1.75+, .NET 8, Node 20), System requirements, Clone & build (`just restore && just check`), Quick start (`just dev`), First project walkthrough with screenshots, Common setup issues and solutions |
| `docs/USER_GUIDE.md` | Dashboard overview, Creating and managing projects, Work item types (Epic/Story/Task) & hierarchy, Sprint planning workflow (create → start → complete), Kanban board (drag & drop, keyboard shortcuts), Time tracking (timers and manual entries), Dependencies (blocking/blocked by), Activity history sidebar, Keyboard shortcuts table, Offline behavior |
| `docs/DEPLOYMENT_GUIDE.md` | Building release (`just build-release`), Platform-specific instructions (macOS/Windows/Linux), Code signing (macOS notarization, Windows signing), Configuration options (config.toml reference with all fields), Database location & backup strategy, Log file locations and rotation, Auto-update configuration, Updating the app |
| `docs/API_DOCUMENTATION.md` | WebSocket protocol overview, Connection flow (connect → authenticate → subscribe → operate), Message format (protobuf binary wire format), Complete request/response catalog with examples, Error codes and handling (validation, not found, forbidden), Circuit breaker behavior, Health endpoints (/health, /ready, /live), LLM context query API |
| `docs/TROUBLESHOOTING.md` | Connection refused (firewall, port in use), Database locked (concurrent access), Build failures (missing dependencies, version mismatches), Slow performance (indexing, WAL checkpoints), Debug logging (how to enable, log levels), Crash reporting, How to report issues (GitHub template link) |

**Files to Update:**
- `README.md` - Feature list, Quick start, Documentation links, Badge placeholders
- `CLAUDE.md` - Mark Session 70 complete, Update test counts, Add new file references
- `docs/implementation-plan-v2.md` - Update Session 70 status and completion notes

---

## Execution Order Summary

| Order | Task | Depends On | Parallelizable With |
|-------|------|------------|---------------------|
| 1 | Activity Log Backend (1A) | - | 1B, 1C |
| 1 | Toast Service (1B) | - | 1A, 1C |
| 1 | LLM Context Seed + Query (1C) | - | 1A, 1B |
| 2 | Activity Log Frontend (2A) | 1A | 2B |
| 2 | Store Toast Integration (2B) | 1B | 2A |
| 3 | Documentation (3A) | All above | - |

---

## Production Grade Checklist (9.25/10 Target)

### Core Functionality
- [x] Pagination for activity log (offset/limit with total_count, has_more)
- [x] Real-time activity updates via WebSocket broadcast
- [x] LLM query endpoint (context is accessible, not just stored)

### Error Handling
- [x] Validation errors with specific field context
- [x] Not found errors with entity type and ID
- [x] Access denied without leaking entity existence
- [x] Error recovery UI (retry buttons)
- [x] Graceful degradation (cached data when offline)

### Security
- [x] Project-level access control on activity queries
- [x] Correlation IDs in all responses for debugging

### Performance
- [x] Lazy loading with "load more" pagination
- [x] Skeleton loaders for initial load states
- [x] Debouncing on load more button
- [x] Activity log retention policy (configurable cleanup)
- [x] Circuit breaker integration for database operations

### Accessibility
- [x] ARIA roles (feed, listitem, status, progressbar)
- [x] Keyboard navigation (focus, shortcuts)
- [x] Screen reader announcements (aria-live)
- [x] Focus visible indicators
- [x] Semantic HTML (time, article, aside)

### Resilience
- [x] Toast queue management (prevents spam)
- [x] Error toasts bypass queue (always show)
- [x] Component disposal (cleanup subscriptions)
- [x] Real-time subscription cleanup on unmount

### Testing
- [x] Integration tests for handlers
- [x] Property-based fuzz tests for edge cases
- [x] Component tests for UI states
- [x] Accessibility assertions in tests

### Documentation
- [x] Getting started guide
- [x] User guide with feature walkthrough
- [x] Deployment guide with platform specifics
- [x] Complete API documentation
- [x] Troubleshooting guide

---

## Verification

1. **Backend**: `just test-backend && just clippy-backend`
2. **Frontend**: `just test-frontend && just build-frontend`
3. **Integration**:
   - `just dev`
   - Create work item → verify toast appears + activity in sidebar
   - Edit work item → verify activity shows field change with old/new values
   - Load more activity → verify pagination (no duplicate loads)
   - In second browser tab, edit same item → verify real-time activity update in first tab
   - Disconnect network → verify error state with retry button
   - Press R key in activity feed → verify refresh
   - Tab through activity items → verify focus indicators
   - Use screen reader → verify announcements
4. **LLM Context**:
   ```sql
   SELECT COUNT(*) FROM pm_llm_context;  -- Should be 28
   SELECT * FROM pm_llm_context WHERE context_type = 'query_pattern';  -- Should return 8
   ```
   Also test via WebSocket: `GetLlmContextRequest` with category filter returns correct entries
5. **Security**:
   - Request activity for entity in project you don't have access to → verify 403
6. **Docs**: Follow GETTING_STARTED.md on fresh clone (should complete successfully)
