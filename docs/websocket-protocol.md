# WebSocket Protocol Documentation

This document describes the real-time communication protocol using WebSocket and Protocol Buffers.

## Overview

- **Protocol**: WebSocket (bidirectional)
- **Serialization**: Protocol Buffers (protobuf3)
- **Message Format**: Binary protobuf messages
- **Connection**: Persistent, authenticated WebSocket connection

---

## Connection Lifecycle

### 1. Connection Establishment

```
Client → Server: WebSocket handshake with JWT token
GET /ws HTTP/1.1
Authorization: Bearer <jwt_token>
Upgrade: websocket
```

**Server validates:**
- JWT token signature
- Token expiration
- Extract `tenant_id` from claims
- Extract `user_id` from claims

**On success:**
- WebSocket connection established
- Client added to tenant's broadcast channel
- Server does NOT send initial subscription confirmation (client must subscribe)

**On failure:**
- HTTP 401 Unauthorized
- Connection closed

### 2. Subscription

Client must explicitly subscribe to resources they want updates for:

```protobuf
WebSocketMessage {
  message_id: "uuid-1",
  timestamp: 1234567890,
  payload: Subscribe {
    project_ids: ["project-1", "project-2"],
    sprint_ids: ["sprint-1"],
    work_item_ids: []  // Empty = don't subscribe to individual items
  }
}
```

**Server responds:**

```protobuf
WebSocketMessage {
  message_id: "uuid-2",
  timestamp: 1234567891,
  payload: SubscriptionConfirmed {
    subscribed_project_ids: ["project-1", "project-2"],
    subscribed_sprint_ids: ["sprint-1"],
    subscribed_work_item_ids: []
  }
}
```

### 3. Receiving Updates

Once subscribed, client receives all events related to those resources:

```protobuf
WebSocketMessage {
  message_id: "uuid-3",
  timestamp: 1234567892,
  payload: WorkItemUpdated {
    work_item: WorkItem { ... },
    changes: [
      FieldChange {
        field_name: "status",
        old_value: "in_progress",
        new_value: "done"
      }
    ],
    user_id: "user-123"
  }
}
```

### 4. Heartbeat

Client and server exchange ping/pong to detect dead connections:

**Every 30 seconds:**

```protobuf
// Client → Server
WebSocketMessage {
  message_id: "ping-1",
  timestamp: 1234567890,
  payload: Ping {
    timestamp: 1234567890
  }
}

// Server → Client
WebSocketMessage {
  message_id: "pong-1",
  timestamp: 1234567891,
  payload: Pong {
    timestamp: 1234567891,
    client_timestamp: 1234567890  // Echo client's timestamp
  }
}
```

**If no pong received within 60 seconds:**
- Client assumes connection dead
- Client initiates reconnection

### 5. Disconnection

**Graceful disconnect:**
```protobuf
WebSocketMessage {
  message_id: "unsub-1",
  timestamp: 1234567890,
  payload: Unsubscribe {
    project_ids: ["project-1", "project-2"],
    sprint_ids: ["sprint-1"],
    work_item_ids: []
  }
}
```

Then close WebSocket connection.

**Server cleanup:**
- Remove client from broadcast channels
- Log disconnect event

---

## Message Types

### Work Item Events

| Event | Direction | Description |
|-------|-----------|-------------|
| `WorkItemCreated` | Server → Client | New work item created |
| `WorkItemUpdated` | Server → Client | Work item fields changed |
| `WorkItemDeleted` | Server → Client | Work item soft deleted |
| `WorkItemMoved` | Server → Client | Work item moved to new parent/sprint |

**Example - Task Status Change:**

```protobuf
WorkItemUpdated {
  work_item: WorkItem {
    id: "task-123",
    item_type: WORK_ITEM_TYPE_TASK,
    title: "Fix login bug",
    status: "done",  // Changed from "in_progress"
    assignee_id: "user-456",
    updated_at: 1234567890,
    updated_by: "user-456"
  },
  changes: [
    FieldChange {
      field_name: "status",
      old_value: "in_progress",
      new_value: "done"
    }
  ],
  user_id: "user-456"
}
```

**Example - Task Moved to Sprint:**

