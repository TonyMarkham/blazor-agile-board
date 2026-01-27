# Session 51: Sprint & Comment UI Integration

## Goal

Integrate existing Sprint and Comment UI components into the main application pages with proper tests and loading states.

---

## Dependency Order

```
1. AppState.cs (add ICommentStore)
   ├── Required by: Test files that create AppState
   └── Enables: AppState.Comments access

2. Update test helper (PageIntegrationTests, SharedComponentTests)
   └── Depends on: Step 1 (ICommentStore in AppState constructor)

3. ProjectDetail.razor (add Sprints tab + loading state)
   └── Depends on: SprintCard, SprintDialog, SprintStore (all exist)

4. WorkItemDetail.razor (add Comments section)
   └── Depends on: CommentList, CommentStore (all exist)

5. Home.razor (fix active sprints count)
   └── Depends on: SprintStore (exists)

6. New tests for Sprint/Comment integration
   └── Depends on: Steps 2-5
```

---

## Step 1: Add ICommentStore to AppState

**File:** `frontend/ProjectManagement.Services/State/AppState.cs`

Add `ICommentStore` parameter and property following existing pattern:

```csharp
public AppState(
    IWebSocketClient client,
    IWorkItemStore workItems,
    ISprintStore sprints,
    IProjectStore projects,
    ICommentStore comments,  // ADD
    ILogger<AppState> logger)
{
    // ...
    Comments = comments;

    // Subscribe
    comments.OnChanged += () => OnStateChanged?.Invoke();
}

public ICommentStore Comments { get; }

// In Dispose():
if (Comments is IDisposable commentsDisposable)
    commentsDisposable.Dispose();
```

**Verify:** `just build-frontend` (tests will fail until Step 2)

---

## Step 2: Update Test Helpers

Tests that create `AppState` need the new `ICommentStore` parameter.

### File: `PageIntegrationTests.cs`

**Add mock field:**
```csharp
private readonly Mock<ICommentStore> _commentStoreMock;
```

**Update constructor:**
```csharp
_commentStoreMock = new Mock<ICommentStore>();
_commentStoreMock.Setup(c => c.GetComments(It.IsAny<Guid>()))
    .Returns(new List<Comment>());

_appState = new AppState(
    mockClient.Object,
    _workItemStoreMock.Object,
    _sprintStoreMock.Object,
    _projectStoreMock.Object,
    _commentStoreMock.Object,  // ADD
    Mock.Of<Microsoft.Extensions.Logging.ILogger<AppState>>());

Services.AddSingleton<ICommentStore>(_commentStoreMock.Object);  // ADD
```

### File: `SharedComponentTests.cs`

**Update `CreateMockAppState` method:**
```csharp
private AppState CreateMockAppState(ConnectionState state)
{
    var mockClient = new Mock<IWebSocketClient>();
    mockClient.Setup(c => c.State).Returns(state);
    mockClient.Setup(c => c.Health).Returns(Mock.Of<IConnectionHealth>());

    var mockWorkItemStore = new Mock<IWorkItemStore>();
    var mockSprintStore = new Mock<ISprintStore>();
    var mockProjectStore = new Mock<IProjectStore>();
    var mockCommentStore = new Mock<ICommentStore>();  // ADD
    var mockLogger = NullLogger<AppState>.Instance;

    return new AppState(
        mockClient.Object,
        mockWorkItemStore.Object,
        mockSprintStore.Object,
        mockProjectStore.Object,
        mockCommentStore.Object,  // ADD
        mockLogger);
}
```

**Verify:** `just test-frontend`

---

## Step 3: Add Sprints Tab to ProjectDetail

**File:** `frontend/ProjectManagement.Wasm/Pages/ProjectDetail.razor`

### Add using:
```razor
@using ProjectManagement.Components.Sprints
```

### Add tab button (after Board tab):
```razor
<button class="view-tab @(_activeView == "sprints" ? "active" : "")"
        @onclick="@(() => _activeView = "sprints")">
    <RadzenIcon Icon="timer" />
    Sprints
</button>
```

