# Prompt for Fixing Subsequent Session Plans

Use this prompt in a clean Claude session to fix session plan documentation based on learnings from Session 20.2.

---

You are reviewing Blazor frontend session plans for a project. Session 20.2 was just completed and revealed several issues in the plan documentation. Please review and fix the remaining session plans (20.3, 20.4, 20.5, 20.6) for similar issues.

## Files to Fix

Read and fix these session plan documents:
- docs/session-plans/session-20.3-plan.md
- docs/session-plans/session-20.4-plan.md
- docs/session-plans/session-20.5-plan.md
- docs/session-plans/session-20.6-plan.md

## Context: What Exists After Session 20.2

The following types and patterns are established:

**Namespace conventions:**
- Protobuf types: `using Pm = ProjectManagement.Core.Proto;`
  - Use `Pm.WebSocketMessage`, `Pm.Ping`, `Pm.Pong`, etc.
  - NEVER use `ProjectManagement.Protos` (wrong namespace)

**Modern .NET WebSocket types:**
- Use `ValueTask` not `Task` for Send operations
- Use `ValueTask<ValueWebSocketReceiveResult>` not `Task<WebSocketReceiveResult>` for Receive
- The old types are obsolete

**Existing types from Session 20.1:**
- `ProjectManagement.Core.Models.*` - WorkItem, Sprint, Comment, CreateWorkItemRequest, UpdateWorkItemRequest, FieldChange, ConnectionState
- `ProjectManagement.Core.Interfaces.*` - IWebSocketClient, IConnectionHealth, IValidator
- `ProjectManagement.Core.Converters.ProtoConverter` - Conversion between domain and proto types
- `ProjectManagement.Core.Validation.*` - Validator implementations
- `ProjectManagement.Core.Exceptions.*` - ConnectionException, RequestTimeoutException, ServerRejectedException

**Existing types from Session 20.2:**
- `ProjectManagement.Services.WebSocket.WebSocketOptions`
- `ProjectManagement.Services.WebSocket.PendingRequest`
- `ProjectManagement.Services.WebSocket.IWebSocketConnection` (internal)
- `ProjectManagement.Services.WebSocket.BrowserWebSocketConnection` (internal)
- `ProjectManagement.Services.WebSocket.WebSocketClient`
- `ProjectManagement.Services.WebSocket.ConnectionHealthTracker` (internal)

## Issues Found in Session 20.2 Plan

1. **Wrong protobuf namespace**: Used `ProjectManagement.Protos` instead of `ProjectManagement.Core.Proto`
2. **Missing using statements**: Files didn't show required imports at the top
3. **Inconsistent type references**: Sometimes bare `WebSocketMessage`, sometimes `Pm.WebSocketMessage`
4. **Obsolete .NET types**: Used old `Task<WebSocketReceiveResult>` instead of modern `ValueTask<ValueWebSocketReceiveResult>`
5. **Dependency order wrong**: WebSocketClient code came before ConnectionHealthTracker even though it depends on it
6. **Missing interface methods**: Showed partial implementations without stub methods for remaining interface members

## What to Fix

For each session plan file (20.3 through 20.6):

1. **Add using statements** to every code block that needs them:
   ```csharp
   using System.Collections.Concurrent;
   using System.Net.WebSockets;
   using Microsoft.Extensions.Logging;
   using ProjectManagement.Core.Models;
   using ProjectManagement.Core.Interfaces;
   using Pm = ProjectManagement.Core.Proto;
   ```

2. **Fix all protobuf type references**:
   - Change `WebSocketMessage` → `Pm.WebSocketMessage`
   - Change `WebSocketMessage.PayloadOneofCase` → `Pm.WebSocketMessage.PayloadOneofCase`
   - Ensure `using Pm = ProjectManagement.Core.Proto;` is present

3. **Check dependency order**:
   - If Phase X uses a type defined in Phase Y, make sure Phase Y comes first
   - Flag any circular dependencies

4. **Use modern .NET types**:
   - WebSocket operations should use `ValueTask` not `Task` for sends
   - Receive should return `ValueTask<ValueWebSocketReceiveResult>` not `Task<WebSocketReceiveResult>`

5. **Check for missing implementations**:
   - If a class implements an interface, ensure all methods are shown or have stubs

6. **Verify type availability**:
   - Only use types that exist in Session 20.1 or 20.2
   - If introducing new types, define them before using them

## Output Format

For each file you fix, show:
1. File name
2. List of issues found
3. The corrected sections (only show what changed, not the entire file)

If a file has no issues, just say "session-20.X-plan.md: No issues found"

Please proceed to review and fix all four session plan files.
