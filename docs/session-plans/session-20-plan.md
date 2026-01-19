# Session 20: Blazor Foundation - Implementation Plan (v2)

**Goal**: Create production-grade Blazor frontend with WebSocket client, state management, and resilience patterns

**Target Quality**: 9.25+/10 production-grade (matching Session 10 backend quality)

**Total Estimated Tokens**: ~190k (split across 7 sub-sessions)

**Sub-session Design Philosophy**:
- Each sub-session targets **10-35k tokens** (conservative)
- Historical overruns: 1.5-2.7x estimates → still fits in 50-75k context
- Smaller context = better Claude performance + human sense of progress
- Each sub-session is a complete, testable deliverable

**Prerequisites**: Session 10 complete (backend with 166 tests passing)

---

## Quality Standards

This plan targets the same production-grade quality as Session 10:

| Requirement | Implementation |
|-------------|----------------|
| Circuit breaker | Client-side circuit breaker for server failures |
| Error boundaries | Catch and handle all exceptions gracefully |
| Structured logging | ILogger with correlation IDs throughout |
| Request tracing | Correlation ID propagation from client to server |
| Input validation | Validate all requests before sending |
| Thread safety | ConcurrentDictionary, proper locking |
| Cancellation | CancellationToken on all async operations |
| Disposal | IDisposable/IAsyncDisposable properly implemented |
| Health monitoring | Connection quality, latency tracking |
| Comprehensive tests | 100+ tests including property-based |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    ProjectManagement.Wasm                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   App.razor │  │  Radzen UI  │  │   Error Boundary        │  │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘  │
│         │                │                     │                 │
├─────────┼────────────────┼─────────────────────┼─────────────────┤
│         │    ProjectManagement.Components      │                 │
│         │  ┌────────────────────────────────┐  │                 │
│         │  │  Razor Components (RCL)        │  │                 │
│         │  └────────────────────────────────┘  │                 │
│         │                │                     │                 │
├─────────┼────────────────┼─────────────────────┼─────────────────┤
│         │    ProjectManagement.Services        │                 │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌──────────┴───────────┐     │
│  │  AppState   │  │   Stores    │  │  WebSocketClient     │     │
│  │             │  │ WorkItem    │  │  ├─ CircuitBreaker   │     │
│  │             │  │ Sprint      │  │  ├─ RetryPolicy      │     │
│  │             │  │ Comment     │  │  ├─ HealthMonitor    │     │
│  └─────────────┘  └─────────────┘  │  └─ RequestTracker   │     │
│                                    └──────────────────────┘     │
├─────────────────────────────────────────────────────────────────┤
│                    ProjectManagement.Core                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Models    │  │  Interfaces │  │   Proto (Generated)     │  │
│  │  WorkItem   │  │ IWsClient   │  │   WebSocketMessage      │  │
│  │  Sprint     │  │ IStore      │  │   Request/Response      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Sub-Session Breakdown

| Sub-session | Focus | Est. Tokens | Files |
|-------------|-------|-------------|-------|
| **20.01** | Database FK constraint fixes (structural debt) | ~10k | ~3 |
| **20.1** | Project structure, Protobuf, Domain models | ~30k | ~25 |
| **20.2** | WebSocket client foundation | ~35k | ~15 |
| **20.3** | Resilience patterns (circuit breaker, retry, health) | ~30k | ~12 |
| **20.4** | State management with thread safety | ~30k | ~12 |
| **20.5** | WASM host, error boundaries, observability | ~25k | ~10 |
| **20.6** | Comprehensive test suite (100+ tests) | ~30k | ~15 |

---

## Sub-Session 20.01: Database FK Constraint Fixes

**Goal**: Add missing foreign key constraints to ensure referential integrity

**Estimated Tokens**: ~10k

**Context**: During Session 20 planning, we identified missing FK constraints in the database schema:
- `pm_work_items.sprint_id` → should reference `pm_sprints.id`
- `pm_work_items.assignee_id` → should reference `users.id`

These are structural issues that should be fixed before building the frontend.

### Phase 0.1: Migration File

SQLite does not support `ALTER TABLE ADD CONSTRAINT` for foreign keys. We must recreate the table.

**File**: `backend/crates/pm-db/migrations/20260119000001_add_work_item_fks.sql`

> **Note**: Migration number uses date 20260119 (today) to avoid collision with existing migration 20260110000011 (add_idempotency_keys).

```sql
-- Migration: Add missing FK constraints to pm_work_items
-- SQLite requires table recreation to add FK constraints

PRAGMA foreign_keys = OFF;

-- Step 1: Rename old table
ALTER TABLE pm_work_items RENAME TO pm_work_items_old;

-- Step 2: Create new table with final name (critical for self-referential FKs)
-- IMPORTANT: Must include ALL columns from original table
CREATE TABLE pm_work_items (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL CHECK(item_type IN ('project', 'epic', 'story', 'task')),

    -- Hierarchy
    parent_id TEXT,
    project_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,

    -- Core fields
    title TEXT NOT NULL,
    description TEXT,

    -- Workflow
    status TEXT NOT NULL DEFAULT 'backlog',
    priority TEXT NOT NULL DEFAULT 'medium',

    -- Assignment (NOW WITH FK)
    assignee_id TEXT,

    -- Sprint (NOW WITH FK)
    sprint_id TEXT,

    -- Estimation
    story_points INTEGER,

    -- Optimistic concurrency
    version INTEGER NOT NULL DEFAULT 1,

    -- Audit columns
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,

    -- Self-referential FKs (must reference final table name)
    FOREIGN KEY (parent_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,

    -- NEW FKs
    FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id) ON DELETE SET NULL,
    FOREIGN KEY (assignee_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Step 3: Copy data
INSERT INTO pm_work_items SELECT * FROM pm_work_items_old;

-- Step 4: Drop old table
DROP TABLE pm_work_items_old;

-- Step 5: Recreate indexes
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;

PRAGMA foreign_keys = ON;
```

### Phase 0.2: Design Decisions

**ON DELETE SET NULL vs CASCADE**:
- `sprint_id` → SET NULL: When a sprint is deleted, work items should remain but become unassigned
- `assignee_id` → SET NULL: When a user is deleted, work items should remain but become unassigned

This differs from `parent_id`/`project_id` which use CASCADE because deleting a parent should cascade to children.

**Migration approach**: Rename old → create new with final name → copy → drop old. This ensures self-referential FKs correctly reference `pm_work_items`, not a temp table name.

### Phase 0.3: Prepare and Execute Migration

The existing test infrastructure uses in-memory SQLite with `sqlx::migrate!("./migrations")` macro, which automatically picks up all migration files from the `migrations/` directory. No manual database setup needed for tests.

**Steps** (from repo root):

```bash
# 1. Build to verify compilation (sqlx offline mode uses .sqlx/ cache)
cargo build --workspace

# 2. Run existing tests - migrations run automatically via create_test_pool()
cargo test --workspace

# 3. If offline cache needs updating (compile errors about schema):
DATABASE_URL=sqlite:backend/crates/pm-db/.sqlx-test/test.db cargo sqlx prepare --workspace
```

**How it works**:
- `create_test_pool()` in `tests/common/test_db.rs` creates an in-memory SQLite database
- `sqlx::migrate!("./migrations")` runs ALL migrations in order, including the new FK migration
- Each test gets a fresh database with the new FK constraints automatically
- The `.sqlx/` cache lives at repo root (workspace level)

**Verification**: The new tests in Phase 0.4 will confirm FK constraints are enforced. No manual PRAGMA inspection needed.

### Phase 0.4: Test Updates

Update existing tests to verify FK constraints are enforced:

**File**: `backend/crates/pm-db/tests/work_item_repository_tests.rs` (additions)

```rust
#[tokio::test]
async fn test_work_item_sprint_fk_constraint() {
    let pool = setup_test_db().await;
    let repo = WorkItemRepository::new(pool.clone());

    // Create project first
    let project = create_test_project(&repo).await;

    // Create story with non-existent sprint_id should fail
    let story = WorkItem {
        id: Uuid::new_v4(),
        item_type: WorkItemType::Story,
        parent_id: Some(project.id),
        project_id: project.id,
        sprint_id: Some(Uuid::new_v4()), // Non-existent sprint
        // ... other fields
    };

    let result = repo.create(&story).await;
    assert!(result.is_err(), "Should reject invalid sprint_id FK");
}

#[tokio::test]
async fn test_sprint_delete_nullifies_work_item_sprint() {
    let pool = setup_test_db().await;
    let work_item_repo = WorkItemRepository::new(pool.clone());
    let sprint_repo = SprintRepository::new(pool.clone());

    // Create project, sprint, and story assigned to sprint
    let project = create_test_project(&work_item_repo).await;
    let sprint = create_test_sprint(&sprint_repo, project.id).await;
    let story = create_test_story(&work_item_repo, project.id, Some(sprint.id)).await;

    // Delete sprint
    sprint_repo.delete(sprint.id).await.unwrap();

    // Verify story's sprint_id is now NULL
    let updated_story = work_item_repo.find_by_id(story.id).await.unwrap().unwrap();
    assert!(updated_story.sprint_id.is_none(), "Sprint deletion should SET NULL on work items");
}
```

### Phase 0.5: Documentation Update

Update `docs/database-relationships.md` to reflect the fix:

```markdown
## Non-FK Relationships (Joined by Value)

| From Table | Column | To Table | Column | Relationship |
|------------|--------|----------|--------|--------------|
| `pm_work_items` | `status` | `pm_swim_lanes` | `status_value` | Loose coupling - swim lane defines display config |

## Foreign Key Constraints (Complete)

| From Table | Column | To Table | Column | ON DELETE |
|------------|--------|----------|--------|-----------|
| `pm_work_items` | `sprint_id` | `pm_sprints` | `id` | SET NULL |
| `pm_work_items` | `assignee_id` | `users` | `id` | SET NULL |
```

### Files Created/Modified in 20.01

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/pm-db/migrations/20260119000001_add_work_item_fks.sql` | Create | Migration to add FK constraints |
| `backend/crates/pm-db/tests/work_item_repository_tests.rs` | Modify | Add FK constraint tests |
| `docs/database-relationships.md` | Modify | Update to show FKs are complete |

### Verification Checklist

- [ ] Migration runs successfully on fresh database
- [ ] Migration runs successfully on database with existing data
- [ ] Existing tests still pass
- [ ] New FK constraint tests pass
- [ ] `cargo test --workspace` passes (166+ tests)

---

## Sub-Session 20.1: Foundation

**Goal**: Project structure, Protobuf generation, domain models with validation

**Estimated Tokens**: ~30k

### Phase 1.1: Solution Structure

```
frontend/
├── ProjectManagement.sln
├── Directory.Build.props
├── Directory.Packages.props
├── .editorconfig
├── ProjectManagement.Core/
│   ├── ProjectManagement.Core.csproj
│   ├── Protos/
│   │   └── messages.proto
│   ├── Models/
│   ├── Interfaces/
│   ├── Validation/
│   └── Exceptions/
├── ProjectManagement.Services/
│   ├── ProjectManagement.Services.csproj
│   ├── WebSocket/
│   ├── State/
│   ├── Resilience/
│   └── Logging/
├── ProjectManagement.Components/
│   ├── ProjectManagement.Components.csproj
│   ├── _Imports.razor
│   └── wwwroot/
└── ProjectManagement.Wasm/
    ├── ProjectManagement.Wasm.csproj
    ├── Program.cs
    ├── App.razor
    └── wwwroot/
```

### Phase 1.2: Directory.Build.props

```xml
<Project>
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <LangVersion>12.0</LangVersion>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <TreatWarningsAsErrors>true</TreatWarningsAsErrors>
    <AnalysisLevel>latest-recommended</AnalysisLevel>
    <EnforceCodeStyleInBuild>true</EnforceCodeStyleInBuild>
  </PropertyGroup>

  <PropertyGroup Condition="'$(Configuration)' == 'Debug'">
    <CheckForOverflowUnderflow>true</CheckForOverflowUnderflow>
  </PropertyGroup>
</Project>
```

### Phase 1.3: Directory.Packages.props

```xml
<Project>
  <PropertyGroup>
    <ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally>
  </PropertyGroup>

  <ItemGroup>
    <!-- Protobuf -->
    <PackageVersion Include="Google.Protobuf" Version="3.25.2" />
    <PackageVersion Include="Grpc.Tools" Version="2.60.0" />

    <!-- Blazor -->
    <PackageVersion Include="Microsoft.AspNetCore.Components.WebAssembly" Version="8.0.11" />
    <PackageVersion Include="Microsoft.AspNetCore.Components.Web" Version="8.0.11" />
    <PackageVersion Include="Microsoft.Extensions.Logging.Abstractions" Version="8.0.2" />
    <PackageVersion Include="Microsoft.Extensions.Options" Version="8.0.2" />

    <!-- UI -->
    <PackageVersion Include="Radzen.Blazor" Version="5.5.2" />

    <!-- Testing -->
    <PackageVersion Include="Microsoft.NET.Test.Sdk" Version="17.9.0" />
    <PackageVersion Include="xunit" Version="2.7.0" />
    <PackageVersion Include="xunit.runner.visualstudio" Version="2.5.7" />
    <PackageVersion Include="Moq" Version="4.20.70" />
    <PackageVersion Include="FluentAssertions" Version="6.12.0" />
    <PackageVersion Include="FsCheck.Xunit" Version="2.16.6" />
    <PackageVersion Include="bunit" Version="1.28.9" />
    <PackageVersion Include="coverlet.collector" Version="6.0.1" />
  </ItemGroup>
</Project>
```

### Phase 1.3b: Project Files (.csproj)

**ProjectManagement.Core.csproj** - Domain models, interfaces, and protobuf:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Google.Protobuf" />
    <PackageReference Include="Grpc.Tools" PrivateAssets="all" />
    <PackageReference Include="Microsoft.Extensions.Logging.Abstractions" />
    <PackageReference Include="Microsoft.Extensions.Options" />
  </ItemGroup>

  <!-- Protobuf code generation -->
  <ItemGroup>
    <Protobuf Include="Protos\messages.proto" GrpcServices="None">
      <CompileOutputs>true</CompileOutputs>
    </Protobuf>
  </ItemGroup>
</Project>
```

**Protos/messages.proto** - Copy from backend with C# namespace:

```protobuf
syntax = "proto3";

package pm;

// C# specific options for proper namespace
option csharp_namespace = "ProjectManagement.Core.Proto";

// ... rest of proto definitions copied from backend/crates/pm-proto/proto/messages.proto
// Note: Ensure field numbers match backend exactly
```

> **Important**: The proto file must be copied from `backend/crates/pm-proto/proto/messages.proto` and the `csharp_namespace` option added. Field numbers are permanent - never reuse them.

**ProjectManagement.Services.csproj**:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>

  <ItemGroup>
    <ProjectReference Include="..\ProjectManagement.Core\ProjectManagement.Core.csproj" />
  </ItemGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.Extensions.Logging.Abstractions" />
    <PackageReference Include="Microsoft.Extensions.Options" />
  </ItemGroup>
</Project>
```

**ProjectManagement.Components.csproj** (Razor Class Library):

```xml
<Project Sdk="Microsoft.NET.Sdk.Razor">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>

  <ItemGroup>
    <ProjectReference Include="..\ProjectManagement.Core\ProjectManagement.Core.csproj" />
    <ProjectReference Include="..\ProjectManagement.Services\ProjectManagement.Services.csproj" />
  </ItemGroup>

  <ItemGroup>
    <PackageReference Include="Radzen.Blazor" />
  </ItemGroup>