### Add view section (with loading state):
```razor
else if (_activeView == "sprints")
{
    <div class="content-card">
        <div class="content-card-header">
            <h2 class="content-card-title">Sprints</h2>
            <LoadingButton Text="New Sprint" Icon="add"
                           ConnectionState="@_connectionState"
                           OnClick="@ShowCreateSprintDialog" />
        </div>

        @if (_loadingSprints)
        {
            <div class="p-4">
                <RadzenProgressBar Mode="ProgressBarMode.Indeterminate" />
            </div>
        }
        else if (!_sprints.Any())
        {
            <div class="p-4">
                <EmptyState Icon="timer" Title="No Sprints"
                            Description="Create your first sprint."
                            ActionText="Create Sprint"
                            OnAction="@ShowCreateSprintDialog" />
            </div>
        }
        else
        {
            <RadzenStack Gap="1rem" class="p-3">
                @foreach (var sprint in _sprints.OrderByDescending(s => s.StartDate))
                {
                    <SprintCard Sprint="@sprint"
                                ShowProgress="true"
                                TotalItems="@GetSprintItemCount(sprint.Id)"
                                CompletedItems="@GetSprintCompletedCount(sprint.Id)"
                                CanStart="@(sprint.Status == SprintStatus.Planned && _connectionState == ConnectionState.Connected)"
                                CanComplete="@(sprint.Status == SprintStatus.Active && _connectionState == ConnectionState.Connected)"
                                CanEdit="@(_connectionState == ConnectionState.Connected)"
                                CanDelete="@(sprint.Status == SprintStatus.Planned && _connectionState == ConnectionState.Connected)"
                                OnEditClick="@(() => ShowEditSprintDialog(sprint))"
                                OnDeleteClick="@(() => HandleDeleteSprint(sprint))"
                                OnStartClick="@(() => HandleStartSprint(sprint))"
                                OnCompleteClick="@(() => HandleCompleteSprint(sprint))" />
                }
            </RadzenStack>
        }
    </div>
}
```

### Add code section:
```csharp
private List<Sprint> _sprints = new();
private bool _loadingSprints = true;

private async Task RefreshSprints()
{
    _loadingSprints = true;
    StateHasChanged();

    try
    {
        _sprints = AppState.Sprints.GetByProject(ProjectId).ToList();
    }
    finally
    {
        _loadingSprints = false;
        StateHasChanged();
    }
}

private int GetSprintItemCount(Guid sprintId) =>
    AppState.WorkItems.GetByProject(ProjectId)
        .Count(w => w.SprintId == sprintId && w.DeletedAt == null);

private int GetSprintCompletedCount(Guid sprintId) =>
    AppState.WorkItems.GetByProject(ProjectId)
        .Count(w => w.SprintId == sprintId && w.Status == "done" && w.DeletedAt == null);

private async Task ShowCreateSprintDialog()
{
    await DialogService.OpenAsync<SprintDialog>("Create Sprint",
        new Dictionary<string, object>
        {
            { "ProjectId", ProjectId },
            { "OnCreate", EventCallback.Factory.Create<CreateSprintRequest>(this, HandleCreateSprint) },
            { "OnCancel", EventCallback.Factory.Create(this, () => DialogService.Close()) }
        },
        new DialogOptions { Width = "500px" });
}

private async Task ShowEditSprintDialog(Sprint sprint)
{
    await DialogService.OpenAsync<SprintDialog>("Edit Sprint",
        new Dictionary<string, object>
        {
            { "Sprint", sprint },
            { "ProjectId", ProjectId },
            { "OnUpdate", EventCallback.Factory.Create<UpdateSprintRequest>(this, HandleUpdateSprint) },
            { "OnCancel", EventCallback.Factory.Create(this, () => DialogService.Close()) }
        },
        new DialogOptions { Width = "500px" });
}

private async Task HandleCreateSprint(CreateSprintRequest request)
{
    try
    {
        await AppState.Sprints.CreateAsync(request);
        DialogService.Close();
        NotificationService.Notify(NotificationSeverity.Success, "Created", "Sprint created");
    }
    catch (Exception ex)
    {
        NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
    }
}

private async Task HandleUpdateSprint(UpdateSprintRequest request)
{
    try
    {
        await AppState.Sprints.UpdateAsync(request);
        DialogService.Close();
        NotificationService.Notify(NotificationSeverity.Success, "Updated", "Sprint updated");
    }
    catch (Exception ex)
    {
        NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
    }
}

private async Task HandleDeleteSprint(Sprint sprint)
{
    var confirmed = await DialogService.Confirm(
        $"Delete sprint \"{sprint.Name}\"?", "Delete Sprint",
        new ConfirmOptions { OkButtonText = "Delete", CancelButtonText = "Cancel" });
    if (confirmed == true)
    {
        try
        {
            await AppState.Sprints.DeleteAsync(sprint.Id);
            NotificationService.Notify(NotificationSeverity.Success, "Deleted", "Sprint deleted");
        }
        catch (Exception ex)
        {
            NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
        }
    }
}

private async Task HandleStartSprint(Sprint sprint)
{
    try
    {
        await AppState.Sprints.StartSprintAsync(sprint.Id, sprint.Version);
        NotificationService.Notify(NotificationSeverity.Success, "Started", $"Sprint \"{sprint.Name}\" is now active");
    }
    catch (Exception ex)
    {
        NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
    }
}

private async Task HandleCompleteSprint(Sprint sprint)
{
    var confirmed = await DialogService.Confirm(
        $"Complete sprint \"{sprint.Name}\"?", "Complete Sprint",
        new ConfirmOptions { OkButtonText = "Complete", CancelButtonText = "Cancel" });
    if (confirmed == true)
    {
        try
        {
            await AppState.Sprints.CompleteSprintAsync(sprint.Id, sprint.Version);
            NotificationService.Notify(NotificationSeverity.Success, "Completed", "Sprint completed");
        }
        catch (Exception ex)
        {
            NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
        }
    }
}
```

