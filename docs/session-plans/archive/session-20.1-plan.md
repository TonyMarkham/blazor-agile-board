# Session 20.1: Foundation

**Parent Plan**: [session-20-plan.md](session-20-plan.md)
**Target**: ~30k tokens
**Prerequisites**: Session 20.01 complete (FK constraints added)

---

## Scope

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

- [x] `dotnet build frontend/ProjectManagement.sln` succeeds with zero warnings
- [x] Protobuf C# classes generated correctly (445KB generated)
- [x] All models compile with nullable enabled
- [x] Validation framework works
- [x] ProtoConverter handles all edge cases (type aliases used to avoid naming collisions)

---

## Completion Status

**Status**: ✅ COMPLETE (2026-01-19)

**Actual Implementation:**
- Used .NET 10.0 (instead of planned 8.0)
- Latest stable packages: Protobuf 3.33.4, Grpc.Tools 2.76.0, Radzen 8.6.2, Microsoft packages 10.0.2
- Shared proto file from monorepo root (not copied)
- Type aliases in ProtoConverter to resolve naming conflicts between domain and proto types

**Files Created:** ~55 (matching plan)
**Build Result:** 0 warnings, 0 errors
**Commit:** c5cf698

---