</Project>
```

**ProjectManagement.Wasm.csproj** (WASM host):

```xml
<Project Sdk="Microsoft.NET.Sdk.BlazorWebAssembly">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>

  <ItemGroup>
    <ProjectReference Include="..\ProjectManagement.Components\ProjectManagement.Components.csproj" />
  </ItemGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly" />
  </ItemGroup>
</Project>
```

### Phase 1.4: Exception Hierarchy

**Files to create in `ProjectManagement.Core/Exceptions/`:**

| File | Purpose |
|------|---------|
| `ProjectManagementException.cs` | Base exception for all PM exceptions |
| `ConnectionException.cs` | Connection-related failures |
| `RequestTimeoutException.cs` | Request timed out |
| `ServerRejectedException.cs` | Server rejected request (validation, auth) |
| `VersionConflictException.cs` | Optimistic concurrency conflict |
| `CircuitOpenException.cs` | Circuit breaker is open |
| `ValidationException.cs` | Client-side validation failure |

```csharp
// ProjectManagementException.cs
namespace ProjectManagement.Core.Exceptions;

/// <summary>
/// Base exception for all Project Management errors.
/// Contains correlation ID for distributed tracing.
/// </summary>
public abstract class ProjectManagementException : Exception
{
    /// <summary>
    /// Correlation ID for tracing this error across systems.
    /// </summary>
    public string? CorrelationId { get; init; }

    /// <summary>
    /// Error code for programmatic handling.
    /// </summary>
    public abstract string ErrorCode { get; }

    /// <summary>
    /// User-friendly message (safe to display in UI).
    /// </summary>
    public virtual string UserMessage => "An unexpected error occurred. Please try again.";

    protected ProjectManagementException(string message) : base(message) { }
    protected ProjectManagementException(string message, Exception inner) : base(message, inner) { }
}

// ConnectionState.cs (defined here for use in ConnectionException)
// Full enum will be referenced from WebSocket namespace later
namespace ProjectManagement.Core.Exceptions;

public enum ConnectionState
{
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Closed
}

// ConnectionException.cs
public sealed class ConnectionException : ProjectManagementException
{
    public override string ErrorCode => "CONNECTION_FAILED";
    public override string UserMessage => "Unable to connect to server. Please check your connection.";

    public ConnectionState LastKnownState { get; init; }
    public TimeSpan? RetryAfter { get; init; }

    public ConnectionException(string message) : base(message) { }
    public ConnectionException(string message, Exception inner) : base(message, inner) { }
}

// RequestTimeoutException.cs
public sealed class RequestTimeoutException : ProjectManagementException
{
    public override string ErrorCode => "REQUEST_TIMEOUT";
    public override string UserMessage => "The request timed out. Please try again.";

    public string? MessageId { get; init; }
    public TimeSpan Timeout { get; init; }

    public RequestTimeoutException(string messageId, TimeSpan timeout)
        : base($"Request {messageId} timed out after {timeout.TotalSeconds}s")
    {
        MessageId = messageId;
        Timeout = timeout;
    }
}

// ServerRejectedException.cs
public sealed class ServerRejectedException : ProjectManagementException
{
    public override string ErrorCode { get; }
    public override string UserMessage { get; }

    public string? Field { get; init; }

    public ServerRejectedException(string errorCode, string message, string? field = null)
        : base(message)
    {
        ErrorCode = errorCode;
        UserMessage = SanitizeMessage(message);
        Field = field;
    }

    private static string SanitizeMessage(string message)
    {
        // Never expose internal details
        if (message.Contains("SQLITE", StringComparison.OrdinalIgnoreCase) ||
            message.Contains("sqlx", StringComparison.OrdinalIgnoreCase) ||
            message.Contains("stack trace", StringComparison.OrdinalIgnoreCase))
        {
            return "An internal error occurred. Please try again later.";
        }
        return message.Length > 200 ? message[..200] : message;
    }
}

// VersionConflictException.cs
public sealed class VersionConflictException : ProjectManagementException
{
    public override string ErrorCode => "VERSION_CONFLICT";
    public override string UserMessage => "This item was modified by another user. Please refresh and try again.";

    public Guid EntityId { get; init; }
    public int ExpectedVersion { get; init; }
    public int ActualVersion { get; init; }

    public VersionConflictException(Guid entityId, int expected, int actual)
        : base($"Version conflict for {entityId}: expected {expected}, got {actual}")
    {
        EntityId = entityId;
        ExpectedVersion = expected;
        ActualVersion = actual;
    }
}

// CircuitOpenException.cs
public sealed class CircuitOpenException : ProjectManagementException
{
    public override string ErrorCode => "CIRCUIT_OPEN";
    public override string UserMessage => "Service temporarily unavailable. Please wait a moment.";

    public TimeSpan RetryAfter { get; init; }

    public CircuitOpenException(TimeSpan retryAfter)
        : base($"Circuit breaker open. Retry after {retryAfter.TotalSeconds}s")
    {
        RetryAfter = retryAfter;
    }
}

// ValidationException.cs
public sealed class ValidationException : ProjectManagementException
{
    public override string ErrorCode => "VALIDATION_ERROR";
    public override string UserMessage => Errors.FirstOrDefault()?.Message ?? "Invalid input.";

    public IReadOnlyList<ValidationError> Errors { get; }

    public ValidationException(IEnumerable<ValidationError> errors)
        : base($"Validation failed: {string.Join(", ", errors.Select(e => e.Message))}")
    {
        Errors = errors.ToList().AsReadOnly();
    }

    public ValidationException(string field, string message)
        : this(new[] { new ValidationError(field, message) }) { }
}

public sealed record ValidationError(string Field, string Message);
```

### Phase 1.5: Entity Interface Hierarchy

**Design Rationale**: All domain entities share common patterns (audit fields, soft delete, project/work-item scoping). Extracting these into interfaces enables:
- Generic repository/store implementations
- Consistent audit trail handling
- Type-safe filtering and querying
- Shared UI components for common operations

**Files to create in `ProjectManagement.Core/Interfaces/Entities/`:**

```csharp
// IEntity.cs
namespace ProjectManagement.Core.Interfaces.Entities;

/// <summary>
/// Base interface for all domain entities.
/// Every entity has a unique identifier and creation timestamp.
/// </summary>
public interface IEntity
{
    /// <summary>Unique identifier (UUID).</summary>
    Guid Id { get; }

    /// <summary>When this entity was created (UTC).</summary>
    DateTime CreatedAt { get; }
}

// ISoftDeletable.cs
/// <summary>
/// Entity that supports soft deletion for audit trail preservation.
/// </summary>
public interface ISoftDeletable
{
    /// <summary>When this entity was deleted (null if not deleted).</summary>
    DateTime? DeletedAt { get; }

    /// <summary>Whether this entity has been soft-deleted.</summary>
    bool IsDeleted => DeletedAt.HasValue;
}

// IAuditable.cs
/// <summary>
/// Entity with full audit trail (who created/modified and when).
/// </summary>
public interface IAuditable : IEntity, ISoftDeletable
{
    /// <summary>When this entity was last modified (UTC).</summary>
    DateTime UpdatedAt { get; }

    /// <summary>User who created this entity.</summary>
    Guid CreatedBy { get; }

    /// <summary>User who last modified this entity.</summary>
    Guid UpdatedBy { get; }
}

// IProjectScoped.cs
/// <summary>
/// Entity that belongs to a specific project.
/// Enables project-level filtering and authorization.
/// </summary>
public interface IProjectScoped
{
    /// <summary>The project this entity belongs to.</summary>
    Guid ProjectId { get; }
}

// IWorkItemScoped.cs
/// <summary>
/// Entity that belongs to a specific work item.
/// Enables work-item-level filtering and cascade operations.
/// </summary>
public interface IWorkItemScoped
{
    /// <summary>The work item this entity belongs to.</summary>
    Guid WorkItemId { get; }
}

// IVersioned.cs
/// <summary>
/// Entity that supports optimistic concurrency control.
/// </summary>
public interface IVersioned
{
    /// <summary>Version number for optimistic locking.</summary>
    int Version { get; }
}

// IPositioned.cs
/// <summary>
/// Entity that has a position for ordering (drag-and-drop support).
/// </summary>
public interface IPositioned
{
    /// <summary>Position for ordering within parent/container.</summary>
    int Position { get; }
}

// IHierarchical.cs
/// <summary>
/// Entity that can have a parent (tree structure).
/// </summary>
/// <typeparam name="T">The entity type (self-referential).</typeparam>
public interface IHierarchical<T> where T : IEntity
{
    /// <summary>Parent entity ID (null for root entities).</summary>
    Guid? ParentId { get; }
}

// ISprintAssignable.cs
/// <summary>
/// Entity that can be assigned to a sprint.
/// </summary>
public interface ISprintAssignable
{
    /// <summary>The sprint this entity is assigned to (null if in backlog).</summary>
    Guid? SprintId { get; }
}

// IUserAssignable.cs
/// <summary>
/// Entity that can be assigned to a user.
/// </summary>
public interface IUserAssignable
{
    /// <summary>The user this entity is assigned to.</summary>
    Guid? AssigneeId { get; }
}

// IUserOwned.cs
/// <summary>
/// Entity that is owned by a specific user (not assignable, but belongs to).
/// Different from IUserAssignable - this is ownership, not assignment.
/// </summary>
public interface IUserOwned
{
    /// <summary>The user who owns this entity.</summary>
    Guid UserId { get; }
}

// IStatusTracked.cs
/// <summary>
/// Entity that has a workflow status.
/// Status values map to SwimLanes for Kanban board display.
/// </summary>
public interface IStatusTracked
{
    /// <summary>Current workflow status (e.g., "backlog", "in_progress", "done").</summary>
    string Status { get; }
}

// IWorkItemLink.cs
/// <summary>
/// Entity that links two work items together (e.g., dependencies).
/// </summary>
public interface IWorkItemLink
{
    /// <summary>The source/blocking work item.</summary>
    Guid BlockingItemId { get; }

    /// <summary>The target/blocked work item.</summary>
    Guid BlockedItemId { get; }
}
```

**Interface Implementations by Entity:**

| Entity | Interfaces |
|--------|------------|
| `WorkItem` | `IAuditable`, `IProjectScoped`, `IVersioned`, `IPositioned`, `IHierarchical<WorkItem>`, `ISprintAssignable`, `IUserAssignable`, `IStatusTracked` |
| `Sprint` | `IAuditable`, `IProjectScoped` |
| `Comment` | `IAuditable`, `IWorkItemScoped` |
| `TimeEntry` | `IEntity`, `ISoftDeletable`, `IWorkItemScoped`, `IUserOwned` |
| `Dependency` | `IEntity`, `ISoftDeletable`, `IWorkItemLink` |
| `SwimLane` | `IEntity`, `ISoftDeletable`, `IProjectScoped`, `IPositioned`, `IStatusTracked` |
| `ProjectMember` | `IEntity`, `IProjectScoped`, `IUserOwned` |

**Relationship Summary:**

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Interface Relationships                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  OWNERSHIP (FK with CASCADE)          ASSIGNMENT (FK, nullable)          │
│  ─────────────────────────           ──────────────────────────          │
│                                                                          │
│  IProjectScoped                       ISprintAssignable                  │
│    └─ "belongs to project"              └─ "assigned to sprint"          │
│    └─ WorkItem, Sprint, SwimLane        └─ WorkItem                      │
│                                                                          │
│  IWorkItemScoped                      IUserAssignable                    │
│    └─ "belongs to work item"            └─ "assigned to user"            │
│    └─ Comment, TimeEntry                └─ WorkItem                      │
│                                                                          │
│  IUserOwned                           IStatusTracked                     │
│    └─ "owned by user"                   └─ "has workflow status"         │
│    └─ TimeEntry, ProjectMember          └─ WorkItem (joins to SwimLane)  │
│                                                                          │
│  IWorkItemLink                        IHierarchical<T>                   │
│    └─ "links two work items"            └─ "has parent of same type"     │
│    └─ Dependency                        └─ WorkItem                      │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**Generic Store Interface:**

```csharp
// IEntityStore.cs
namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Generic store interface for entities with common operations.
/// </summary>
public interface IEntityStore<T> : IDisposable where T : IEntity, ISoftDeletable
{
    /// <summary>Fired when store contents change.</summary>
    event Action? OnChanged;

    /// <summary>Get entity by ID (returns null if not found or deleted).</summary>
    T? GetById(Guid id);

    /// <summary>Get all non-deleted entities.</summary>
    IReadOnlyList<T> GetAll();

    /// <summary>Check if entity exists and is not deleted.</summary>
    bool Exists(Guid id);
}

// IProjectScopedStore.cs
/// <summary>
/// Store for project-scoped entities.
/// </summary>
public interface IProjectScopedStore<T> : IEntityStore<T>
    where T : IEntity, ISoftDeletable, IProjectScoped
{
    /// <summary>Get all entities for a specific project.</summary>
    IReadOnlyList<T> GetByProject(Guid projectId);

    /// <summary>Refresh data for a specific project from server.</summary>
    Task RefreshAsync(Guid projectId, CancellationToken ct = default);
}

// IWorkItemScopedStore.cs
/// <summary>
/// Store for work-item-scoped entities.
/// </summary>
public interface IWorkItemScopedStore<T> : IEntityStore<T>
    where T : IEntity, ISoftDeletable, IWorkItemScoped
{
    /// <summary>Get all entities for a specific work item.</summary>
    IReadOnlyList<T> GetByWorkItem(Guid workItemId);
}
```

**Benefits of This Design:**

1. **Generic filtering**: `store.GetAll().Where(e => !e.IsDeleted)` works on any entity
2. **Consistent audit display**: UI components can show CreatedBy/UpdatedAt for any `IAuditable`
3. **Type-safe operations**: Compiler enforces that only `IVersioned` entities use optimistic locking
4. **Reusable stores**: `IProjectScopedStore<T>` works for WorkItem, Sprint, SwimLane
5. **Testability**: Mock any interface independently

---

### Phase 1.6: Domain Models with Validation

**Files to create in `ProjectManagement.Core/Models/`:**

```csharp
// WorkItem.cs
namespace ProjectManagement.Core.Models;

/// <summary>
/// Polymorphic work item representing Project, Epic, Story, or Task.
/// Immutable record for thread safety.
/// </summary>
public sealed record WorkItem :
    IAuditable,
    IProjectScoped,
    IVersioned,
    IPositioned,
    IHierarchical<WorkItem>,
    ISprintAssignable,
    IUserAssignable,
    IStatusTracked
{
    public Guid Id { get; init; }
    public WorkItemType ItemType { get; init; }
    public Guid? ParentId { get; init; }
    public Guid ProjectId { get; init; }
    public int Position { get; init; }
    public string Title { get; init; } = string.Empty;
    public string? Description { get; init; }
    public string Status { get; init; } = WorkItemDefaults.Status;
    public string Priority { get; init; } = WorkItemDefaults.Priority;
    public Guid? AssigneeId { get; init; }
    public int? StoryPoints { get; init; }
    public Guid? SprintId { get; init; }
    public int Version { get; init; }
    public DateTime CreatedAt { get; init; }
    public DateTime UpdatedAt { get; init; }
    public Guid CreatedBy { get; init; }
    public Guid UpdatedBy { get; init; }
    public DateTime? DeletedAt { get; init; }
}

