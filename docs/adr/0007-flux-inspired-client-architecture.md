# ADR-0007: Flux-Inspired Client Architecture with Server-Authoritative State

## Status
Accepted

## Context
When building the Blazor WebAssembly frontend, we needed to decide on a client-side architecture pattern. Traditional options include:

- **MVC (Model-View-Controller)**: Controller handles user input and updates model
- **MVVM (Model-View-ViewModel)**: ViewModel contains presentation logic and commands

However, our architecture has a key constraint: **all business logic lives on the server** (Rust/Axum). The client is essentially a "smart terminal" that:
- Renders UI based on local state
- Sends commands to the server
- Reacts to server events

Neither MVC nor MVVM fit well because:
- MVC implies controllers making decisions the server should make
- MVVM implies ViewModels containing business logic and validation

We needed a pattern that embraces unidirectional data flow with server-authoritative state.

## Decision
We adopt a **Flux-inspired architecture** adapted for server-authoritative state:

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENT (Blazor WASM)                     │
│                                                                  │
│   ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐     │
│   │   UI    │───▶│ Action  │───▶│ Server  │───▶│  Store  │──┐  │
│   │Component│    │(Command)│    │   ↕     │    │ (State) │  │  │
│   └─────────┘    └─────────┘    └─────────┘    └─────────┘  │  │
│        ▲                             │              │        │  │
│        │                             │              ▼        │  │
│        │                        ┌─────────┐   ┌──────────┐  │  │
│        │                        │  Event  │◀──│ViewModel │  │  │
│        │                        │(Server) │   │(UI State)│  │  │
│        │                        └─────────┘   └──────────┘  │  │
│        │                                                     │  │
│        └─────────────────────────────────────────────────────┘  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                       SERVER (Rust/Axum)                         │
│                                                                  │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│   │   Command   │───▶│  Business   │───▶│   Event     │        │
│   │   Handler   │    │   Logic     │    │  Broadcast  │        │
│   └─────────────┘    └─────────────┘    └─────────────┘        │
│                              │                                   │
│                              ▼                                   │
│                       ┌─────────────┐                           │
│                       │   SQLite    │                           │
│                       │  (Source of │                           │
│                       │   Truth)    │                           │
│                       └─────────────┘                           │
└──────────────────────────────────────────────────────────────────┘
```

### Core Principles

#### 1. Unidirectional Data Flow
Data flows in one direction: UI → Command → Server → Event → Store → UI

- **Commands** are requests to change state (sent to server)
- **Events** are notifications of state changes (received from server)
- **Stores** hold local copies of server state
- **UI** renders based on store state

#### 2. Server-Authoritative State
The server is the single source of truth:

- Client state is a **cache**, not the authority
- All mutations go through the server
- Server validates, persists, then broadcasts events
- Client updates local state only after server confirmation

#### 3. Optimistic Updates with Rollback
For responsive UX, we apply changes optimistically:

```csharp
// In WorkItemStore
public async Task UpdateAsync(WorkItem item)
{
    var original = _items[item.Id];

    // 1. Apply optimistically
    _items[item.Id] = item;
    _pendingUpdates[item.Id] = original; // Track for rollback
    NotifyChange();

    // 2. Send to server
    var result = await _webSocket.SendCommandAsync(new UpdateWorkItemCommand(item));

    // 3. On success: clear pending, on failure: rollback
    if (result.Success)
        _pendingUpdates.Remove(item.Id);
    else
        _items[item.Id] = original; // Rollback

    NotifyChange();
}
```

#### 4. ViewModels as Presentation Wrappers
ViewModels do NOT contain business logic. They compose:

- **Domain model** (the actual data from server)
- **UI state** (pending sync, selection, expanded/collapsed)

```csharp
public interface IViewModel<out TModel> where TModel : class
{
    TModel Model { get; }      // Domain data
    bool IsPendingSync { get; } // UI state
}

public sealed record WorkItemViewModel : IViewModel<WorkItem>
{
    public required WorkItem Model { get; init; }
    public bool IsPendingSync { get; init; }

    // Convenience accessors (no logic, just delegation)
    public Guid Id => Model.Id;
    public string Title => Model.Title;
}
```

### Component Responsibilities

| Layer | Responsibility | Does NOT Do |
|-------|---------------|-------------|
| **UI Components** | Render state, capture user intent | Business logic, direct API calls |
| **Stores** | Hold state, notify subscribers, track pending | Validation, persistence |
| **ViewModels** | Compose domain + UI state | Business logic, commands |
| **WebSocket Client** | Send commands, receive events | State management |
| **Server** | Validate, persist, broadcast | UI concerns |

### Store Interface Pattern

```csharp
public interface IWorkItemStore
{
    // Queries (return ViewModels)
    WorkItemViewModel? GetById(Guid id);
    IEnumerable<WorkItemViewModel> GetByProject(Guid projectId);
    IEnumerable<WorkItemViewModel> GetByType(WorkItemType type);

    // Commands (send to server, update optimistically)
    Task CreateAsync(WorkItem item);
    Task UpdateAsync(WorkItem item);
    Task DeleteAsync(Guid id);

    // Subscriptions
    event Action OnChange;
}
```

### Event Handling

Server events update stores, which notify UI:

```csharp
// WebSocket receives event
_webSocket.OnWorkItemUpdated += (item) =>
{
    _workItemStore.ApplyServerUpdate(item); // Updates internal state
    // Store calls OnChange, UI re-renders
};
```

## Consequences

### Positive
- **Clear separation**: UI is purely presentational, server owns all logic
- **Predictable state**: Unidirectional flow makes debugging easier
- **Optimistic UX**: Users see immediate feedback while server processes
- **Testable**: Stores can be mocked, UI tested in isolation
- **Scalable pattern**: Same architecture works for any entity type
- **Offline-capable**: Stores can queue commands when disconnected

### Negative
- **Boilerplate**: Each entity needs Store + ViewModel
- **Learning curve**: Developers familiar with MVVM may need adjustment
- **Indirection**: More layers between user action and result

### Trade-offs vs Traditional Patterns

| Aspect | This Pattern | Traditional MVVM |
|--------|-------------|------------------|
| Business logic location | Server only | ViewModel |
| Validation | Server | ViewModel + Server |
| State authority | Server | Mixed |
| Offline mutations | Queued commands | Local changes |
| Complexity | Distributed | Concentrated in VM |

## Related Decisions
- [ADR-0005](0005-websocket-with-protobuf.md): WebSocket as transport for commands/events
- [ADR-0006](0006-single-tenant-desktop-first.md): Single server process simplifies event routing

## References
- [Flux Architecture](https://facebook.github.io/flux/docs/in-depth-overview/) - Original pattern from Facebook
- [Redux](https://redux.js.org/understanding/thinking-in-redux/three-principles) - Popular implementation
- [CQRS](https://martinfowler.com/bliki/CQRS.html) - Command Query Responsibility Segregation