```protobuf
WorkItemMoved {
  work_item_id: "task-123",
  new_parent_id: "story-456",  // Parent unchanged
  new_position: 2,              // New position in parent
  new_sprint_id: "sprint-789",  // Moved from backlog to sprint
  user_id: "user-123"
}
```

### Sprint Events

| Event | Direction | Description |
|-------|-----------|-------------|
| `SprintCreated` | Server → Client | New sprint created |
| `SprintUpdated` | Server → Client | Sprint details changed |
| `SprintDeleted` | Server → Client | Sprint deleted |
| `SprintStarted` | Server → Client | Sprint status changed to active |
| `SprintCompleted` | Server → Client | Sprint completed with stats |

**Example - Sprint Started:**

```protobuf
SprintStarted {
  sprint_id: "sprint-123",
  start_date: 1234567890,
  user_id: "user-456"
}
```

### Comment Events

| Event | Direction | Description |
|-------|-----------|-------------|
| `CommentAdded` | Server → Client | New comment posted |
| `CommentUpdated` | Server → Client | Comment edited |
| `CommentDeleted` | Server → Client | Comment deleted |

**Example - Comment Added:**

```protobuf
CommentAdded {
  comment: Comment {
    id: "comment-123",
    work_item_id: "task-456",
    content: "I've fixed the issue, ready for review.",
    created_at: 1234567890,
    created_by: "user-789"
  },
  user_id: "user-789"
}
```

### Time Tracking Events

| Event | Direction | Description |
|-------|-----------|-------------|
| `TimeEntryStarted` | Server → Client | Timer started on task |
| `TimeEntryStopped` | Server → Client | Timer stopped, duration recorded |
| `TimeEntryUpdated` | Server → Client | Time entry manually adjusted |
| `TimeEntryDeleted` | Server → Client | Time entry removed |

**Example - Timer Started:**

```protobuf
TimeEntryStarted {
  time_entry: TimeEntry {
    id: "entry-123",
    work_item_id: "task-456",
    user_id: "user-789",
    started_at: 1234567890,
    ended_at: null,  // Still running
    duration_seconds: null,
    description: "Working on authentication logic",
    created_at: 1234567890
  },
  user_id: "user-789"
}
```

### Dependency Events

| Event | Direction | Description |
|-------|-----------|-------------|
| `DependencyCreated` | Server → Client | New dependency/blocker added |
| `DependencyDeleted` | Server → Client | Dependency removed |

**Example - Dependency Created:**

```protobuf
DependencyCreated {
  dependency: Dependency {
    id: "dep-123",
    blocking_item_id: "task-456",  // This must complete first
    blocked_item_id: "task-789",   // This is blocked
    dependency_type: DEPENDENCY_TYPE_BLOCKS,
    created_at: 1234567890,
    created_by: "user-123"
  },
  user_id: "user-123"
}
```

### Presence & Collaboration

| Event | Direction | Description |
|-------|-----------|-------------|
| `UserPresence` | Server → Client | User online/offline/viewing |
| `UserTyping` | Server → Client | User typing in comment section |

**Example - User Viewing Task:**

```protobuf
UserPresence {
  user_id: "user-123",
  status: PRESENCE_STATUS_ONLINE,
  viewing_item_id: "task-456",  // Currently viewing this task
  editing_item_id: null
}
```

**Example - User Typing:**

```protobuf
UserTyping {
  user_id: "user-123",
  work_item_id: "task-456",
  is_typing: true  // Started typing (false when stopped)
}
```

---

## Error Handling

### Error Response Format

```protobuf
ErrorResponse {
  error_code: "VALIDATION_ERROR",
  error_message: "Work item title cannot be empty",
  request_message_id: "uuid-123",  // Correlates to failed request
  field_name: "title"
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `AUTHENTICATION_FAILED` | JWT token invalid or expired |
| `AUTHORIZATION_FAILED` | User lacks permission for action |
| `VALIDATION_ERROR` | Invalid data in request |
| `NOT_FOUND` | Referenced entity doesn't exist |
| `CONFLICT` | Operation conflicts with current state |
| `RATE_LIMIT_EXCEEDED` | Too many messages sent |
| `INTERNAL_ERROR` | Server-side error |

---

## Broadcasting Strategy

### Server-Side Architecture

```rust
// Per-tenant broadcast channel
struct TenantBroadcast {
    tenant_id: String,
    sender: broadcast::Sender<WebSocketMessage>,
}