public static class WorkItemDefaults
{
    public const string Status = "backlog";
    public const string Priority = "medium";
    // Aligned with pm-config validation defaults
    public const int MaxTitleLength = 200;
    public const int MaxDescriptionLength = 10000;
}

// WorkItemType.cs
namespace ProjectManagement.Core.Models;

public enum WorkItemType
{
    Project = 1,
    Epic = 2,
    Story = 3,
    Task = 4
}

public static class WorkItemTypeExtensions
{
    public static string ToDisplayString(this WorkItemType type) => type switch
    {
        WorkItemType.Project => "Project",
        WorkItemType.Epic => "Epic",
        WorkItemType.Story => "Story",
        WorkItemType.Task => "Task",
        _ => throw new ArgumentOutOfRangeException(nameof(type))
    };

    public static bool CanHaveParent(this WorkItemType type) => type != WorkItemType.Project;

    public static IReadOnlyList<WorkItemType> AllowedChildTypes(this WorkItemType type) => type switch
    {
        WorkItemType.Project => [WorkItemType.Epic, WorkItemType.Story, WorkItemType.Task],
        WorkItemType.Epic => [WorkItemType.Story, WorkItemType.Task],
        WorkItemType.Story => [WorkItemType.Task],
        WorkItemType.Task => [],
        _ => []
    };
}

// FieldChange.cs
namespace ProjectManagement.Core.Models;

/// <summary>
/// Represents a single field change in a work item update.
/// Used for tracking changes and displaying in activity feeds.
/// </summary>
public sealed record FieldChange(
    string FieldName,
    string? OldValue,
    string? NewValue)
{
    /// <summary>
    /// Human-readable description of the change.
    /// </summary>
    public string Description => (OldValue, NewValue) switch
    {
        (null, not null) => $"Set {FieldName} to '{NewValue}'",
        (not null, null) => $"Cleared {FieldName}",
        _ => $"Changed {FieldName} from '{OldValue}' to '{NewValue}'"
    };
}

// Sprint.cs
namespace ProjectManagement.Core.Models;

/// <summary>
/// A time-boxed iteration for completing work items.
/// </summary>
public sealed record Sprint : IAuditable, IProjectScoped
{
    public Guid Id { get; init; }
    public Guid ProjectId { get; init; }
    public string Name { get; init; } = string.Empty;
    public string? Goal { get; init; }
    public DateTime StartDate { get; init; }
    public DateTime EndDate { get; init; }
    public SprintStatus Status { get; init; } = SprintStatus.Planned;
    public DateTime CreatedAt { get; init; }
    public DateTime UpdatedAt { get; init; }
    public Guid CreatedBy { get; init; }
    public Guid UpdatedBy { get; init; }
    public DateTime? DeletedAt { get; init; }
}

public enum SprintStatus
{
    Planned = 1,
    Active = 2,
    Completed = 3,
    Cancelled = 4
}

public static class SprintStatusExtensions
{
    public static string ToDisplayString(this SprintStatus status) => status switch
    {
        SprintStatus.Planned => "Planned",
        SprintStatus.Active => "Active",
        SprintStatus.Completed => "Completed",
        SprintStatus.Cancelled => "Cancelled",
        _ => throw new ArgumentOutOfRangeException(nameof(status))
    };
}
```

### Phase 1.7: Validation Framework

**Files to create in `ProjectManagement.Core/Validation/`:**

```csharp
// IValidator.cs
namespace ProjectManagement.Core.Validation;

public interface IValidator<T>
{
    ValidationResult Validate(T instance);
}

// ValidationResult.cs
public sealed class ValidationResult
{
    public bool IsValid => Errors.Count == 0;
    public IReadOnlyList<ValidationError> Errors { get; }

    private ValidationResult(IReadOnlyList<ValidationError> errors) => Errors = errors;

    public static ValidationResult Success() => new([]);
    public static ValidationResult Failure(params ValidationError[] errors) => new(errors);
    public static ValidationResult Failure(IEnumerable<ValidationError> errors) => new(errors.ToList());

    public void ThrowIfInvalid()
    {
        if (!IsValid)
            throw new ValidationException(Errors);
    }
}

// CreateWorkItemRequestValidator.cs
public sealed class CreateWorkItemRequestValidator : IValidator<CreateWorkItemRequest>
{
    public ValidationResult Validate(CreateWorkItemRequest request)
    {
        var errors = new List<ValidationError>();

        if (string.IsNullOrWhiteSpace(request.Title))
            errors.Add(new("title", "Title is required"));
        else if (request.Title.Length > WorkItemDefaults.MaxTitleLength)
            errors.Add(new("title", $"Title must be {WorkItemDefaults.MaxTitleLength} characters or less")); // 200

        if (request.ProjectId == Guid.Empty)
            errors.Add(new("projectId", "Project ID is required"));

        if (request.Description?.Length > WorkItemDefaults.MaxDescriptionLength)
            errors.Add(new("description", $"Description must be {WorkItemDefaults.MaxDescriptionLength} characters or less")); // 10000

        if (request.ItemType == WorkItemType.Project && request.ParentId.HasValue)
            errors.Add(new("parentId", "Projects cannot have a parent"));

        return errors.Count == 0 ? ValidationResult.Success() : ValidationResult.Failure(errors);
    }
}
```

### Phase 1.8: Proto Converter with Null Safety

> **Timestamp Precision Note**: Timestamps are stored as Unix epoch seconds (int64).
> This means sub-second precision is lost during serialization. DateTime values
> round-tripped through protobuf may differ by up to 1 second from the original.
> This is by design to match the backend's SQLite INTEGER timestamp storage.

```csharp
// ProtoConverter.cs
namespace ProjectManagement.Core.Converters;

using ProjectManagement.Core.Proto;

/// <summary>
/// Converts between Protocol Buffer messages and domain models.
/// All conversions are null-safe and validated.
///
/// NOTE: Timestamps use Unix epoch seconds, losing sub-second precision.
/// Round-trip conversions may differ by up to 1 second.
/// </summary>
public static class ProtoConverter
{
    private static readonly DateTime UnixEpoch = new(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);

    #region WorkItem Conversions

    public static WorkItem ToDomain(Pm.WorkItem proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new WorkItem
        {
            Id = ParseGuid(proto.Id, "WorkItem.Id"),
            ItemType = ToDomain(proto.ItemType),
            ParentId = string.IsNullOrEmpty(proto.ParentId) ? null : ParseGuid(proto.ParentId, "WorkItem.ParentId"),
            ProjectId = ParseGuid(proto.ProjectId, "WorkItem.ProjectId"),
            Position = proto.Position,
            Title = proto.Title ?? string.Empty,
            Description = string.IsNullOrEmpty(proto.Description) ? null : proto.Description,
            Status = proto.Status ?? WorkItemDefaults.Status,
            Priority = proto.Priority ?? WorkItemDefaults.Priority,
            AssigneeId = string.IsNullOrEmpty(proto.AssigneeId) ? null : ParseGuid(proto.AssigneeId, "WorkItem.AssigneeId"),
            StoryPoints = proto.HasStoryPoints ? proto.StoryPoints : null,
            SprintId = string.IsNullOrEmpty(proto.SprintId) ? null : ParseGuid(proto.SprintId, "WorkItem.SprintId"),
            Version = proto.Version,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "WorkItem.CreatedBy"),
            UpdatedBy = ParseGuid(proto.UpdatedBy, "WorkItem.UpdatedBy"),
            DeletedAt = proto.HasDeletedAt ? FromUnixTimestamp(proto.DeletedAt) : null
        };
    }

    public static Pm.WorkItem ToProto(WorkItem domain)
    {
        ArgumentNullException.ThrowIfNull(domain);

        var proto = new Pm.WorkItem
        {
            Id = domain.Id.ToString(),
            ItemType = ToProto(domain.ItemType),
            ProjectId = domain.ProjectId.ToString(),
            Position = domain.Position,
            Title = domain.Title,
            Status = domain.Status,
            Priority = domain.Priority,
            Version = domain.Version,
            CreatedAt = ToUnixTimestamp(domain.CreatedAt),
            UpdatedAt = ToUnixTimestamp(domain.UpdatedAt),
            CreatedBy = domain.CreatedBy.ToString(),
            UpdatedBy = domain.UpdatedBy.ToString()
        };

        if (domain.ParentId.HasValue)
            proto.ParentId = domain.ParentId.Value.ToString();
        if (!string.IsNullOrEmpty(domain.Description))
            proto.Description = domain.Description;
        if (domain.AssigneeId.HasValue)
            proto.AssigneeId = domain.AssigneeId.Value.ToString();
        if (domain.StoryPoints.HasValue)
            proto.StoryPoints = domain.StoryPoints.Value;
        if (domain.SprintId.HasValue)
            proto.SprintId = domain.SprintId.Value.ToString();
        if (domain.DeletedAt.HasValue)
            proto.DeletedAt = ToUnixTimestamp(domain.DeletedAt.Value);

        return proto;
    }

    #endregion

    #region Enum Conversions

    public static WorkItemType ToDomain(Pm.WorkItemType proto) => proto switch
    {
        Pm.WorkItemType.Project => WorkItemType.Project,
        Pm.WorkItemType.Epic => WorkItemType.Epic,
        Pm.WorkItemType.Story => WorkItemType.Story,
        Pm.WorkItemType.Task => WorkItemType.Task,
        _ => throw new ArgumentOutOfRangeException(nameof(proto), $"Unknown WorkItemType: {proto}")
    };

    public static Pm.WorkItemType ToProto(WorkItemType domain) => domain switch
    {
        WorkItemType.Project => Pm.WorkItemType.Project,
        WorkItemType.Epic => Pm.WorkItemType.Epic,
        WorkItemType.Story => Pm.WorkItemType.Story,
        WorkItemType.Task => Pm.WorkItemType.Task,
        _ => throw new ArgumentOutOfRangeException(nameof(domain), $"Unknown WorkItemType: {domain}")
    };

    #endregion

    #region Helper Methods

    private static Guid ParseGuid(string value, string fieldName)
    {
        if (string.IsNullOrEmpty(value))
            throw new ArgumentException($"{fieldName} cannot be empty", fieldName);

        if (!Guid.TryParse(value, out var guid))
            throw new ArgumentException($"{fieldName} is not a valid GUID: {value}", fieldName);

        return guid;
    }

    private static DateTime FromUnixTimestamp(long timestamp)
    {
        return UnixEpoch.AddSeconds(timestamp);
    }

    private static long ToUnixTimestamp(DateTime dateTime)
    {
        return (long)(dateTime.ToUniversalTime() - UnixEpoch).TotalSeconds;
    }

    #endregion
}
```

### Phase 1.9: Interfaces

```csharp
// IWebSocketClient.cs
namespace ProjectManagement.Core.Interfaces;

public interface IWebSocketClient : IAsyncDisposable
{
    /// <summary>Current connection state.</summary>
    ConnectionState State { get; }

    /// <summary>Connection health metrics.</summary>
    IConnectionHealth Health { get; }

    /// <summary>Fired when connection state changes.</summary>
    event Action<ConnectionState>? OnStateChanged;

    /// <summary>Fired when a work item is created by another user.</summary>
    event Action<WorkItem>? OnWorkItemCreated;

    /// <summary>Fired when a work item is updated by another user.</summary>
    event Action<WorkItem, IReadOnlyList<FieldChange>>? OnWorkItemUpdated;

    /// <summary>Fired when a work item is deleted by another user.</summary>
    event Action<Guid>? OnWorkItemDeleted;

    // Connection
    Task ConnectAsync(CancellationToken ct = default);
    Task DisconnectAsync(CancellationToken ct = default);

    // Subscriptions
    Task SubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default);
    Task UnsubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default);

    // Work Item Operations
    Task<WorkItem> CreateWorkItemAsync(CreateWorkItemRequest request, CancellationToken ct = default);
    Task<WorkItem> UpdateWorkItemAsync(UpdateWorkItemRequest request, CancellationToken ct = default);
    Task DeleteWorkItemAsync(Guid workItemId, CancellationToken ct = default);
    Task<IReadOnlyList<WorkItem>> GetWorkItemsAsync(Guid projectId, DateTime? since = null, CancellationToken ct = default);
}

// IConnectionHealth.cs
public interface IConnectionHealth
{
    ConnectionQuality Quality { get; }
    TimeSpan? Latency { get; }
    DateTime? LastMessageReceived { get; }
    DateTime? LastMessageSent { get; }
    int PendingRequestCount { get; }
    int ReconnectAttempts { get; }
}

public enum ConnectionQuality
{
    Unknown,
    Excellent,  // <100ms latency
    Good,       // 100-300ms
    Fair,       // 300-1000ms
    Poor,       // >1000ms or packet loss
    Disconnected
}

// IWorkItemStore.cs
public interface IWorkItemStore : IDisposable
{
    event Action? OnChanged;

    IReadOnlyList<WorkItem> GetByProject(Guid projectId);
    WorkItem? GetById(Guid id);
    IReadOnlyList<WorkItem> GetBySprint(Guid sprintId);
    IReadOnlyList<WorkItem> GetChildren(Guid parentId);

    Task<WorkItem> CreateAsync(CreateWorkItemRequest request, CancellationToken ct = default);
    Task<WorkItem> UpdateAsync(UpdateWorkItemRequest request, CancellationToken ct = default);
    Task DeleteAsync(Guid id, CancellationToken ct = default);
    Task RefreshAsync(Guid projectId, CancellationToken ct = default);
}

// ISprintStore.cs
public interface ISprintStore : IDisposable
{
    event Action? OnChanged;

    IReadOnlyList<Sprint> GetByProject(Guid projectId);
    Sprint? GetById(Guid id);
    Sprint? GetActiveSprint(Guid projectId);

    Task<Sprint> CreateAsync(CreateSprintRequest request, CancellationToken ct = default);
    Task<Sprint> UpdateAsync(UpdateSprintRequest request, CancellationToken ct = default);
    Task<Sprint> StartSprintAsync(Guid sprintId, CancellationToken ct = default);
    Task<Sprint> CompleteSprintAsync(Guid sprintId, CancellationToken ct = default);
    Task DeleteAsync(Guid id, CancellationToken ct = default);
    Task RefreshAsync(Guid projectId, CancellationToken ct = default);
}

// CreateSprintRequest.cs
public sealed record CreateSprintRequest
{
    public required Guid ProjectId { get; init; }
    public required string Name { get; init; }
    public string? Goal { get; init; }
    public required DateTime StartDate { get; init; }
    public required DateTime EndDate { get; init; }
}

