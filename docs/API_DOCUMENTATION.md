# API Documentation

This system is **WebSocket‑first**. There is no production REST API in the current implementation. All real‑time operations use WebSocket + Protocol Buffers.

---

## WebSocket Protocol

See:
- `docs/websocket-protocol.md`

Key concepts:
- Client connects with JWT (or desktop mode auth disabled)
- Client **must subscribe** to receive updates
- All CRUD operations are protobuf messages
- Server broadcasts events to all subscribed clients

---

## Message Schema

The protobuf schema lives in:
- `proto/messages.proto`

Generated code:
- Rust: `backend/crates/pm-proto`
- C#: `frontend/ProjectManagement.Core/Proto`

---

## REST API (Planned)

Read‑only REST endpoints for LLM integration are planned for **Session 60**.
