# ADR-0005: WebSocket Communication with Protobuf

## Status
Accepted

> **Updated (2026-01-19)**: Simplified for single-tenant desktop deployment (see [ADR-0006](0006-single-tenant-desktop-first.md)). No multi-tenant broadcast channels needed. Authentication is optional in desktop mode.

## Context
The project management application requires real-time updates for collaborative features. Multiple users may view and edit the same projects, tasks, and sprints simultaneously. We need to choose a real-time communication protocol and serialization format.

Options considered:

**Communication protocols:**
1. REST with polling
2. Server-Sent Events (SSE)
3. WebSocket

**Serialization formats:**
1. JSON
2. Protocol Buffers (protobuf)
3. MessagePack
4. FlatBuffers

## Decision
We will use WebSocket for bidirectional real-time communication with Protocol Buffers for message serialization.

Technology stack:
- **Axum WebSocket**: Native WebSocket support in Axum
- **Prost**: Rust protobuf implementation with codegen
- **Google.Protobuf**: C# protobuf library for Blazor frontend

## Consequences

### Positive

#### WebSocket Benefits
- **Bidirectional**: Full duplex communication for instant client/server interaction
- **Low latency**: Persistent connection eliminates HTTP handshake overhead
- **Server push**: Server can push updates to clients without polling
- **Production-grade**: Standard protocol with mature tooling and monitoring
- **Collaborative features**: Enables presence indicators, live editing, typing indicators
- **Optimistic updates**: Client can send update and receive confirmation/rejection
- **Connection as session**: Authenticated WebSocket connection represents active user session

#### Protobuf Benefits
- **Efficient**: Smaller message sizes than JSON (typically 3-10x smaller)
- **Strongly typed**: Generated code provides type safety on both client and server
- **Schema evolution**: Field numbering enables backward/forward compatibility
- **Performance**: Faster serialization/deserialization than JSON
- **Cross-language**: Same `.proto` files generate code for Rust and C#
- **Versioning**: Can evolve message formats without breaking clients

### Negative

#### WebSocket Challenges
- **Connection management**: Must handle reconnection, heartbeats, and cleanup
- **Scaling complexity**: Requires sticky sessions or distributed message bus
- **Debugging**: Harder to inspect than REST requests
- **Infrastructure**: Need to handle WebSocket proxying in load balancers

#### Protobuf Challenges
- **Not human-readable**: Can't easily inspect messages in browser dev tools
- **Build complexity**: Requires code generation step in build process
- **Schema management**: `.proto` files must be kept in sync between client and server
- **Debugging**: Need tools to decode binary messages

### Implementation Strategy

**WebSocket Lifecycle:**
```
1. Client connects to /ws
2. Server validates JWT (if auth enabled) or accepts connection (desktop mode)
3. Client subscribes to specific projects/sprints
4. Server adds client to broadcast channel
5. Bidirectional message exchange
6. Heartbeat/ping-pong for connection health
7. Graceful disconnect and cleanup
```

**Message Structure:**
```protobuf
message WebSocketMessage {
  string message_id = 1;  // For request/response correlation
  oneof payload {
    TaskCreated task_created = 2;
    TaskUpdated task_updated = 3;
    TaskDeleted task_deleted = 4;
    CommentAdded comment_added = 5;
    SprintUpdated sprint_updated = 6;
    UserPresence user_presence = 7;
    // ... extensible
  }
}
```

**Broadcasting Strategy:**
- Single broadcast channel using `tokio::sync::broadcast` (single-tenant)
- Clients subscribe to projects/sprints they're viewing
- Server publishes updates to subscribed clients
- Automatic fan-out to all connected clients viewing relevant entities

**Fallback Strategy:**
- If WebSocket fails, client can fall back to REST + polling
- Core functionality remains available without WebSocket
- Graceful degradation for older browsers or network restrictions

### Monitoring Considerations
- Track active WebSocket connections per tenant
- Monitor message throughput and latency
- Alert on connection drops or failed authentication
- Log reconnection patterns for reliability analysis