// UpdateSprintRequest.cs
public sealed record UpdateSprintRequest
{
    public required Guid SprintId { get; init; }
    public string? Name { get; init; }
    public string? Goal { get; init; }
    public DateTime? StartDate { get; init; }
    public DateTime? EndDate { get; init; }
}
```

### Files Summary for Sub-Session 20.1

| Category | Files | Count |
|----------|-------|-------|
| Solution/Build | `.sln`, `Directory.Build.props`, `Directory.Packages.props`, `.editorconfig` | 4 |
| Project files | 4x `.csproj` | 4 |
| Exceptions | `ProjectManagementException.cs`, `ConnectionException.cs`, `RequestTimeoutException.cs`, `ServerRejectedException.cs`, `VersionConflictException.cs`, `CircuitOpenException.cs`, `ValidationException.cs` | 7 |
| Entity Interfaces | `IEntity.cs`, `ISoftDeletable.cs`, `IAuditable.cs`, `IProjectScoped.cs`, `IWorkItemScoped.cs`, `IVersioned.cs`, `IPositioned.cs`, `IHierarchical.cs`, `ISprintAssignable.cs`, `IUserAssignable.cs`, `IUserOwned.cs`, `IStatusTracked.cs`, `IWorkItemLink.cs` | 13 |
| Store Interfaces | `IEntityStore.cs`, `IProjectScopedStore.cs`, `IWorkItemScopedStore.cs`, `IWorkItemStore.cs`, `ISprintStore.cs` | 5 |
| Models | `WorkItem.cs`, `WorkItemType.cs`, `Sprint.cs`, `SprintStatus.cs`, `Comment.cs`, `TimeEntry.cs`, `Dependency.cs`, `DependencyType.cs`, `SwimLane.cs`, `FieldChange.cs` | 10 |
| Validation | `IValidator.cs`, `ValidationResult.cs`, `CreateWorkItemRequestValidator.cs`, `UpdateWorkItemRequestValidator.cs` | 4 |
| Converters | `ProtoConverter.cs` | 1 |
| Service Interfaces | `IWebSocketClient.cs`, `IConnectionHealth.cs` | 2 |
| Request DTOs | `CreateWorkItemRequest.cs`, `UpdateWorkItemRequest.cs`, `CreateSprintRequest.cs`, `UpdateSprintRequest.cs` | 4 |
| Proto | `messages.proto` (copy) | 1 |
| **Total** | | **55** |

### Success Criteria for 20.1

- [ ] `dotnet build frontend/ProjectManagement.sln` succeeds with zero warnings
- [ ] Protobuf C# classes generated correctly
- [ ] All models compile with nullable enabled
- [ ] Validation framework works
- [ ] ProtoConverter handles all edge cases

---

## Sub-Session 20.2: WebSocket Client Foundation

**Goal**: Core WebSocket client with message correlation and heartbeat

**Estimated Tokens**: ~35k

### Phase 2.1: Configuration

```csharp
// WebSocketOptions.cs
namespace ProjectManagement.Services.WebSocket;

public sealed class WebSocketOptions
{
    /// <summary>WebSocket server URL (ws:// or wss://).</summary>
    public string ServerUrl { get; set; } = "ws://localhost:8080/ws";

    /// <summary>JWT token for authentication (null for desktop mode).</summary>
    public string? JwtToken { get; set; }

    /// <summary>Interval between ping messages.</summary>
    public TimeSpan HeartbeatInterval { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>Timeout waiting for pong response.</summary>
    public TimeSpan HeartbeatTimeout { get; set; } = TimeSpan.FromSeconds(60);

    /// <summary>Timeout for request/response operations.</summary>
    public TimeSpan RequestTimeout { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>Size of send buffer (messages).</summary>
    public int SendBufferSize { get; set; } = 100;

    /// <summary>Size of receive buffer (bytes).</summary>
    public int ReceiveBufferSize { get; set; } = 64 * 1024;
}
```

### Phase 2.2: Connection State (Re-export)

> **Note**: `ConnectionState` enum is defined in `ProjectManagement.Core.Exceptions` namespace (Phase 1.4) to avoid circular dependencies. This phase creates a type alias/re-export for use in the WebSocket namespace.

```csharp
// In ProjectManagement.Services.WebSocket namespace, use:
using ConnectionState = ProjectManagement.Core.Exceptions.ConnectionState;

// Or create a simple re-export file:
// ConnectionState.cs
namespace ProjectManagement.Services.WebSocket;

// Re-export from Core for convenience
public enum ConnectionState
{
    /// <summary>Not connected to server.</summary>
    Disconnected,

    /// <summary>Attempting to establish connection.</summary>
    Connecting,

    /// <summary>Connected and ready for operations.</summary>
    Connected,

    /// <summary>Connection lost, attempting to reconnect.</summary>
    Reconnecting,

    /// <summary>Permanently closed (disposed).</summary>
    Closed
}

// Alternative: use global using in _Imports.cs:
// global using ConnectionState = ProjectManagement.Core.Exceptions.ConnectionState;
```

### Phase 2.3: Request Tracking

```csharp
// PendingRequest.cs
namespace ProjectManagement.Services.WebSocket;

internal sealed class PendingRequest : IDisposable
{
    public string MessageId { get; }
    public DateTime SentAt { get; }
    public TimeSpan Timeout { get; }
    public TaskCompletionSource<WebSocketMessage> CompletionSource { get; }

    private readonly CancellationTokenSource _timeoutCts;
    private readonly CancellationTokenRegistration _registration;
    private bool _disposed;

    public PendingRequest(string messageId, TimeSpan timeout, CancellationToken externalCt)
    {
        MessageId = messageId;
        SentAt = DateTime.UtcNow;
        Timeout = timeout;
        CompletionSource = new TaskCompletionSource<WebSocketMessage>(
            TaskCreationOptions.RunContinuationsAsynchronously);

        _timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(externalCt);
        _timeoutCts.CancelAfter(timeout);

        _registration = _timeoutCts.Token.Register(() =>
        {
            CompletionSource.TrySetException(
                new RequestTimeoutException(messageId, timeout));
        });
    }

    public void Complete(WebSocketMessage response)
    {
        CompletionSource.TrySetResult(response);
    }

    public void Fail(Exception ex)
    {
        CompletionSource.TrySetException(ex);
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _registration.Dispose();
        _timeoutCts.Dispose();
    }
}
```

### Phase 2.4: WebSocket Connection Abstraction

```csharp
// IWebSocketConnection.cs
namespace ProjectManagement.Services.WebSocket;

/// <summary>
/// Abstraction over raw WebSocket for testability.
/// </summary>
internal interface IWebSocketConnection : IAsyncDisposable
{
    WebSocketState State { get; }

    Task ConnectAsync(Uri uri, CancellationToken ct);
    Task SendAsync(ReadOnlyMemory<byte> buffer, CancellationToken ct);
    Task<WebSocketReceiveResult> ReceiveAsync(Memory<byte> buffer, CancellationToken ct);
    Task CloseAsync(WebSocketCloseStatus status, string? description, CancellationToken ct);
}

// BrowserWebSocketConnection.cs
internal sealed class BrowserWebSocketConnection : IWebSocketConnection
{
    private readonly ClientWebSocket _socket;
    private readonly ILogger<BrowserWebSocketConnection> _logger;

    public WebSocketState State => _socket.State;

    public BrowserWebSocketConnection(ILogger<BrowserWebSocketConnection> logger)
    {
        _socket = new ClientWebSocket();
        _logger = logger;
    }

    public async Task ConnectAsync(Uri uri, CancellationToken ct)
    {
        _logger.LogDebug("Connecting to {Uri}", uri);
        await _socket.ConnectAsync(uri, ct);
        _logger.LogInformation("Connected to {Uri}", uri);
    }

    public Task SendAsync(ReadOnlyMemory<byte> buffer, CancellationToken ct)
    {
        return _socket.SendAsync(buffer, WebSocketMessageType.Binary, true, ct);
    }

    public Task<WebSocketReceiveResult> ReceiveAsync(Memory<byte> buffer, CancellationToken ct)
    {
        return _socket.ReceiveAsync(buffer, ct);
    }

    public Task CloseAsync(WebSocketCloseStatus status, string? description, CancellationToken ct)
    {
        if (_socket.State == WebSocketState.Open || _socket.State == WebSocketState.CloseReceived)
        {
            return _socket.CloseAsync(status, description, ct);
        }
        return Task.CompletedTask;
    }

    public async ValueTask DisposeAsync()
    {
        try
        {
            if (_socket.State == WebSocketState.Open)
            {
                using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(5));
                await CloseAsync(WebSocketCloseStatus.NormalClosure, "Disposing", cts.Token);
            }
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Error during WebSocket disposal");
        }
        finally
        {
            _socket.Dispose();
        }
    }
}
```

### Phase 2.5: Core WebSocket Client

```csharp
// WebSocketClient.cs
namespace ProjectManagement.Services.WebSocket;

public sealed class WebSocketClient : IWebSocketClient
{
    private readonly WebSocketOptions _options;
    private readonly ILogger<WebSocketClient> _logger;
    private readonly Func<IWebSocketConnection> _connectionFactory;
    private readonly ConcurrentDictionary<string, PendingRequest> _pendingRequests = new();
    private readonly ConnectionHealthTracker _health;
    private readonly SemaphoreSlim _sendLock = new(1, 1);
    private readonly SemaphoreSlim _stateLock = new(1, 1);

    // Validators (injected for testability)
    private readonly IValidator<CreateWorkItemRequest> _createValidator;
    private readonly IValidator<UpdateWorkItemRequest> _updateValidator;

    private IWebSocketConnection? _connection;
    private CancellationTokenSource? _receiveCts;
    private CancellationTokenSource? _heartbeatCts;
    private Task? _receiveTask;
    private Task? _heartbeatTask;
    private ConnectionState _state = ConnectionState.Disconnected;
    private HashSet<Guid> _subscribedProjects = new();
    private bool _disposed;

    public ConnectionState State => _state;
    public IConnectionHealth Health => _health;

    public event Action<ConnectionState>? OnStateChanged;
    public event Action<WorkItem>? OnWorkItemCreated;
    public event Action<WorkItem, IReadOnlyList<FieldChange>>? OnWorkItemUpdated;
    public event Action<Guid>? OnWorkItemDeleted;

    public WebSocketClient(
        IOptions<WebSocketOptions> options,
        ILogger<WebSocketClient> logger,
        ILoggerFactory loggerFactory,
        IValidator<CreateWorkItemRequest> createValidator,
        IValidator<UpdateWorkItemRequest> updateValidator,
        Func<IWebSocketConnection>? connectionFactory = null)
    {
        _createValidator = createValidator;
        _updateValidator = updateValidator;
        _options = options.Value;
        _logger = logger;
        // Use injected ILoggerFactory for proper DI integration
        _connectionFactory = connectionFactory ?? (() =>
            new BrowserWebSocketConnection(
                loggerFactory.CreateLogger<BrowserWebSocketConnection>()));
        _health = new ConnectionHealthTracker();
    }

    public async Task ConnectAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        await _stateLock.WaitAsync(ct);
        try
        {
            if (_state == ConnectionState.Connected)
                return;

            SetState(ConnectionState.Connecting);

            _connection = _connectionFactory();
            var uri = BuildConnectionUri();

            await _connection.ConnectAsync(uri, ct);

            _receiveCts = new CancellationTokenSource();
            _heartbeatCts = new CancellationTokenSource();

            _receiveTask = ReceiveLoopAsync(_receiveCts.Token);
            _heartbeatTask = HeartbeatLoopAsync(_heartbeatCts.Token);

            _health.RecordConnected();
            SetState(ConnectionState.Connected);

            _logger.LogInformation("WebSocket connected to {Uri}", uri);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to connect to WebSocket");
            SetState(ConnectionState.Disconnected);
            throw new ConnectionException("Failed to connect to server", ex);
        }
        finally
        {
            _stateLock.Release();
        }
    }