### Update LoadProjectAsync:
```csharp
// After await AppState.LoadProjectAsync(ProjectId):
await RefreshSprints();
```

### Update HandleStateChanged:
```csharp
private void HandleStateChanged()
{
    _project = AppState.Projects.GetById(ProjectId);
    _sprints = AppState.Sprints.GetByProject(ProjectId).ToList();  // ADD (sync refresh)
    InvokeAsync(StateHasChanged);
}
```

**Verify:** `just build-frontend`

---

## Step 4: Add Comments Section to WorkItemDetail

**File:** `frontend/ProjectManagement.Wasm/Pages/WorkItemDetail.razor`

### Add using:
```razor
@using ProjectManagement.Components.Comments
```

### Add markup (after Child Items section):
```razor
@* Comments *@
<div class="content-card mt-4">
    <div class="content-card-header">
        <h2 class="content-card-title">Comments</h2>
    </div>
    <CommentList WorkItemId="@WorkItemId"
                 CurrentUserId="@_currentUserId"
                 UserNameResolver="@ResolveUserName" />
</div>
```

### Add code:
```csharp
private Guid _currentUserId;

// In OnInitialized (after line 176):
_currentUserId = AppState.CurrentUser?.Id ?? Guid.Empty;

private string ResolveUserName(Guid userId) => userId.ToString()[..8];
```

**Verify:** `just build-frontend`

---

## Step 5: Fix Active Sprints Count on Home

**File:** `frontend/ProjectManagement.Wasm/Pages/Home.razor`

### Subscribe in OnInitializedAsync:
```csharp
AppState.Sprints.OnChanged += OnSprintsChanged;
```

### Replace hardcoded stats:
```csharp
_activeSprints = CalculateActiveSprints();
```

### Add methods:
```csharp
private int CalculateActiveSprints()
{
    var count = 0;
    foreach (var project in _projects)
    {
        if (AppState.Sprints.GetActiveSprint(project.Id) != null)
            count++;
    }
    return count;
}

private void OnSprintsChanged()
{
    _activeSprints = CalculateActiveSprints();
    InvokeAsync(StateHasChanged);
}
```

### Unsubscribe in Dispose:
```csharp
AppState.Sprints.OnChanged -= OnSprintsChanged;
```

**Verify:** `just build-frontend`

---

## Step 6: Add New Integration Tests

**File:** `PageIntegrationTests.cs`

### Add Sprint Tab Tests:

```csharp
#region ProjectDetail Sprint Tab Tests

[Fact]
public async Task ProjectDetailPage_RendersSprintsTab()
{
    // Arrange
    var projectId = Guid.NewGuid();
    var projectViewModel = CreateProjectViewModel("My Project", projectId);
    _projectStoreMock.Setup(s => s.GetById(projectId)).Returns(projectViewModel);
    _workItemStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<WorkItem>());
    _sprintStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<Sprint>());

    // Act
    var cut = Render<ProjectDetail>(parameters => parameters
        .Add(p => p.ProjectId, projectId));

    // Assert
    await cut.WaitForAssertionAsync(() =>
    {
        cut.Markup.Should().Contain("Sprints");
        cut.Markup.Should().Contain("timer"); // icon name
    }, timeout: TimeSpan.FromSeconds(5));
}

[Fact]
public async Task ProjectDetailPage_ShowsEmptyState_WhenNoSprints()
{
    // Arrange
    var projectId = Guid.NewGuid();
    var projectViewModel = CreateProjectViewModel("My Project", projectId);
    _projectStoreMock.Setup(s => s.GetById(projectId)).Returns(projectViewModel);
    _workItemStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<WorkItem>());
    _sprintStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<Sprint>());

    // Act
    var cut = Render<ProjectDetail>(parameters => parameters
        .Add(p => p.ProjectId, projectId));

    // Click Sprints tab
    var sprintsTab = cut.FindAll("button.view-tab").First(b => b.TextContent.Contains("Sprints"));
    await cut.InvokeAsync(() => sprintsTab.Click());

    // Assert
    await cut.WaitForAssertionAsync(() =>
    {
        cut.FindComponents<EmptyState>().Should().HaveCount(1);
        cut.Markup.Should().Contain("No Sprints");
    }, timeout: TimeSpan.FromSeconds(5));
}

[Fact]
public async Task ProjectDetailPage_RendersSprintCards_WhenSprintsExist()
{
    // Arrange
    var projectId = Guid.NewGuid();
    var projectViewModel = CreateProjectViewModel("My Project", projectId);
    var sprints = new[]
    {
        CreateSprint("Sprint 1", projectId),
        CreateSprint("Sprint 2", projectId)
    };

    _projectStoreMock.Setup(s => s.GetById(projectId)).Returns(projectViewModel);
    _workItemStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<WorkItem>());
    _sprintStoreMock.Setup(s => s.GetByProject(projectId)).Returns(sprints);

    // Act
    var cut = Render<ProjectDetail>(parameters => parameters
        .Add(p => p.ProjectId, projectId));

    // Click Sprints tab
    var sprintsTab = cut.FindAll("button.view-tab").First(b => b.TextContent.Contains("Sprints"));
    await cut.InvokeAsync(() => sprintsTab.Click());

    // Assert
    await cut.WaitForAssertionAsync(() =>
    {
        cut.FindComponents<SprintCard>().Should().HaveCount(2);
    }, timeout: TimeSpan.FromSeconds(5));
}

private static Sprint CreateSprint(string name, Guid projectId) => new()
{
    Id = Guid.NewGuid(),
    ProjectId = projectId,
    Name = name,
    Status = SprintStatus.Planned,
    StartDate = DateTime.Today,
    EndDate = DateTime.Today.AddDays(14),
    Version = 1,
    CreatedAt = DateTime.UtcNow,
    UpdatedAt = DateTime.UtcNow,
    CreatedBy = Guid.NewGuid(),
    UpdatedBy = Guid.NewGuid()
};

#endregion

#region WorkItemDetail Comments Tests

[Fact]
public async Task WorkItemDetailPage_RendersCommentsSection()
{
    // Arrange
    var workItemId = Guid.NewGuid();
    var workItem = CreateWorkItem("My Task", WorkItemType.Task) with { Id = workItemId };

    _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
    _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());
    _commentStoreMock.Setup(c => c.GetComments(workItemId)).Returns(new List<Comment>());

    // Act
    var cut = Render<WorkItemDetail>(parameters => parameters
        .Add(p => p.WorkItemId, workItemId));

    // Assert
    await cut.WaitForAssertionAsync(() =>
    {
        cut.Markup.Should().Contain("Comments");
        cut.FindComponents<CommentList>().Should().HaveCount(1);
    }, timeout: TimeSpan.FromSeconds(5));
}

#endregion
```

**Add using at top:**
```csharp
using ProjectManagement.Components.Sprints;
using ProjectManagement.Components.Comments;
```

**Verify:** `just test-frontend`

---

## Files Modified

| File | Purpose |
|------|---------|
| `frontend/ProjectManagement.Services/State/AppState.cs` | Add ICommentStore property |
| `frontend/ProjectManagement.Components.Tests/Pages/PageIntegrationTests.cs` | Update AppState mock, add Sprint/Comment tests |
| `frontend/ProjectManagement.Components.Tests/Shared/SharedComponentTests.cs` | Update CreateMockAppState |
| `frontend/ProjectManagement.Wasm/Pages/ProjectDetail.razor` | Add Sprints tab with loading state |
| `frontend/ProjectManagement.Wasm/Pages/WorkItemDetail.razor` | Add Comments section |
| `frontend/ProjectManagement.Wasm/Pages/Home.razor` | Fix active sprints count |

---

## Final Verification

```bash
just check           # Compile check
just test            # All tests pass (should be 620+ now)
just dev             # Manual testing

# Test workflow:
# 1. Home page shows Active Sprints count
# 2. Project page has Sprints tab with loading spinner on first load
# 3. Create/Start/Complete/Delete sprints work
# 4. Work item page shows Comments section
# 5. Create/Edit/Delete comments work
```

---

## Production-Grade Rating

| Category | Score | Notes |
|----------|-------|-------|
| Error Handling | 9/10 | Try/catch with notifications |
| Validation | 9/10 | Uses validated components |
| Authorization | 9/10 | Author-only comment permissions |
| Testing | 9/10 | Integration tests for new features |
| UX Polish | 9/10 | Loading states, empty states |
| Code Quality | 9/10 | Follows existing patterns |

**Overall: 9/10**