// Each connected client
struct ClientConnection {
    user_id: String,
    tenant_id: String,
    subscriptions: HashSet<String>,  // project/sprint/item IDs
    receiver: broadcast::Receiver<WebSocketMessage>,
}
```

### Filtering Messages

Server broadcasts all events to tenant channel, but clients only receive messages for their subscriptions:

```rust
// Server broadcasts event
tenant_broadcast.send(message);

// Each client filters
if message.relates_to(client.subscriptions) {
    client.send(message);
}
```

**Example:**

- User subscribed to `project-1` and `sprint-5`
- Event: `WorkItemUpdated` for task in `project-1` → **Received**
- Event: `WorkItemUpdated` for task in `project-2` → **Filtered out**
- Event: `SprintUpdated` for `sprint-5` → **Received**

---

## Reconnection Strategy

### Client-Side Reconnection Logic

```
1. Detect disconnect (missed pong, connection error, etc.)
2. Wait 1 second (exponential backoff: 1s, 2s, 4s, 8s, max 30s)
3. Attempt reconnection with same JWT token
4. On success:
   a. Re-subscribe to same resources
   b. Fetch missed updates via REST API (query activity log since last_seen_timestamp)
   c. Reconcile local state
5. On failure:
   a. If 401: Token expired, redirect to login
   b. Otherwise: Retry with backoff
```

### Handling Missed Updates

Client tracks `last_received_timestamp`:

```
On reconnect:
1. GET /api/v1/activity?since={last_received_timestamp}&project_ids=project-1,project-2
2. Apply missed changes to local state
3. Resume normal WebSocket message handling
```

---

## Performance Considerations

### Message Size

- Protobuf messages are compact (typically 100-500 bytes)
- Avoid sending full entity when only ID needed
- Use `FieldChange` to communicate what changed (not entire object)

### Rate Limiting

Server enforces rate limits per connection:

- **100 messages per minute** per user
- **1000 messages per minute** per tenant
- Exceeded limits trigger `RATE_LIMIT_EXCEEDED` error

### Batching (Future Enhancement)

For high-frequency updates, consider batching:

```protobuf
message BatchUpdate {
  repeated WebSocketMessage messages = 1;
}
```

---

## Security Considerations

### Authentication

- JWT token validated on connection
- Token must contain `tenant_id` and `user_id` claims
- Token expiration checked (clients must reconnect with fresh token)

### Authorization

- Users can only subscribe to resources within their tenant
- Server validates all subscriptions against tenant_id
- Users cannot receive messages from other tenants

### Data Isolation

- Each tenant has separate broadcast channel
- Messages never cross tenant boundaries
- Even with malicious subscriptions, tenant_id enforced

---

## Testing

### Mock WebSocket Client

```rust
// Connect
let mut client = MockWebSocketClient::connect("ws://localhost:3000/ws", jwt_token).await?;

// Subscribe
client.subscribe(vec!["project-1"]).await?;

// Assert received subscription confirmation
let msg = client.receive().await?;
assert!(matches!(msg.payload, Some(SubscriptionConfirmed { .. })));

// Trigger event (via REST API or another client)
create_work_item("task-1", "project-1").await?;

// Assert received event
let msg = client.receive().await?;
assert!(matches!(msg.payload, Some(WorkItemCreated { .. })));
```

### Integration Tests

Test scenarios:
1. ✅ Connect with valid JWT
2. ✅ Subscribe to project, receive updates
3. ✅ Multiple clients receive same broadcast
4. ✅ Client only receives subscribed updates
5. ✅ Reconnect after disconnect
6. ✅ Rate limiting enforced
7. ✅ Invalid JWT rejected
8. ✅ Cross-tenant isolation

---

## Future Enhancements (v1.1+)

- **Message compression**: gzip for large payloads
- **Delta updates**: Only changed fields, not full objects
- **Batch updates**: Group related changes
- **Offline queue**: Buffer messages when client offline
- **Conflict resolution**: Operational transforms for concurrent edits