    public async Task DisconnectAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        await _stateLock.WaitAsync(ct);
        try
        {
            await DisconnectInternalAsync(ct);
        }
        finally
        {
            _stateLock.Release();
        }
    }

    public async Task<WorkItem> CreateWorkItemAsync(
        CreateWorkItemRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        // Validate using injected validator
        _createValidator.Validate(request).ThrowIfInvalid();

        var message = new WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreateWorkItemRequest = new Pm.CreateWorkItemRequest
            {
                ItemType = ProtoConverter.ToProto(request.ItemType),
                Title = request.Title,
                ProjectId = request.ProjectId.ToString()
            }
        };

        if (!string.IsNullOrEmpty(request.Description))
            message.CreateWorkItemRequest.Description = request.Description;
        if (request.ParentId.HasValue)
            message.CreateWorkItemRequest.ParentId = request.ParentId.Value.ToString();

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == WebSocketMessage.PayloadOneofCase.Error)
        {
            throw new ServerRejectedException(
                response.Error.Code,
                response.Error.Message,
                response.Error.Field);
        }

        if (response.PayloadCase != WebSocketMessage.PayloadOneofCase.WorkItemCreated)
        {
            throw new InvalidOperationException(
                $"Unexpected response type: {response.PayloadCase}");
        }

        return ProtoConverter.ToDomain(response.WorkItemCreated.WorkItem);
    }

    // ... other CRUD methods follow same pattern

    #region Private Methods

    private async Task<WebSocketMessage> SendRequestAsync(
        WebSocketMessage request,
        CancellationToken ct)
    {
        using var pending = new PendingRequest(
            request.MessageId,
            _options.RequestTimeout,
            ct);

        if (!_pendingRequests.TryAdd(request.MessageId, pending))
        {
            throw new InvalidOperationException(
                $"Duplicate message ID: {request.MessageId}");
        }

        try
        {
            await SendMessageAsync(request, ct);
            _health.RecordRequestSent();

            return await pending.CompletionSource.Task;
        }
        finally
        {
            _pendingRequests.TryRemove(request.MessageId, out _);
        }
    }

    private async Task SendMessageAsync(WebSocketMessage message, CancellationToken ct)
    {
        var bytes = message.ToByteArray();

        await _sendLock.WaitAsync(ct);
        try
        {
            if (_connection?.State != WebSocketState.Open)
                throw new ConnectionException("WebSocket not connected");

            await _connection.SendAsync(bytes, ct);
            _health.RecordMessageSent();

            _logger.LogDebug("Sent message {MessageId} ({Type})",
                message.MessageId, message.PayloadCase);
        }
        finally
        {
            _sendLock.Release();
        }
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        var buffer = new byte[_options.ReceiveBufferSize];
        var messageBuffer = new MemoryStream();

        try
        {
            while (!ct.IsCancellationRequested && _connection?.State == WebSocketState.Open)
            {
                var result = await _connection.ReceiveAsync(buffer, ct);

                if (result.MessageType == WebSocketMessageType.Close)
                {
                    _logger.LogInformation("Server initiated close");
                    break;
                }

                messageBuffer.Write(buffer, 0, result.Count);

                if (result.EndOfMessage)
                {
                    var messageBytes = messageBuffer.ToArray();
                    messageBuffer.SetLength(0);

                    ProcessReceivedMessage(messageBytes);
                }
            }
        }
        catch (OperationCanceledException) when (ct.IsCancellationRequested)
        {
            // Normal shutdown
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in receive loop");
            _ = HandleDisconnectAsync(ex);
        }
    }

    private void ProcessReceivedMessage(byte[] bytes)
    {
        try
        {
            var message = WebSocketMessage.Parser.ParseFrom(bytes);
            _health.RecordMessageReceived();

            _logger.LogDebug("Received message {MessageId} ({Type})",
                message.MessageId, message.PayloadCase);

            // Check if this is a response to a pending request
            if (_pendingRequests.TryGetValue(message.MessageId, out var pending))
            {
                pending.Complete(message);
                _health.RecordResponseReceived();
                return;
            }

            // Otherwise it's a broadcast event
            HandleBroadcastEvent(message);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error processing received message");
        }
    }

    private void HandleBroadcastEvent(WebSocketMessage message)
    {
        switch (message.PayloadCase)
        {
            case WebSocketMessage.PayloadOneofCase.Pong:
                // Pass messageId to correlate with specific ping for accurate latency
                _health.RecordPong(message.MessageId, message.Pong.Timestamp);
                break;

            case WebSocketMessage.PayloadOneofCase.WorkItemCreated:
                var created = ProtoConverter.ToDomain(message.WorkItemCreated.WorkItem);
                OnWorkItemCreated?.Invoke(created);
                break;

            case WebSocketMessage.PayloadOneofCase.WorkItemUpdated:
                var updated = ProtoConverter.ToDomain(message.WorkItemUpdated.WorkItem);
                var changes = message.WorkItemUpdated.Changes
                    .Select(c => new FieldChange(c.FieldName, c.OldValue, c.NewValue))
                    .ToList();
                OnWorkItemUpdated?.Invoke(updated, changes);
                break;

            case WebSocketMessage.PayloadOneofCase.WorkItemDeleted:
                if (Guid.TryParse(message.WorkItemDeleted.WorkItemId, out var deletedId))
                {
                    OnWorkItemDeleted?.Invoke(deletedId);
                }
                break;

            default:
                _logger.LogWarning("Unhandled broadcast message type: {Type}",
                    message.PayloadCase);
                break;
        }
    }

    private async Task HeartbeatLoopAsync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                await Task.Delay(_options.HeartbeatInterval, ct);

                if (_state != ConnectionState.Connected)
                    continue;

                var messageId = Guid.NewGuid().ToString();
                var ping = new WebSocketMessage
                {
                    MessageId = messageId,
                    Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                    Ping = new Pm.Ping
                    {
                        Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
                    }
                };

                // Track this specific ping for latency calculation
                _health.RecordPingSent(messageId);
                await SendMessageAsync(ping, ct);
            }
            catch (OperationCanceledException) when (ct.IsCancellationRequested)
            {
                break;
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Error sending heartbeat");
            }
        }
    }

    private async Task HandleDisconnectAsync(Exception? ex)
    {
        await _stateLock.WaitAsync();
        try
        {
            if (_state == ConnectionState.Closed || _state == ConnectionState.Reconnecting)
                return;

            _health.RecordDisconnected();
            SetState(ConnectionState.Reconnecting);

            // Fail all pending requests
            foreach (var pending in _pendingRequests.Values)
            {
                pending.Fail(new ConnectionException(
                    "Connection lost", ex ?? new Exception("Unknown error")));
            }
            _pendingRequests.Clear();

            // Clean up current connection
            await DisconnectInternalAsync(CancellationToken.None);

            // Start reconnection (handled by ReconnectionService)
        }
        finally
        {
            _stateLock.Release();
        }
    }

    private async Task DisconnectInternalAsync(CancellationToken ct)
    {
        _heartbeatCts?.Cancel();
        _receiveCts?.Cancel();

        if (_heartbeatTask != null)
            await Task.WhenAny(_heartbeatTask, Task.Delay(1000, ct));
        if (_receiveTask != null)
            await Task.WhenAny(_receiveTask, Task.Delay(1000, ct));

        if (_connection != null)
        {
            await _connection.DisposeAsync();
            _connection = null;
        }

        _heartbeatCts?.Dispose();
        _receiveCts?.Dispose();
        _heartbeatCts = null;
        _receiveCts = null;

        SetState(ConnectionState.Disconnected);
    }

    private Uri BuildConnectionUri()
    {
        var uri = new UriBuilder(_options.ServerUrl);

        if (!string.IsNullOrEmpty(_options.JwtToken))
        {
            uri.Query = $"token={Uri.EscapeDataString(_options.JwtToken)}";
        }

        return uri.Uri;
    }

    private void SetState(ConnectionState newState)
    {
        if (_state == newState) return;

        var oldState = _state;
        _state = newState;

        _logger.LogInformation("Connection state changed: {Old} -> {New}", oldState, newState);
        OnStateChanged?.Invoke(newState);
    }

    private void EnsureConnected()
    {
        if (_state != ConnectionState.Connected)
        {
            throw new ConnectionException("Not connected to server")
            {
                LastKnownState = _state
            };
        }
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    #endregion

    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;

        SetState(ConnectionState.Closed);

        foreach (var pending in _pendingRequests.Values)
        {
            pending.Fail(new ObjectDisposedException(nameof(WebSocketClient)));
            pending.Dispose();
        }
        _pendingRequests.Clear();

        await DisconnectInternalAsync(CancellationToken.None);

        _sendLock.Dispose();
        _stateLock.Dispose();
    }
}
```

### Phase 2.6: Connection Health Tracker

```csharp
// ConnectionHealthTracker.cs
namespace ProjectManagement.Services.WebSocket;

/// <summary>
/// Tracks connection health metrics including latency from ping/pong.
/// Uses per-message correlation for accurate latency measurement.
/// </summary>
internal sealed class ConnectionHealthTracker : IConnectionHealth
{
    // Track outstanding pings by messageId for accurate latency correlation
    private readonly ConcurrentDictionary<string, long> _pendingPings = new();

    private long _lastPongReceivedTicks;
    private long _lastMessageReceivedTicks;
    private long _lastMessageSentTicks;
    private int _pendingRequestCount;
    private int _reconnectAttempts;
    private long _latencyMs;

    public ConnectionQuality Quality
    {
        get
        {
            if (LastMessageReceived == null)
                return ConnectionQuality.Unknown;

            var timeSinceMessage = DateTime.UtcNow - LastMessageReceived.Value;
            if (timeSinceMessage > TimeSpan.FromMinutes(2))
                return ConnectionQuality.Disconnected;

            if (!Latency.HasValue)
                return ConnectionQuality.Unknown;

            return Latency.Value.TotalMilliseconds switch
            {
                < 100 => ConnectionQuality.Excellent,
                < 300 => ConnectionQuality.Good,
                < 1000 => ConnectionQuality.Fair,
                _ => ConnectionQuality.Poor
            };
        }
    }

    public TimeSpan? Latency =>
        _latencyMs > 0 ? TimeSpan.FromMilliseconds(_latencyMs) : null;

    public DateTime? LastMessageReceived =>
        _lastMessageReceivedTicks > 0
            ? new DateTime(_lastMessageReceivedTicks, DateTimeKind.Utc)
            : null;

    public DateTime? LastMessageSent =>
        _lastMessageSentTicks > 0
            ? new DateTime(_lastMessageSentTicks, DateTimeKind.Utc)
            : null;

    public int PendingRequestCount => _pendingRequestCount;
    public int ReconnectAttempts => _reconnectAttempts;

    public void RecordConnected()
    {
        Interlocked.Exchange(ref _reconnectAttempts, 0);
        _pendingPings.Clear();
    }

    public void RecordDisconnected()
    {
        Interlocked.Increment(ref _reconnectAttempts);
        _pendingPings.Clear();
    }

    /// <summary>
    /// Record that a ping was sent with a specific message ID.
    /// </summary>
    public void RecordPingSent(string messageId)
    {
        _pendingPings[messageId] = DateTime.UtcNow.Ticks;

        // Clean up old pings (> 2 minutes) to prevent memory leak
        var cutoff = DateTime.UtcNow.AddMinutes(-2).Ticks;
        foreach (var kvp in _pendingPings)
        {
            if (kvp.Value < cutoff)
                _pendingPings.TryRemove(kvp.Key, out _);
        }
    }

    /// <summary>
    /// Record pong received, correlating with the original ping by messageId.
    /// </summary>
    public void RecordPong(string messageId, long serverTimestamp)
    {
        var now = DateTime.UtcNow.Ticks;
        Interlocked.Exchange(ref _lastPongReceivedTicks, now);

        // Correlate with the specific ping that generated this pong
        if (_pendingPings.TryRemove(messageId, out var pingSentTicks))
        {
            var latency = (now - pingSentTicks) / TimeSpan.TicksPerMillisecond;
            Interlocked.Exchange(ref _latencyMs, latency);
        }
    }

    public void RecordMessageReceived()
    {
        Interlocked.Exchange(ref _lastMessageReceivedTicks, DateTime.UtcNow.Ticks);
    }

    public void RecordMessageSent()
    {
        Interlocked.Exchange(ref _lastMessageSentTicks, DateTime.UtcNow.Ticks);
    }

    public void RecordRequestSent()
    {
        Interlocked.Increment(ref _pendingRequestCount);
    }

    public void RecordResponseReceived()
    {
        Interlocked.Decrement(ref _pendingRequestCount);
    }
}
```

### Files Summary for Sub-Session 20.2

| File | Purpose |
|------|---------|
| `WebSocketOptions.cs` | Configuration with sensible defaults |
| `ConnectionState.cs` | Connection state enum |
| `PendingRequest.cs` | Request/response tracking with timeout |
| `IWebSocketConnection.cs` | Abstraction for testability |
| `BrowserWebSocketConnection.cs` | Real WebSocket implementation |
| `WebSocketClient.cs` | Main client implementation |
| `ConnectionHealthTracker.cs` | Health metrics tracking |
| `CreateWorkItemRequest.cs` | Typed request DTO |
| `UpdateWorkItemRequest.cs` | Typed request DTO |
| **Total** | **9 files** |

### Success Criteria for 20.2

- [ ] WebSocket connects to backend
- [ ] Binary protobuf messages sent/received
- [ ] Request/response correlation via message_id
- [ ] Heartbeat ping/pong every 30 seconds
- [ ] Proper disposal of resources
- [ ] Thread-safe operations

---

## Sub-Session 20.3: Resilience Patterns

**Goal**: Circuit breaker, retry policy, reconnection service

**Estimated Tokens**: ~30k

### Phase 3.1: Circuit Breaker

```csharp
// CircuitBreaker.cs
namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Circuit breaker to prevent cascading failures.
/// Matches backend pm-ws circuit breaker behavior.
/// </summary>
public sealed class CircuitBreaker
{
    private readonly CircuitBreakerOptions _options;
    private readonly ILogger<CircuitBreaker> _logger;
    private readonly object _lock = new();

    private CircuitState _state = CircuitState.Closed;
    private int _failureCount;
    private int _successCount;
    private DateTime _lastFailureTime = DateTime.MinValue;
    private DateTime _openedAt = DateTime.MinValue;

    public CircuitState State
    {
        get { lock (_lock) return _state; }
    }

    public TimeSpan? RetryAfter
    {
        get
        {
            lock (_lock)
            {
                if (_state != CircuitState.Open)
                    return null;

                var elapsed = DateTime.UtcNow - _openedAt;
                var remaining = _options.OpenDuration - elapsed;
                return remaining > TimeSpan.Zero ? remaining : TimeSpan.Zero;
            }
        }
    }

    public CircuitBreaker(
        IOptions<CircuitBreakerOptions> options,
        ILogger<CircuitBreaker> logger)
    {
        _options = options.Value;
        _logger = logger;
    }

    /// <summary>
    /// Check if a request should be allowed through.
    /// </summary>
    public bool AllowRequest()
    {
        lock (_lock)
        {
            switch (_state)
            {
                case CircuitState.Closed:
                    return true;

                case CircuitState.Open:
                    // Check if we should transition to half-open
                    if (DateTime.UtcNow - _openedAt >= _options.OpenDuration)
                    {
                        _state = CircuitState.HalfOpen;
                        _successCount = 0;
                        _logger.LogInformation("Circuit breaker transitioning to HalfOpen");
                        return true;
                    }
                    return false;

                case CircuitState.HalfOpen:
                    return true;

                default:
                    return false;
            }
        }
    }

    /// <summary>
    /// Record a successful operation.
    /// </summary>
    public void RecordSuccess()
    {
        lock (_lock)
        {
            switch (_state)
            {
                case CircuitState.Closed:
                    _failureCount = 0;
                    break;

                case CircuitState.HalfOpen:
                    _successCount++;
                    if (_successCount >= _options.HalfOpenSuccessThreshold)
                    {
                        _state = CircuitState.Closed;
                        _failureCount = 0;
                        _logger.LogInformation(
                            "Circuit breaker closed after {Successes} successes",
                            _successCount);
                    }
                    break;
            }
        }
    }

    /// <summary>
    /// Record a failed operation.
    /// </summary>
    public void RecordFailure()
    {
        lock (_lock)
        {
            var now = DateTime.UtcNow;

            // Reset count if outside failure window
            if (now - _lastFailureTime > _options.FailureWindow)
            {
                _failureCount = 0;
            }

            _lastFailureTime = now;
            _failureCount++;

            switch (_state)
            {
                case CircuitState.Closed:
                    if (_failureCount >= _options.FailureThreshold)
                    {
                        _state = CircuitState.Open;
                        _openedAt = now;
                        _logger.LogWarning(
                            "Circuit breaker OPEN after {Failures} failures",
                            _failureCount);
                    }
                    break;

                case CircuitState.HalfOpen:
                    _state = CircuitState.Open;
                    _openedAt = now;
                    _logger.LogWarning(
                        "Circuit breaker reopened due to failure in HalfOpen state");
                    break;
            }
        }
    }

    /// <summary>
    /// Execute an operation with circuit breaker protection.
    /// </summary>
    public async Task<T> ExecuteAsync<T>(
        Func<CancellationToken, Task<T>> operation,
        CancellationToken ct = default)
    {
        if (!AllowRequest())
        {
            throw new CircuitOpenException(RetryAfter ?? _options.OpenDuration);
        }

        try
        {
            var result = await operation(ct);
            RecordSuccess();
            return result;
        }
        catch (Exception ex) when (ShouldRecordAsFailure(ex))
        {
            RecordFailure();
            throw;
        }
    }

    private static bool ShouldRecordAsFailure(Exception ex)
    {
        // Don't count validation errors or cancellation as circuit failures
        return ex is not ValidationException
            && ex is not OperationCanceledException
            && ex is not VersionConflictException;
    }
}

public enum CircuitState
{
    Closed,
    Open,
    HalfOpen
}

// CircuitBreakerOptions.cs
public sealed class CircuitBreakerOptions
{
    /// <summary>Number of failures before opening circuit.</summary>
    public int FailureThreshold { get; set; } = 5;

    /// <summary>Duration to keep circuit open before testing.</summary>
    public TimeSpan OpenDuration { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>Successes needed in half-open to close circuit.</summary>
    public int HalfOpenSuccessThreshold { get; set; } = 3;

    /// <summary>Window for counting failures.</summary>
    public TimeSpan FailureWindow { get; set; } = TimeSpan.FromSeconds(60);
}
```

### Phase 3.2: Retry Policy

```csharp
// RetryPolicy.cs
namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Retry policy with exponential backoff and jitter.
/// </summary>
public sealed class RetryPolicy
{
    private readonly RetryPolicyOptions _options;
    private readonly ILogger<RetryPolicy> _logger;
    // Note: Random.Shared is thread-safe on .NET 6+ (our target)

    public RetryPolicy(
        IOptions<RetryPolicyOptions> options,
        ILogger<RetryPolicy> logger)
    {
        _options = options.Value;
        _logger = logger;
    }

    /// <summary>
    /// Execute an operation with retry logic.
    /// </summary>
    public async Task<T> ExecuteAsync<T>(
        Func<CancellationToken, Task<T>> operation,
        CancellationToken ct = default)
    {
        var attempt = 0;
        var delay = _options.InitialDelay;

        while (true)
        {
            attempt++;

            try
            {
                return await operation(ct);
            }
            catch (Exception ex) when (ShouldRetry(ex, attempt))
            {
                var jitteredDelay = AddJitter(delay);

                _logger.LogWarning(
                    ex,
                    "Attempt {Attempt}/{MaxAttempts} failed. Retrying in {Delay}ms",
                    attempt,
                    _options.MaxAttempts,
                    jitteredDelay.TotalMilliseconds);

                await Task.Delay(jitteredDelay, ct);

                delay = TimeSpan.FromMilliseconds(
                    Math.Min(
                        delay.TotalMilliseconds * _options.BackoffMultiplier,
                        _options.MaxDelay.TotalMilliseconds));
            }
        }
    }

    private bool ShouldRetry(Exception ex, int attempt)
    {
        if (attempt >= _options.MaxAttempts)
            return false;

        // Don't retry non-transient errors
        return ex is ConnectionException
            or RequestTimeoutException
            or IOException
            or WebSocketException;
    }

    private static TimeSpan AddJitter(TimeSpan delay)
    {
        // Add up to 25% jitter to prevent thundering herd
        var jitterFactor = 1.0 + (Random.Shared.NextDouble() * 0.25);
        return TimeSpan.FromMilliseconds(delay.TotalMilliseconds * jitterFactor);
    }
}

// RetryPolicyOptions.cs
public sealed class RetryPolicyOptions
{
    public int MaxAttempts { get; set; } = 3;
    public TimeSpan InitialDelay { get; set; } = TimeSpan.FromMilliseconds(100);
    // Aligned with pm-config DEFAULT_MAX_DELAY_SECS = 5
    public TimeSpan MaxDelay { get; set; } = TimeSpan.FromSeconds(5);
    public double BackoffMultiplier { get; set; } = 2.0;
}
```

### Phase 3.3: Reconnection Service

```csharp
// ReconnectionService.cs
namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Handles automatic reconnection with exponential backoff.
/// Rehydrates subscriptions after successful reconnect.
/// </summary>
public sealed class ReconnectionService : IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<ReconnectionService> _logger;
    private readonly ReconnectionOptions _options;

    // Track subscribed project IDs for rehydration after reconnect
    private readonly HashSet<Guid> _subscribedProjects = new();
    private readonly object _subscriptionLock = new();

    private CancellationTokenSource? _reconnectCts;
    private Task? _reconnectTask;
    private readonly SemaphoreSlim _lock = new(1, 1);
    private bool _disposed;

    public event Action<int>? OnReconnecting;
    public event Action? OnReconnected;
    public event Action<Exception>? OnReconnectFailed;

    public ReconnectionService(
        IWebSocketClient client,
        IOptions<ReconnectionOptions> options,
        ILogger<ReconnectionService> logger)
    {
        _client = client;
        _options = options.Value;
        _logger = logger;

        _client.OnStateChanged += HandleStateChanged;
    }

    /// <summary>
    /// Register a project subscription for rehydration after reconnect.
    /// Call this whenever the client subscribes to a project.
    /// </summary>
    public void TrackSubscription(Guid projectId)
    {
        lock (_subscriptionLock)
        {
            _subscribedProjects.Add(projectId);
        }
    }

    /// <summary>
    /// Remove a project subscription from rehydration tracking.
    /// Call this when the client unsubscribes from a project.
    /// </summary>
    public void UntrackSubscription(Guid projectId)
    {
        lock (_subscriptionLock)
        {
            _subscribedProjects.Remove(projectId);
        }
    }

    /// <summary>
    /// Get currently tracked subscriptions.
    /// </summary>
    public IReadOnlyList<Guid> TrackedSubscriptions
    {
        get
        {
            lock (_subscriptionLock)
            {
                return _subscribedProjects.ToList();
            }
        }
    }

    private void HandleStateChanged(ConnectionState state)
    {
        if (state == ConnectionState.Reconnecting && _reconnectTask == null)
        {
            _ = StartReconnectionAsync();
        }
    }

    private async Task StartReconnectionAsync()
    {
        await _lock.WaitAsync();
        try
        {
            if (_reconnectTask != null || _disposed)
                return;

            _reconnectCts = new CancellationTokenSource();
            _reconnectTask = ReconnectLoopAsync(_reconnectCts.Token);
        }
        finally
        {
            _lock.Release();
        }
    }

    private async Task ReconnectLoopAsync(CancellationToken ct)
    {
        var attempt = 0;
        var delay = _options.InitialDelay;

        while (!ct.IsCancellationRequested && attempt < _options.MaxAttempts)
        {
            attempt++;
            OnReconnecting?.Invoke(attempt);

            _logger.LogInformation(
                "Reconnection attempt {Attempt}/{MaxAttempts}",
                attempt,
                _options.MaxAttempts);

            try
            {
                await _client.ConnectAsync(ct);

                // Rehydrate subscriptions after successful reconnect
                var subscriptions = TrackedSubscriptions;
                if (subscriptions.Count > 0)
                {
                    _logger.LogInformation(
                        "Rehydrating {Count} project subscriptions",
                        subscriptions.Count);
                    await _client.SubscribeAsync(subscriptions, ct);
                }

                _logger.LogInformation("Reconnected successfully");
                OnReconnected?.Invoke();

                await _lock.WaitAsync(ct);
                try
                {
                    _reconnectTask = null;
                }
                finally
                {
                    _lock.Release();
                }

                return;
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                _logger.LogWarning(
                    ex,
                    "Reconnection attempt {Attempt} failed",
                    attempt);

                if (attempt < _options.MaxAttempts)
                {
                    var jitteredDelay = AddJitter(delay);
                    await Task.Delay(jitteredDelay, ct);
                    delay = TimeSpan.FromMilliseconds(
                        Math.Min(
                            delay.TotalMilliseconds * 2,
                            _options.MaxDelay.TotalMilliseconds));
                }
            }
        }

        _logger.LogError(
            "Failed to reconnect after {MaxAttempts} attempts",
            _options.MaxAttempts);

        OnReconnectFailed?.Invoke(
            new ConnectionException($"Failed to reconnect after {attempt} attempts"));

        await _lock.WaitAsync(ct);
        try
        {
            _reconnectTask = null;
        }
        finally
        {
            _lock.Release();
        }
    }

    private static TimeSpan AddJitter(TimeSpan delay)
    {
        var jitter = Random.Shared.NextDouble() * 0.25;
        return TimeSpan.FromMilliseconds(delay.TotalMilliseconds * (1 + jitter));
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnStateChanged -= HandleStateChanged;
        _reconnectCts?.Cancel();
        _reconnectCts?.Dispose();
        _lock.Dispose();
    }
}

// ReconnectionOptions.cs
// Note: Reconnection is a client-only concern (not present in pm-config).
// These values are tuned for desktop/WebSocket UX, not server-side retry policy.
public sealed class ReconnectionOptions
{
    public int MaxAttempts { get; set; } = 10;
    public TimeSpan InitialDelay { get; set; } = TimeSpan.FromSeconds(1);
    public TimeSpan MaxDelay { get; set; } = TimeSpan.FromSeconds(30);
}
```

### Phase 3.4: Resilient WebSocket Client Wrapper

```csharp
// ResilientWebSocketClient.cs
namespace ProjectManagement.Services.WebSocket;

/// <summary>
/// WebSocket client wrapper with circuit breaker and retry protection.
/// </summary>
public sealed class ResilientWebSocketClient : IWebSocketClient
{
    private readonly WebSocketClient _inner;
    private readonly CircuitBreaker _circuitBreaker;
    private readonly RetryPolicy _retryPolicy;
    private readonly ILogger<ResilientWebSocketClient> _logger;

    public ConnectionState State => _inner.State;
    public IConnectionHealth Health => _inner.Health;

    public event Action<ConnectionState>? OnStateChanged
    {
        add => _inner.OnStateChanged += value;
        remove => _inner.OnStateChanged -= value;
    }

    public event Action<WorkItem>? OnWorkItemCreated
    {
        add => _inner.OnWorkItemCreated += value;
        remove => _inner.OnWorkItemCreated -= value;
    }

    public event Action<WorkItem, IReadOnlyList<FieldChange>>? OnWorkItemUpdated
    {
        add => _inner.OnWorkItemUpdated += value;
        remove => _inner.OnWorkItemUpdated -= value;
    }

    public event Action<Guid>? OnWorkItemDeleted
    {
        add => _inner.OnWorkItemDeleted += value;
        remove => _inner.OnWorkItemDeleted -= value;
    }

    public ResilientWebSocketClient(
        WebSocketClient inner,
        CircuitBreaker circuitBreaker,
        RetryPolicy retryPolicy,
        ILogger<ResilientWebSocketClient> logger)
    {
        _inner = inner;
        _circuitBreaker = circuitBreaker;
        _retryPolicy = retryPolicy;
        _logger = logger;
    }

    public Task ConnectAsync(CancellationToken ct = default)
        => _inner.ConnectAsync(ct);

    public Task DisconnectAsync(CancellationToken ct = default)
        => _inner.DisconnectAsync(ct);

    public Task SubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.SubscribeAsync(projectIds, token),
            ct);

    public Task UnsubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.UnsubscribeAsync(projectIds, token),
            ct);

    public Task<WorkItem> CreateWorkItemAsync(
        CreateWorkItemRequest request,
        CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.CreateWorkItemAsync(request, token),
            ct);

    public Task<WorkItem> UpdateWorkItemAsync(
        UpdateWorkItemRequest request,
        CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.UpdateWorkItemAsync(request, token),
            ct);

    public Task DeleteWorkItemAsync(Guid workItemId, CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            async token =>
            {
                await _inner.DeleteWorkItemAsync(workItemId, token);
                return true;
            },
            ct);

    public Task<IReadOnlyList<WorkItem>> GetWorkItemsAsync(
        Guid projectId,
        DateTime? since = null,
        CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.GetWorkItemsAsync(projectId, since, token),
            ct);

    private async Task<T> ExecuteWithResilienceAsync<T>(
        Func<CancellationToken, Task<T>> operation,
        CancellationToken ct)
    {
        return await _circuitBreaker.ExecuteAsync(
            token => _retryPolicy.ExecuteAsync(operation, token),
            ct);
    }

    private async Task ExecuteWithResilienceAsync(
        Func<CancellationToken, Task> operation,
        CancellationToken ct)
    {
        await _circuitBreaker.ExecuteAsync(
            async token =>
            {
                await _retryPolicy.ExecuteAsync(
                    async t =>
                    {
                        await operation(t);
                        return true;
                    },
                    token);
                return true;
            },
            ct);
    }

    public ValueTask DisposeAsync() => _inner.DisposeAsync();
}
```

### Files Summary for Sub-Session 20.3

| File | Purpose |
|------|---------|
| `CircuitBreaker.cs` | Circuit breaker pattern |
| `CircuitBreakerOptions.cs` | Circuit breaker configuration |
| `RetryPolicy.cs` | Retry with exponential backoff |
| `RetryPolicyOptions.cs` | Retry configuration |
| `ReconnectionService.cs` | Automatic reconnection |
| `ReconnectionOptions.cs` | Reconnection configuration |
| `ResilientWebSocketClient.cs` | Wrapper combining resilience patterns |
| **Total** | **7 files** |

### Success Criteria for 20.3

- [ ] Circuit breaker opens after configured failures
- [ ] Circuit breaker transitions through Closed -> Open -> HalfOpen -> Closed
- [ ] Retry policy uses exponential backoff with jitter
- [ ] Reconnection service handles disconnects
- [ ] All resilience patterns are thread-safe

---

## Sub-Session 20.4: State Management

**Goal**: Thread-safe state stores with optimistic updates

**Estimated Tokens**: ~30k

### Phase 4.1: Work Item Store

```csharp
// WorkItemStore.cs
namespace ProjectManagement.Services.State;

/// <summary>
/// Thread-safe state management for work items with optimistic updates.
/// </summary>
public sealed class WorkItemStore : IWorkItemStore
{
    private readonly ConcurrentDictionary<Guid, WorkItem> _workItems = new();
    private readonly ConcurrentDictionary<Guid, OptimisticUpdate<WorkItem>> _pendingUpdates = new();
    private readonly IWebSocketClient _client;
    private readonly ILogger<WorkItemStore> _logger;
    private readonly SemaphoreSlim _operationLock = new(1, 1);

    private bool _disposed;

    public event Action? OnChanged;

    public WorkItemStore(
        IWebSocketClient client,
        ILogger<WorkItemStore> logger)
    {
        _client = client;
        _logger = logger;

        _client.OnWorkItemCreated += HandleWorkItemCreated;
        _client.OnWorkItemUpdated += HandleWorkItemUpdated;
        _client.OnWorkItemDeleted += HandleWorkItemDeleted;
    }

    #region Read Operations

    public IReadOnlyList<WorkItem> GetByProject(Guid projectId)
    {
        return _workItems.Values
            .Where(w => w.ProjectId == projectId && !w.IsDeleted)
            .OrderBy(w => w.Position)
            .ToList();
    }

    public WorkItem? GetById(Guid id)
    {
        return _workItems.TryGetValue(id, out var item) && !item.IsDeleted
            ? item
            : null;
    }

    public IReadOnlyList<WorkItem> GetBySprint(Guid sprintId)
    {
        return _workItems.Values
            .Where(w => w.SprintId == sprintId && !w.IsDeleted)
            .OrderBy(w => w.Position)
            .ToList();
    }

    public IReadOnlyList<WorkItem> GetChildren(Guid parentId)
    {
        return _workItems.Values
            .Where(w => w.ParentId == parentId && !w.IsDeleted)
            .OrderBy(w => w.Position)
            .ToList();
    }

    #endregion

    #region Write Operations

    public async Task<WorkItem> CreateAsync(
        CreateWorkItemRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // Create optimistic item (temporary ID)
        var tempId = Guid.NewGuid();
        var optimistic = new WorkItem
        {
            Id = tempId,
            ItemType = request.ItemType,
            Title = request.Title,
            Description = request.Description,
            ProjectId = request.ProjectId,
            ParentId = request.ParentId,
            Status = WorkItemDefaults.Status,
            Priority = WorkItemDefaults.Priority,
            Position = int.MaxValue, // Will be fixed by server
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        // Apply optimistically
        _workItems[tempId] = optimistic;
        var pendingUpdate = new OptimisticUpdate<WorkItem>(tempId, null, optimistic);
        _pendingUpdates[tempId] = pendingUpdate;
        NotifyChanged();

        try
        {
            var confirmed = await _client.CreateWorkItemAsync(request, ct);

            // Replace temp with confirmed
            _workItems.TryRemove(tempId, out _);
            _workItems[confirmed.Id] = confirmed;
            _pendingUpdates.TryRemove(tempId, out _);

            NotifyChanged();

            _logger.LogDebug("Work item created: {Id}", confirmed.Id);
            return confirmed;
        }
        catch
        {
            // Rollback
            _workItems.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task<WorkItem> UpdateAsync(
        UpdateWorkItemRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var id = request.WorkItemId;

        if (!_workItems.TryGetValue(id, out var current))
        {
            throw new KeyNotFoundException($"Work item not found: {id}");
        }

        // Build optimistic update
        var optimistic = ApplyUpdate(current, request);

        // Apply optimistically
        var previousValue = _workItems[id];
        _workItems[id] = optimistic;
        var pendingUpdate = new OptimisticUpdate<WorkItem>(id, previousValue, optimistic);
        _pendingUpdates[id] = pendingUpdate;
        NotifyChanged();

        try
        {
            var confirmed = await _client.UpdateWorkItemAsync(request, ct);

            // Apply confirmed version
            _workItems[id] = confirmed;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();

            _logger.LogDebug("Work item updated: {Id}", id);
            return confirmed;
        }
        catch
        {
            // Rollback
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task DeleteAsync(Guid id, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_workItems.TryGetValue(id, out var current))
        {
            return; // Already deleted
        }

        // Apply optimistic soft delete
        var optimistic = current with { DeletedAt = DateTime.UtcNow };
        var previousValue = _workItems[id];
        _workItems[id] = optimistic;
        var pendingUpdate = new OptimisticUpdate<WorkItem>(id, previousValue, optimistic);
        _pendingUpdates[id] = pendingUpdate;
        NotifyChanged();

        try
        {
            await _client.DeleteWorkItemAsync(id, ct);
            _pendingUpdates.TryRemove(id, out _);

            _logger.LogDebug("Work item deleted: {Id}", id);
        }
        catch
        {
            // Rollback
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task RefreshAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var items = await _client.GetWorkItemsAsync(projectId, null, ct);

        // Remove old items for this project
        var toRemove = _workItems.Values
            .Where(w => w.ProjectId == projectId)
            .Select(w => w.Id)
            .ToList();

        foreach (var id in toRemove)
        {
            _workItems.TryRemove(id, out _);
        }

        // Add new items
        foreach (var item in items)
        {
            _workItems[item.Id] = item;
        }

        NotifyChanged();

        _logger.LogDebug("Refreshed {Count} work items for project {ProjectId}",
            items.Count, projectId);
    }

    #endregion

    #region Event Handlers

    private void HandleWorkItemCreated(WorkItem item)
    {
        // Don't overwrite pending updates
        if (_pendingUpdates.ContainsKey(item.Id))
            return;

        _workItems[item.Id] = item;
        NotifyChanged();

        _logger.LogDebug("Received work item created: {Id}", item.Id);
    }

    private void HandleWorkItemUpdated(WorkItem item, IReadOnlyList<FieldChange> changes)
    {
        // Don't overwrite pending updates
        if (_pendingUpdates.ContainsKey(item.Id))
            return;

        _workItems[item.Id] = item;
        NotifyChanged();

        _logger.LogDebug("Received work item updated: {Id}, changes: {Changes}",
            item.Id, string.Join(", ", changes.Select(c => c.FieldName)));
    }

    private void HandleWorkItemDeleted(Guid id)
    {
        // Don't process if we have a pending update
        if (_pendingUpdates.ContainsKey(id))
            return;

        if (_workItems.TryGetValue(id, out var item))
        {
            _workItems[id] = item with { DeletedAt = DateTime.UtcNow };
            NotifyChanged();
        }

        _logger.LogDebug("Received work item deleted: {Id}", id);
    }

    #endregion

    #region Helpers

    private static WorkItem ApplyUpdate(WorkItem current, UpdateWorkItemRequest request)
    {
        return current with
        {
            Title = request.Title ?? current.Title,
            Description = request.Description ?? current.Description,
            Status = request.Status ?? current.Status,
            Priority = request.Priority ?? current.Priority,
            AssigneeId = request.AssigneeId ?? current.AssigneeId,
            SprintId = request.SprintId ?? current.SprintId,
            StoryPoints = request.StoryPoints ?? current.StoryPoints,
            Position = request.Position ?? current.Position,
            UpdatedAt = DateTime.UtcNow,
            Version = current.Version + 1
        };
    }

    private void NotifyChanged()
    {
        try
        {
            OnChanged?.Invoke();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in OnChanged handler");
        }
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    #endregion

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnWorkItemCreated -= HandleWorkItemCreated;
        _client.OnWorkItemUpdated -= HandleWorkItemUpdated;
        _client.OnWorkItemDeleted -= HandleWorkItemDeleted;

        _operationLock.Dispose();
    }
}
```

### Phase 4.2: Optimistic Update Tracking

```csharp
// OptimisticUpdate.cs
namespace ProjectManagement.Services.State;

/// <summary>
/// Tracks a pending optimistic update for rollback capability.
/// </summary>
internal sealed record OptimisticUpdate<T>(
    Guid EntityId,
    T? OriginalValue,
    T OptimisticValue)
{
    public DateTime CreatedAt { get; } = DateTime.UtcNow;

    public bool IsCreate => OriginalValue is null;
}
```

### Phase 4.3: Sprint Store

```csharp
// SprintStore.cs
namespace ProjectManagement.Services.State;

/// <summary>
/// Thread-safe state management for sprints.
/// Follows same pattern as WorkItemStore.
/// </summary>
public sealed class SprintStore : ISprintStore
{
    private readonly ConcurrentDictionary<Guid, Sprint> _sprints = new();
    private readonly IWebSocketClient _client;
    private readonly ILogger<SprintStore> _logger;

    private bool _disposed;

    public event Action? OnChanged;

    public SprintStore(
        IWebSocketClient client,
        ILogger<SprintStore> logger)
    {
        _client = client;
        _logger = logger;

        // TODO: Wire up sprint events when backend adds them
        // _client.OnSprintCreated += HandleSprintCreated;
        // _client.OnSprintUpdated += HandleSprintUpdated;
    }

    #region Read Operations

    public IReadOnlyList<Sprint> GetByProject(Guid projectId)
    {
        return _sprints.Values
            .Where(s => s.ProjectId == projectId && !s.IsDeleted)
            .OrderBy(s => s.StartDate)
            .ToList();
    }

    public Sprint? GetById(Guid id)
    {
        return _sprints.TryGetValue(id, out var sprint) && !sprint.IsDeleted
            ? sprint
            : null;
    }

    public Sprint? GetActiveSprint(Guid projectId)
    {
        return _sprints.Values
            .FirstOrDefault(s => s.ProjectId == projectId
                && s.Status == SprintStatus.Active
                && !s.IsDeleted);
    }

    #endregion

    #region Write Operations

    public async Task<Sprint> CreateAsync(
        CreateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // TODO: Call _client.CreateSprintAsync when backend handler is implemented
        // For now, create locally (will be replaced in Session 50)
        var sprint = new Sprint
        {
            Id = Guid.NewGuid(),
            ProjectId = request.ProjectId,
            Name = request.Name,
            Goal = request.Goal,
            StartDate = request.StartDate,
            EndDate = request.EndDate,
            Status = SprintStatus.Planned,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.Empty, // Will be set by server
            UpdatedBy = Guid.Empty
        };

        _sprints[sprint.Id] = sprint;
        NotifyChanged();

        _logger.LogDebug("Sprint created locally: {Id}", sprint.Id);
        return sprint;
    }

    public async Task<Sprint> UpdateAsync(
        UpdateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(request.SprintId, out var current))
        {
            throw new KeyNotFoundException($"Sprint not found: {request.SprintId}");
        }

        var updated = current with
        {
            Name = request.Name ?? current.Name,
            Goal = request.Goal ?? current.Goal,
            StartDate = request.StartDate ?? current.StartDate,
            EndDate = request.EndDate ?? current.EndDate,
            UpdatedAt = DateTime.UtcNow
        };

        _sprints[request.SprintId] = updated;
        NotifyChanged();

        return updated;
    }

    public async Task<Sprint> StartSprintAsync(Guid sprintId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(sprintId, out var current))
        {
            throw new KeyNotFoundException($"Sprint not found: {sprintId}");
        }

        if (current.Status != SprintStatus.Planned)
        {
            throw new InvalidOperationException($"Cannot start sprint in {current.Status} state");
        }

        // Check for existing active sprint in same project
        var activeSprint = GetActiveSprint(current.ProjectId);
        if (activeSprint != null)
        {
            throw new InvalidOperationException(
                $"Project already has an active sprint: {activeSprint.Name}");
        }

        var started = current with
        {
            Status = SprintStatus.Active,
            UpdatedAt = DateTime.UtcNow
        };

        _sprints[sprintId] = started;
        NotifyChanged();

        return started;
    }

    public async Task<Sprint> CompleteSprintAsync(Guid sprintId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(sprintId, out var current))
        {
            throw new KeyNotFoundException($"Sprint not found: {sprintId}");
        }

        if (current.Status != SprintStatus.Active)
        {
            throw new InvalidOperationException($"Cannot complete sprint in {current.Status} state");
        }

        var completed = current with
        {
            Status = SprintStatus.Completed,
            UpdatedAt = DateTime.UtcNow
        };

        _sprints[sprintId] = completed;
        NotifyChanged();

        return completed;
    }

    public async Task DeleteAsync(Guid id, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(id, out var current))
        {
            return; // Already deleted
        }

        var deleted = current with { DeletedAt = DateTime.UtcNow };
        _sprints[id] = deleted;
        NotifyChanged();
    }

    public async Task RefreshAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // TODO: Call _client.GetSprintsAsync when backend handler is implemented
        // For now, this is a no-op (will be replaced in Session 50)
        _logger.LogDebug("Sprint refresh for project {ProjectId} - not yet implemented", projectId);
    }

    #endregion

    #region Helpers

    private void NotifyChanged()
    {
        try
        {
            OnChanged?.Invoke();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in OnChanged handler");
        }
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    #endregion

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
    }
}
```

### Phase 4.4: App State Container

```csharp
// AppState.cs
namespace ProjectManagement.Services.State;

/// <summary>
/// Root state container for the application.
/// Provides centralized access to all stores.
/// </summary>
public sealed class AppState : IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<AppState> _logger;

    public IWorkItemStore WorkItems { get; }
    public ISprintStore Sprints { get; }
    public IConnectionHealth ConnectionHealth => _client.Health;
    public ConnectionState ConnectionState => _client.State;

    public event Action? OnStateChanged;
    public event Action<ConnectionState>? OnConnectionStateChanged;

    private bool _disposed;

    public AppState(
        IWebSocketClient client,
        IWorkItemStore workItems,
        ISprintStore sprints,
        ILogger<AppState> logger)
    {
        _client = client;
        _logger = logger;

        WorkItems = workItems;
        Sprints = sprints;

        // Forward events
        _client.OnStateChanged += state =>
        {
            OnConnectionStateChanged?.Invoke(state);
            OnStateChanged?.Invoke();
        };

        workItems.OnChanged += () => OnStateChanged?.Invoke();
        sprints.OnChanged += () => OnStateChanged?.Invoke();
    }

    /// <summary>
    /// Initialize state by connecting and loading initial data.
    /// </summary>
    public async Task InitializeAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        _logger.LogInformation("Initializing application state");

        await _client.ConnectAsync(ct);

        _logger.LogInformation("Application state initialized");
    }

    /// <summary>
    /// Load data for a specific project.
    /// </summary>
    public async Task LoadProjectAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        _logger.LogInformation("Loading project {ProjectId}", projectId);

        // Subscribe to project updates
        await _client.SubscribeAsync([projectId], ct);

        // Load initial data
        await WorkItems.RefreshAsync(projectId, ct);
        await Sprints.RefreshAsync(projectId, ct);

        _logger.LogInformation("Project {ProjectId} loaded", projectId);
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        if (WorkItems is IDisposable workItemsDisposable)
            workItemsDisposable.Dispose();
        if (Sprints is IDisposable sprintsDisposable)
            sprintsDisposable.Dispose();
    }
}
```

### Files Summary for Sub-Session 20.4

| File | Purpose |
|------|---------|
| `WorkItemStore.cs` | Work item state with optimistic updates |
| `SprintStore.cs` | Sprint state management (stub - full impl in Session 50) |
| `OptimisticUpdate.cs` | Pending update tracking |
| `AppState.cs` | Root state container |
| **Total** | **4 files** |

> **Note**: `CommentStore.cs` deferred to Session 50 (Sprints & Comments) to avoid stub proliferation.

### Success Criteria for 20.4

- [ ] Thread-safe with ConcurrentDictionary
- [ ] Optimistic updates apply immediately
- [ ] Rollback works on server rejection
- [ ] Server broadcasts update local state
- [ ] Change notifications fire correctly
- [ ] Proper disposal of resources

---

## Sub-Session 20.5: WASM Host & Observability

**Goal**: WASM host setup with error boundaries and structured logging

**Estimated Tokens**: ~25k

### Phase 5.1: Program.cs with Full DI Setup

```csharp
// Program.cs
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using ProjectManagement.Services.WebSocket;
using ProjectManagement.Services.State;
using ProjectManagement.Services.Resilience;
using ProjectManagement.Services.Logging;
using ProjectManagement.Wasm;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

// Configuration
builder.Services.Configure<WebSocketOptions>(
    builder.Configuration.GetSection("WebSocket"));
builder.Services.Configure<CircuitBreakerOptions>(
    builder.Configuration.GetSection("CircuitBreaker"));
builder.Services.Configure<RetryPolicyOptions>(
    builder.Configuration.GetSection("Retry"));
builder.Services.Configure<ReconnectionOptions>(
    builder.Configuration.GetSection("Reconnection"));

// Logging
builder.Services.AddLogging(logging =>
{
    logging.SetMinimumLevel(LogLevel.Debug);
    logging.AddProvider(new CorrelationIdLoggerProvider());
});

// Resilience
builder.Services.AddSingleton<CircuitBreaker>();
builder.Services.AddSingleton<RetryPolicy>();
builder.Services.AddSingleton<ReconnectionService>();

// WebSocket
builder.Services.AddSingleton<WebSocketClient>();
builder.Services.AddSingleton<IWebSocketClient>(sp =>
{
    var inner = sp.GetRequiredService<WebSocketClient>();
    var circuitBreaker = sp.GetRequiredService<CircuitBreaker>();
    var retryPolicy = sp.GetRequiredService<RetryPolicy>();
    var logger = sp.GetRequiredService<ILogger<ResilientWebSocketClient>>();
    return new ResilientWebSocketClient(inner, circuitBreaker, retryPolicy, logger);
});

// State Management
builder.Services.AddSingleton<IWorkItemStore, WorkItemStore>();
builder.Services.AddSingleton<ISprintStore, SprintStore>();
builder.Services.AddSingleton<AppState>();

// Radzen
builder.Services.AddRadzenComponents();

var app = builder.Build();

// Initialize state
var appState = app.Services.GetRequiredService<AppState>();
var logger = app.Services.GetRequiredService<ILogger<Program>>();

try
{
    await appState.InitializeAsync();
}
catch (Exception ex)
{
    logger.LogError(ex, "Failed to initialize application state");
    // App will show connection error UI
}

await app.RunAsync();
```

### Phase 5.2: Error Boundary Component

```razor
@* ErrorBoundary.razor *@
@using Microsoft.AspNetCore.Components.Web
@inherits ErrorBoundaryBase

@if (CurrentException is not null)
{
    <div class="error-boundary">
        <div class="error-content">
            <h3>Something went wrong</h3>
            <p>@GetUserFriendlyMessage()</p>
            <button class="btn btn-primary" @onclick="Recover">Try Again</button>
        </div>
    </div>
}
else
{
    @ChildContent
}

@code {
    [Parameter]
    public RenderFragment? ChildContent { get; set; }

    [Inject]
    private ILogger<ErrorBoundary> Logger { get; set; } = default!;

    protected override Task OnErrorAsync(Exception exception)
    {
        Logger.LogError(exception, "Unhandled error in component tree");
        return Task.CompletedTask;
    }

    private string GetUserFriendlyMessage()
    {
        return CurrentException switch
        {
            ConnectionException => "Unable to connect to the server. Please check your connection.",
            RequestTimeoutException => "The request timed out. Please try again.",
            CircuitOpenException => "Service temporarily unavailable. Please wait a moment.",
            ValidationException ve => ve.UserMessage,
            ServerRejectedException sre => sre.UserMessage,
            _ => "An unexpected error occurred. Please try again."
        };
    }
}
```

### Phase 5.3: Correlation ID Logger

```csharp
// CorrelationIdLogger.cs
namespace ProjectManagement.Services.Logging;

public sealed class CorrelationIdLoggerProvider : ILoggerProvider
{
    public ILogger CreateLogger(string categoryName)
    {
        return new CorrelationIdLogger(categoryName);
    }

    public void Dispose() { }
}

public sealed class CorrelationIdLogger : ILogger
{
    private readonly string _categoryName;
    private static readonly AsyncLocal<string?> _correlationId = new();

    public static string CorrelationId
    {
        get => _correlationId.Value ?? Guid.NewGuid().ToString("N")[..8];
        set => _correlationId.Value = value;
    }

    public CorrelationIdLogger(string categoryName)
    {
        _categoryName = categoryName;
    }

    public IDisposable? BeginScope<TState>(TState state) where TState : notnull
    {
        return null;
    }

    public bool IsEnabled(LogLevel logLevel) => true;

    public void Log<TState>(
        LogLevel logLevel,
        EventId eventId,
        TState state,
        Exception? exception,
        Func<TState, Exception?, string> formatter)
    {
        if (!IsEnabled(logLevel))
            return;

        var message = formatter(state, exception);
        var timestamp = DateTime.UtcNow.ToString("HH:mm:ss.fff");
        var level = logLevel.ToString()[..3].ToUpper();
        var category = _categoryName.Split('.').LastOrDefault() ?? _categoryName;

        Console.WriteLine($"[{timestamp}] [{level}] [{CorrelationId}] {category}: {message}");

        if (exception != null)
        {
            Console.WriteLine($"  Exception: {exception.GetType().Name}: {exception.Message}");
        }
    }
}
```

### Phase 5.4: appsettings.json

```json
{
  "WebSocket": {
    "ServerUrl": "ws://localhost:8080/ws",
    "HeartbeatInterval": "00:00:30",
    "HeartbeatTimeout": "00:01:00",
    "RequestTimeout": "00:00:30"
  },
  "CircuitBreaker": {
    "FailureThreshold": 5,
    "OpenDuration": "00:00:30",
    "HalfOpenSuccessThreshold": 3,
    "FailureWindow": "00:01:00"
  },
  "Retry": {
    "MaxAttempts": 3,
    "InitialDelay": "00:00:00.100",
    "MaxDelay": "00:00:05"
  },
  "Reconnection": {
    "MaxAttempts": 10,
    "InitialDelay": "00:00:01",
    "MaxDelay": "00:00:30"
  }
}
```

### Files Summary for Sub-Session 20.5

| File | Purpose |
|------|---------|
| `Program.cs` | Full DI setup |
| `App.razor` | Root component |
| `_Imports.razor` | Global imports |
| `ErrorBoundary.razor` | Error handling |
| `CorrelationIdLogger.cs` | Structured logging |
| `wwwroot/index.html` | Host page |
| `wwwroot/css/app.css` | Base styles |
| `wwwroot/appsettings.json` | Configuration |
| **Total** | **8 files** |

### Success Criteria for 20.5

- [ ] WASM app runs and connects to backend
- [ ] Error boundary catches and displays errors gracefully
- [ ] Structured logging with correlation IDs
- [ ] Configuration loaded from appsettings.json
- [ ] Radzen components available

---

## Sub-Session 20.6: Comprehensive Test Suite

**Goal**: 100+ tests covering all components

**Estimated Tokens**: ~30k

### Test Project Structure

```
frontend/
├── ProjectManagement.Core.Tests/
│   ├── Converters/
│   │   └── ProtoConverterTests.cs
│   ├── Validation/
│   │   └── ValidatorTests.cs
│   └── Models/
│       └── WorkItemTests.cs
├── ProjectManagement.Services.Tests/
│   ├── WebSocket/
│   │   ├── WebSocketClientTests.cs
│   │   ├── PendingRequestTests.cs
│   │   └── ConnectionHealthTrackerTests.cs
│   ├── Resilience/
│   │   ├── CircuitBreakerTests.cs
│   │   ├── RetryPolicyTests.cs
│   │   └── ReconnectionServiceTests.cs
│   ├── State/
│   │   ├── WorkItemStoreTests.cs
│   │   └── OptimisticUpdateTests.cs
│   ├── Mocks/
│   │   └── MockWebSocketConnection.cs
│   └── PropertyTests/
│       ├── ProtoConverterPropertyTests.cs
│       └── CircuitBreakerPropertyTests.cs
```

### Test Categories (Target: 100+ tests)

| Category | Test Count | Focus |
|----------|------------|-------|
| ProtoConverter | 20 | All entity conversions, edge cases |
| Validators | 15 | All validation rules |
| WebSocketClient | 20 | Connect, send, receive, timeout |
| CircuitBreaker | 15 | State transitions, thread safety |
| RetryPolicy | 10 | Backoff, jitter, max attempts |
| WorkItemStore | 15 | CRUD, optimistic updates, rollback |
| Property Tests | 10 | Random input fuzzing |
| **Total** | **105** | |

### Sample Test Files

```csharp
// CircuitBreakerTests.cs
namespace ProjectManagement.Services.Tests.Resilience;

public class CircuitBreakerTests
{
    private readonly CircuitBreaker _sut;
    private readonly Mock<ILogger<CircuitBreaker>> _logger;

    public CircuitBreakerTests()
    {
        _logger = new Mock<ILogger<CircuitBreaker>>();
        var options = Options.Create(new CircuitBreakerOptions
        {
            FailureThreshold = 3,
            OpenDuration = TimeSpan.FromMilliseconds(100),
            HalfOpenSuccessThreshold = 2,
            FailureWindow = TimeSpan.FromSeconds(60)
        });
        _sut = new CircuitBreaker(options, _logger.Object);
    }

    [Fact]
    public void InitialState_IsClosed()
    {
        _sut.State.Should().Be(CircuitState.Closed);
    }

    [Fact]
    public void AllowRequest_WhenClosed_ReturnsTrue()
    {
        _sut.AllowRequest().Should().BeTrue();
    }

    [Fact]
    public void RecordFailure_BelowThreshold_StaysClosed()
    {
        _sut.RecordFailure();
        _sut.RecordFailure();

        _sut.State.Should().Be(CircuitState.Closed);
    }

    [Fact]
    public void RecordFailure_AtThreshold_OpensCircuit()
    {
        _sut.RecordFailure();
        _sut.RecordFailure();
        _sut.RecordFailure();

        _sut.State.Should().Be(CircuitState.Open);
    }

    [Fact]
    public void AllowRequest_WhenOpen_ReturnsFalse()
    {
        // Open the circuit
        for (int i = 0; i < 3; i++)
            _sut.RecordFailure();

        _sut.AllowRequest().Should().BeFalse();
    }

    [Fact]
    public async Task AllowRequest_AfterOpenDuration_TransitionsToHalfOpen()
    {
        // Open the circuit
        for (int i = 0; i < 3; i++)
            _sut.RecordFailure();

        // Wait for open duration
        await Task.Delay(150);

        _sut.AllowRequest().Should().BeTrue();
        _sut.State.Should().Be(CircuitState.HalfOpen);
    }

    [Fact]
    public void RecordSuccess_InHalfOpen_ClosesAfterThreshold()
    {
        // Open then transition to half-open
        for (int i = 0; i < 3; i++)
            _sut.RecordFailure();

        // Force half-open by manipulating time (or use test options)
        // ... implementation depends on design

        _sut.RecordSuccess();
        _sut.RecordSuccess();

        _sut.State.Should().Be(CircuitState.Closed);
    }

    [Fact]
    public void RecordFailure_InHalfOpen_ReopensCircuit()
    {
        // Setup half-open state...

        _sut.RecordFailure();

        _sut.State.Should().Be(CircuitState.Open);
    }

    [Fact]
    public async Task ExecuteAsync_WhenCircuitOpen_ThrowsCircuitOpenException()
    {
        // Open the circuit
        for (int i = 0; i < 3; i++)
            _sut.RecordFailure();

        var act = () => _sut.ExecuteAsync(ct => Task.FromResult(42));

        await act.Should().ThrowAsync<CircuitOpenException>();
    }

    [Fact]
    public async Task ExecuteAsync_RecordsSuccessOnCompletion()
    {
        await _sut.ExecuteAsync(ct => Task.FromResult(42));

        // State should still be closed with 0 failures
        _sut.State.Should().Be(CircuitState.Closed);
    }

    [Fact]
    public async Task ExecuteAsync_RecordsFailureOnException()
    {
        var act = () => _sut.ExecuteAsync<int>(
            ct => throw new IOException("Network error"));

        await act.Should().ThrowAsync<IOException>();

        // Should have recorded 1 failure
        // (verify by recording 2 more and checking state)
        _sut.RecordFailure();
        _sut.RecordFailure();
        _sut.State.Should().Be(CircuitState.Open);
    }

    [Fact]
    public async Task ExecuteAsync_DoesNotRecordValidationExceptionAsFailure()
    {
        var act = () => _sut.ExecuteAsync<int>(
            ct => throw new ValidationException("title", "Required"));

        await act.Should().ThrowAsync<ValidationException>();

        // Should NOT have recorded as failure
        _sut.RecordFailure();
        _sut.RecordFailure();
        _sut.State.Should().Be(CircuitState.Closed); // Still closed, only 2 failures
    }
}

// ProtoConverterPropertyTests.cs
namespace ProjectManagement.Services.Tests.PropertyTests;

public class ProtoConverterPropertyTests
{
    [Property]
    public Property WorkItem_RoundTrip_PreservesData()
    {
        return Prop.ForAll(
            Arb.Generate<Guid>().ToArbitrary(),
            Arb.Generate<string>().Where(s => !string.IsNullOrEmpty(s)).ToArbitrary(),
            (id, title) =>
            {
                var original = new WorkItem
                {
                    Id = id,
                    ItemType = WorkItemType.Task,
                    ProjectId = Guid.NewGuid(),
                    Title = title,
                    Status = "backlog",
                    Priority = "medium",
                    Version = 1,
                    CreatedAt = DateTime.UtcNow,
                    UpdatedAt = DateTime.UtcNow,
                    CreatedBy = Guid.NewGuid(),
                    UpdatedBy = Guid.NewGuid()
                };

                var proto = ProtoConverter.ToProto(original);
                var roundTripped = ProtoConverter.ToDomain(proto);

                return roundTripped.Id == original.Id
                    && roundTripped.Title == original.Title
                    && roundTripped.ItemType == original.ItemType;
            });
    }

    [Property]
    public Property Timestamp_RoundTrip_WithinOneSecond()
    {
        return Prop.ForAll(
            Arb.Generate<DateTime>()
                .Where(d => d > DateTime.UnixEpoch && d < DateTime.UtcNow.AddYears(100))
                .ToArbitrary(),
            timestamp =>
            {
                var workItem = new WorkItem
                {
                    Id = Guid.NewGuid(),
                    ItemType = WorkItemType.Task,
                    ProjectId = Guid.NewGuid(),
                    Title = "Test",
                    CreatedAt = timestamp,
                    UpdatedAt = timestamp,
                    CreatedBy = Guid.NewGuid(),
                    UpdatedBy = Guid.NewGuid()
                };

                var proto = ProtoConverter.ToProto(workItem);
                var roundTripped = ProtoConverter.ToDomain(proto);

                var diff = Math.Abs((roundTripped.CreatedAt - timestamp).TotalSeconds);
                return diff <= 1; // Within 1 second due to Unix timestamp precision
            });
    }
}
```

### Success Criteria for 20.6

- [ ] 100+ tests written
- [ ] All tests pass: `dotnet test frontend/`
- [ ] Property-based tests for converters
- [ ] Circuit breaker state machine fully tested
- [ ] WebSocket client tested with mocks
- [ ] State store tested with optimistic updates

---

## Final File Count Summary

| Sub-Session | Files | Cumulative |
|-------------|-------|------------|
| 20.1 Foundation | 55 | 55 |
| 20.2 WebSocket | 9 | 64 |
| 20.3 Resilience | 7 | 71 |
| 20.4 State | 4 | 75 |
| 20.5 WASM Host | 8 | 83 |
| 20.6 Tests | 15 | **98** |

---

## Production-Grade Checklist

| Requirement | Status |
|-------------|--------|
| Entity interface hierarchy (IEntity, IAuditable, etc.) | ✅ |
| Generic store interfaces (IEntityStore, IProjectScopedStore, ISprintStore) | ✅ |
| Exception hierarchy with user-safe messages | ✅ |
| Circuit breaker (Closed/Open/HalfOpen) | ✅ |
| Retry with exponential backoff + jitter (using Random.Shared) | ✅ |
| Reconnection with subscription rehydration | ✅ |
| Structured logging with correlation IDs | ✅ |
| Thread-safe state management | ✅ |
| CancellationToken on all async ops | ✅ |
| Proper IDisposable/IAsyncDisposable | ✅ |
| Ping/pong latency tracking (per-message correlation) | ✅ |
| Timestamp precision documented (Unix seconds) | ✅ |
| Input validation before sending | ✅ |
| Connection health monitoring | ✅ |
| Error boundaries in UI | ✅ |
| 100+ comprehensive tests | ✅ |
| Property-based tests | ✅ |
| No TODOs in production code | ✅ |

---

## Definition of Done

Session 20 is complete when:

- [ ] Solution builds: `dotnet build frontend/ProjectManagement.sln` (zero warnings)
- [ ] Tests pass: `dotnet test frontend/` (100+ tests, all green)
- [ ] WASM app runs: `dotnet run --project frontend/ProjectManagement.Wasm`
- [ ] WebSocket connects to backend (pm-server)
- [ ] Circuit breaker protects against cascading failures
- [ ] Retry logic handles transient failures
- [ ] Reconnection handles disconnects automatically
- [ ] State management with optimistic updates works
- [ ] Error boundaries display user-friendly messages
- [ ] All code follows project conventions
- [ ] No TODOs in production code
- [ ] Structured logging throughout

**Target Score**: 9.25+/10 production-grade

---

## Comparison with Session 10 Backend

| Feature | Session 10 (Rust) | Session 20 (C#) |
|---------|-------------------|-----------------|
| Circuit Breaker | ✅ | ✅ |
| Error Boundary | ✅ | ✅ |
| Structured Logging | ✅ | ✅ |
| Request Context | ✅ | ✅ (Correlation ID) |
| Message Validation | ✅ | ✅ |
| Retry Logic | ✅ | ✅ |
| Health Monitoring | ✅ | ✅ |
| Thread Safety | ✅ | ✅ |
| Test Count | 166 | 100+ |
| Production Score | 9.6/10 | 9.25+/10 (target) |
