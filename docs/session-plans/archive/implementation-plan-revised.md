# Implementation Plan (Revised - WebSocket First)

This document breaks down the implementation into logical, sequential sessions that fit within 100k token budgets.

**Key Change**: WebSocket + Protobuf is our PRIMARY communication protocol, not REST. We'll build WebSocket early and use it for all operations.

---

## Architecture Decision: WebSocket-First

**Why WebSocket instead of REST?**
1. Real-time collaboration is core to the product
2. Single protocol for reads and writes (simpler)
3. Optimistic updates with server confirmation
4. Bidirectional = client can send commands, server broadcasts events
5. Production-grade from day one

**REST API Role**:
- Optional read-only endpoints for LLM integration
- Bulk data loading on initial page load
- Fallback if WebSocket unavailable

---

## Revised Session Breakdown

**Note**: Sessions numbered 10, 20, 30, etc. to leave room for incremental steps that will inevitably be needed along the way!

**Testing Strategy**: Each major implementation session (x0) is followed by a dedicated testing session (x5) to protect against context compaction and maintain detailed test documentation:
- **x0 sessions**: Implementation and core functionality
- **x5 sessions**: Comprehensive integration testing with Given/When/Then structure

This pattern ensures that if implementation sessions get compacted, the detailed testing work remains accessible in a separate conversation.

### Session 10: Foundation, Database & Protobuf ✅ **COMPLETE** (~120k tokens)

**Status**: Phases 1-6 complete (implementation)

**Goal**: Working database with migrations, repositories, and protobuf messages defined

**Deliverables**:
- ✅ Rust workspace with all crates scaffolded (pm-core, pm-db, pm-auth, pm-proto, pm-server)
- ✅ SQLx migrations for all 9 tables (8 plugin tables + users stub for FKs)
- ✅ Per-tenant connection manager with lazy-loading
- ✅ Core domain models (Rust) matching database schema exactly
- ✅ Repository pattern for all 7 entities (work_items, sprints, comments, time_entries, dependencies, activity_log, swim_lanes)
- ✅ Production-grade error handling with location tracking
- ✅ Protobuf messages matching database models (300+ lines)
- ✅ Protobuf code generation setup (Rust with prost)
- ✅ SQLx offline mode configured

**Files Created** (~50 files):
```
backend/
├── Cargo.toml (workspace with shared dependencies)
├── .cargo/config.toml (SQLX_OFFLINE=true)
├── crates/
│   ├── pm-core/
│   │   └── src/
│   │       ├── models/ (11 files: WorkItem, Sprint, Comment, TimeEntry, Dependency, ActivityLog, SwimLane + enums)
│   │       ├── errors.rs (production error handling with location tracking)
│   │       └── lib.rs
│   ├── pm-db/
│   │   ├── migrations/ (9 SQL files including users stub)
│   │   └── src/
│   │       ├── connection/ (tenant_connection_manager.rs, error.rs)
│   │       ├── repositories/ (7 repository files + mod.rs)
│   │       └── lib.rs
│   ├── pm-auth/ (stubs for JWT validation)
│   ├── pm-proto/ (build.rs, generated code)
│   └── pm-server/ (minimal main.rs)
└── proto/messages.proto (complete protobuf schema)
```

**Key Code**:
- `TenantConnectionManager` - Dynamic SQLite pool management with lazy-loading
- 7 complete repositories with SQLx compile-time checked queries
- Production error handling with `ErrorLocation` tracking
- Soft delete support (`deleted_at IS NULL` in all queries)
- Type-safe enums (WorkItemType, SprintStatus, DependencyType, LlmContextType)
- All 9 migration files (8 plugin tables + users stub for FKs)
- Core models match database columns exactly

**Key Achievements**:
- Zero TODOs in implementation code
- Zero unwrapped errors in library code
- SQLx offline mode (no DATABASE_URL needed for builds)
- Clean separation: core (domain) → db (persistence) → proto (wire)

**Success Criteria**: ✅ **ALL MET**
- ✅ `cargo build --workspace` compiles with zero warnings
- ✅ Can create tenant database and run all migrations automatically
- ✅ Repository CRUD operations implemented for all 7 entities
- ✅ Database schema matches docs/database-schema.md exactly
- ✅ Protobuf code generation working
- ✅ SQLx offline mode configured

---

### Session 15: Integration Tests for Session 10 ✅ **COMPLETE** (~120k tokens)

**Status**: Phase 7 complete - 60 integration tests passing, race condition bug found and fixed

**Goal**: Comprehensive integration testing of all repositories and connection manager

**Deliverables**:
- ✅ Test infrastructure with reusable fixtures and helpers
- ✅ 60 integration tests using Given/When/Then structure
- ✅ googletest assertions for expressive test output
- ✅ 8 test modules covering all repositories + TenantConnectionManager
- ✅ CRUD operations, soft deletes, multi-tenant isolation, foreign keys, edge cases
- ✅ **BONUS: Discovered and fixed race condition bug in TenantConnectionManager**

**Files Created** (~40 files):
```
backend/crates/pm-db/
├── tests/
│   ├── common/
│   │   ├── mod.rs
│   │   ├── test_db.rs (pool creation, user fixtures)
│   │   └── fixtures.rs (entity builders for all models)
│   ├── work_item_repository_tests.rs (7 tests)
│   ├── sprint_repository_tests.rs (7 tests)
│   ├── comment_repository_tests.rs (7 tests)
│   ├── time_entry_repository_tests.rs (8 tests)
│   ├── dependency_repository_tests.rs (8 tests)
│   ├── activity_log_repository_tests.rs (8 tests)
│   ├── swim_lane_repository_tests.rs (8 tests)
│   └── tenant_connection_manager_tests.rs (7 tests)
└── .sqlx/ (30+ query cache files)
```

**Testing Approach**:
- Given/When/Then test naming convention
- googletest assertions (assert_that!, eq(), some(), none(), is_empty())
- Reusable test fixtures for all entities
- In-memory SQLite for fast execution
- Test infrastructure with helper functions

**Bug Fixed**: Race condition in `TenantConnectionManager.get_pool()` where multiple concurrent requests for a new tenant could run migrations simultaneously. Fixed with production-grade double-checked locking pattern.

**Test Coverage**:
- WorkItemRepository: 7 tests (CRUD, hierarchy, soft deletes)
- SprintRepository: 7 tests (CRUD, status transitions, foreign keys)
- CommentRepository: 7 tests (CRUD, work item association)
- TimeEntryRepository: 8 tests (CRUD, running timers, duration calculations)
- DependencyRepository: 8 tests (blocking/blocked queries, immutability)
- ActivityLogRepository: 8 tests (audit trail, timestamp ordering, field changes)
- SwimLaneRepository: 8 tests (CRUD, position ordering, default lanes, UNIQUE constraints)
- TenantConnectionManager: 7 tests (pool creation, caching, concurrency, race conditions)

**Success Criteria**: ✅ **ALL MET**
- ✅ `cargo test --workspace` passes all 60 tests
- ✅ Tests use Given/When/Then naming convention
- ✅ Tests use googletest assertions (assert_that!, eq(), some(), none(), is_empty())
- ✅ Test fixtures reusable across all test modules
- ✅ In-memory SQLite databases for fast test execution
- ✅ All repositories tested: CRUD, soft deletes, query methods, edge cases
- ✅ Multi-tenant data isolation verified
- ✅ Foreign key constraints validated
- ✅ Concurrent access safety verified

**Next Session**: Session 20 - WebSocket Infrastructure

---

### Session 20: WebSocket Infrastructure (Est. 100k tokens)

**Goal**: Working WebSocket server with protobuf message handling

**Note**: Implementation only - testing will be in Session 25

**Deliverables**:
- ✅ JWT authentication middleware
- ✅ Tenant context extraction
- ✅ WebSocket connection handler
- ✅ Protobuf message encoding/decoding
- ✅ Per-tenant broadcast channels
- ✅ Subscription management
- ✅ Heartbeat (ping/pong)
- ✅ Basic Axum server with /ws endpoint
- ✅ Connection tests

**Files Created** (~25 files):
```
backend/
├── crates/pm-auth/ (complete JWT validation)
├── crates/pm-ws/
│   ├── connection.rs      # WebSocket connection handling
│   ├── broadcast.rs       # Per-tenant broadcast channels
│   ├── subscription.rs    # Subscription filtering
│   ├── handlers.rs        # Message handler dispatch
│   └── messages/          # Individual message handlers
└── pm-server/ (complete server with WebSocket route)
```

**Key Concepts**:
- Axum WebSocket upgrade handler
- Bidirectional communication: spawn separate tasks for incoming/outgoing messages
- Per-tenant broadcast channels using tokio::sync::broadcast
- Subscription-based filtering (clients only receive subscribed updates)
- Protobuf message encoding/decoding over binary WebSocket frames
- Heartbeat mechanism (ping/pong) to detect dead connections

**Success Criteria**:
- WebSocket server starts and accepts connections
- JWT authentication middleware validates tokens
- Tenant context extracted from JWT claims
- Protobuf message encoding/decoding works
- Per-tenant broadcast channels created
- Subscription management implemented
- Heartbeat (ping/pong) implemented
- Code compiles and basic smoke tests pass

---

### Session 25: Integration Tests for Session 20 (Est. 60k tokens)

**Goal**: Comprehensive WebSocket integration testing

**Note**: Testing session for Session 20 implementation

**Deliverables**:
- Integration tests for WebSocket connection lifecycle
- JWT authentication and authorization tests
- Subscription management tests
- Broadcast channel tests with multi-client scenarios
- Protobuf serialization/deserialization tests
- Heartbeat and connection timeout tests
- Multi-tenant isolation tests

**Test Coverage**:
- WebSocket connection with valid/invalid JWT
- Tenant context extraction
- Subscribe/unsubscribe operations
- Broadcast to subscribed clients only
- Subscription filtering (clients only receive relevant updates)
- Concurrent connections from same tenant
- Heartbeat keeps connection alive
- Graceful disconnection and cleanup

**Success Criteria**:
- WebSocket connection tests pass
- JWT validation tested (valid tokens accepted, invalid rejected)
- Subscription filtering verified (no cross-tenant leakage)
- Broadcast delivery tested with multiple clients
- Protobuf round-trip serialization verified
- Connection lifecycle tested (connect, subscribe, disconnect)

---

### Session 30: Work Items via WebSocket (Est. 100k tokens)

**Goal**: Complete CRUD for work items using WebSocket commands

**Deliverables**:

**Backend**:
- ✅ Work item message handlers (Create, Update, Delete, Move)
- ✅ Business logic services (validation, hierarchy checks)
- ✅ Broadcast work item events to subscribed clients
- ✅ Error responses via WebSocket

**Frontend**:
- ✅ Blazor project structure (.sln, 4 projects)
- ✅ Core models (C#)
- ✅ Protobuf C# code generation
- ✅ WebSocket client
- ✅ State management
- ✅ Radzen setup
- ✅ Project dashboard page
- ✅ Work item list component
- ✅ Create work item dialog
- ✅ Work item detail view

**Files Created** (~25 backend files + ~20 frontend files):
```
backend/crates/pm-ws/
└── handlers/
    └── work_items.rs (Create, Update, Delete, Move handlers)

frontend/
├── ProjectManagement.sln
├── ProjectManagement.Core/ (models, interfaces)
├── ProjectManagement.Services/ (WebSocket client, state management)
├── ProjectManagement.Components/ (Razor Class Library - UI components)
└── ProjectManagement.Wasm/ (standalone WASM host)
```

**Backend Implementation Pattern**:
- Message handlers validate, call repository, broadcast event
- Pattern: Validate → Get Pool → Use Repository → Broadcast
- All handlers return Result for error handling
- Change tracking for updates (old value → new value)

**Frontend Implementation Pattern**:
- WebSocket client with bidirectional channels (send/receive loops)
- State manager subscribes to updates and applies them locally
- Optimistic UI updates with server confirmation rollback
- Radzen components for professional UI out of the box

**Success Criteria**:
- Backend work item handlers implemented (Create, Update, Delete, Move)
- Frontend work item components built (list, detail, create dialog)
- WebSocket commands send/receive work item operations
- Code compiles and basic smoke tests pass

---

### Session 35: Integration Tests for Session 30 (Est. 60k tokens)

**Goal**: End-to-end testing of work item CRUD via WebSocket

**Note**: Testing session for Session 30 implementation

**Deliverables**:
- Backend integration tests for work item message handlers
- Frontend component tests for work item UI
- End-to-end tests for work item CRUD operations
- Real-time broadcast verification tests
- Hierarchy and relationship tests (parent/child work items)

**Test Coverage**:
- Create work item via WebSocket command
- Update work item fields (title, description, status, assignee)
- Delete work item (soft delete)
- Move work item (change parent, reorder)
- Work item hierarchy queries
- Real-time broadcast to subscribed clients
- Optimistic UI updates with server confirmation
- Error handling and validation

**Success Criteria**:
- Work item CRUD operations tested via WebSocket
- Real-time updates verified across multiple clients
- Frontend UI components tested with bUnit
- Work item hierarchy operations tested
- Validation errors handled correctly

---

### Session 40: Sprints & Comments via WebSocket (Est. 100k tokens)

**Goal**: Sprint management and commenting using WebSocket

**Backend**:
- ✅ Sprint message handlers (Create, Update, Delete, Start, Complete)
- ✅ Comment message handlers (Add, Update, Delete)
- ✅ Sprint assignment validation
- ✅ Broadcast sprint and comment events

**Frontend**:
- ✅ Sprint list & management
- ✅ Sprint board (Kanban)
- ✅ Sprint planning
- ✅ Comment component
- ✅ Real-time comment updates

**Success Criteria**:
- Backend sprint and comment handlers implemented
- Frontend sprint and comment UI components built
- WebSocket commands working for sprints and comments
- Code compiles and basic smoke tests pass

---

### Session 45: Integration Tests for Session 40 (Est. 60k tokens)

**Goal**: Testing sprint management and commenting features

**Note**: Testing session for Session 40 implementation

**Deliverables**:
- Integration tests for sprint CRUD operations via WebSocket
- Integration tests for comment operations via WebSocket
- Sprint assignment and status transition tests
- Real-time sprint and comment update tests
- Frontend component tests for sprint board and comments

**Test Coverage**:
- Create, update, delete sprints
- Sprint status transitions (Planned → Active → Completed)
- Assign/unassign work items to sprints
- Add, edit, delete comments
- Comment threading and replies
- Real-time broadcast of sprint changes
- Real-time broadcast of comment updates
- Sprint velocity calculations

**Success Criteria**:
- Sprint CRUD tested via WebSocket
- Comment operations tested
- Sprint board real-time updates verified
- Comment real-time updates verified
- Sprint assignment logic tested

---

### Session 50: Time Tracking & Dependencies via WebSocket (Est. 100k tokens)

**Goal**: Time tracking with timers and dependency management

**Backend**:
- ✅ Time entry handlers (Start, Stop, Create, Update, Delete)
- ✅ Running timer logic
- ✅ Dependency handlers (Create, Delete)
- ✅ Circular dependency detection
- ✅ Broadcast time and dependency events

**Frontend**:
- ✅ Timer component (start/stop)
- ✅ Time entry list
- ✅ Manual time entry
- ✅ Dependency management UI
- ✅ Blocked task indicators

**Success Criteria**:
- Backend time tracking and dependency handlers implemented
- Frontend timer and dependency UI components built
- WebSocket commands working for time entries and dependencies
- Code compiles and basic smoke tests pass

---

### Session 55: Integration Tests for Session 50 (Est. 60k tokens)

**Goal**: Testing time tracking and dependency management

**Note**: Testing session for Session 50 implementation

**Deliverables**:
- Integration tests for time entry operations via WebSocket
- Integration tests for dependency operations via WebSocket
- Running timer tests (start/stop/auto-stop logic)
- Circular dependency detection tests
- Real-time timer sync tests

**Test Coverage**:
- Start/stop time tracking timers
- Create manual time entries
- Update/delete time entries
- Running timer logic (only one per user)
- Timer duration calculations
- Create/delete dependencies
- Circular dependency prevention
- Blocked task queries
- Real-time broadcast of timer changes
- Real-time broadcast of dependency changes

**Success Criteria**:
- Time tracking operations tested via WebSocket
- Running timer logic verified
- Dependency CRUD tested
- Circular dependency detection verified
- Real-time timer updates verified across clients

---

### Session 60: REST API for LLMs & Bulk Loading (Est. 80k tokens)

**Goal**: Read-only REST endpoints for LLM integration and efficient bulk loading

**Why REST Now?**
- WebSocket is great for real-time updates, not for bulk queries
- LLMs need simple HTTP endpoints
- Initial page load should fetch all data efficiently

**Backend**:
- ✅ Read-only REST endpoints
  - `GET /api/v1/projects/:id/context` - Full project data dump
  - `GET /api/v1/work-items/:id` - Single item detail
  - `GET /api/v1/activity` - Activity log queries
  - `GET /api/v1/llm/context` - Schema documentation
- ✅ Optimized bulk queries
- ✅ CORS configuration

**Frontend**:
- ✅ HTTP client for initial data load
- ✅ Use REST for page load, WebSocket for updates

**Success Criteria**:
- REST API endpoints implemented for read-only queries
- Bulk data loading optimized
- CORS configured for external access
- Code compiles and basic smoke tests pass

---

### Session 65: Integration Tests for Session 60 (Est. 40k tokens)

**Goal**: Testing REST API endpoints

**Note**: Testing session for Session 60 implementation

**Deliverables**:
- Integration tests for REST API endpoints
- Bulk query performance tests
- LLM context endpoint tests
- CORS configuration tests
- Frontend HTTP client tests

**Test Coverage**:
- GET /api/v1/projects/:id/context - full project data dump
- GET /api/v1/work-items/:id - single item detail
- GET /api/v1/activity - activity log queries
- GET /api/v1/llm/context - schema documentation
- Query performance (bulk loading vs individual requests)
- CORS headers verification
- Authentication (optional for read-only)

**Success Criteria**:
- All REST endpoints tested
- Bulk loading performance verified
- LLM context endpoint returns valid schema documentation
- CORS tested with different origins
- Frontend uses REST for initial load, WebSocket for updates

---

### Session 70: Activity Logging, Polish & Documentation (Est. 80k tokens)

**Goal**: Complete production system

**Backend**:
- ✅ Activity log on all mutations
- ✅ LLM context seed data
- ✅ Swim lanes seed data
- ✅ Error handling improvements

**Frontend**:
- ✅ Activity history view
- ✅ User presence indicators
- ✅ Connection status
- ✅ Loading states
- ✅ Error boundaries
- ✅ Toast notifications

**Documentation**:
- ✅ README with setup
- ✅ API documentation
- ✅ Deployment guide

**Success Criteria**:
- Activity logging implemented on all mutations
- Seed data scripts created
- Error handling polished
- Documentation complete
- System production-ready

---

### Session 75: Final Integration Tests & E2E Testing (Est. 60k tokens)

**Goal**: End-to-end testing and production readiness verification

**Note**: Final testing session for Session 70 and overall system

**Deliverables**:
- Activity log integration tests
- End-to-end workflow tests (create project → add tasks → run sprint → complete)
- Performance tests under load
- Production deployment tests
- Documentation verification

**Test Coverage**:
- Activity log captures all mutations correctly
- Activity history view displays correctly
- User presence indicators work
- Connection status indicators accurate
- Error boundaries catch and display errors
- Toast notifications appear for user actions
- Full workflow: Project → Epics → Stories → Tasks → Sprint → Complete
- Multi-user collaboration scenarios
- Load testing (multiple concurrent users)
- Database migration tests on real SQLite files

**Success Criteria**:
- All activity log tests pass
- E2E workflows tested and verified
- Performance acceptable under load
- No critical bugs
- Documentation accurate and complete
- System ready for production deployment

---

## Revised Token Budget

| Session | Type | Tokens | Status | Focus |
|---------|------|--------|--------|-------|
| 10 | Impl | ~120k | ✅ Complete | Database + Protobuf (Phases 1-6) |
| 15 | Test | ~120k | ✅ Complete | Integration Tests for Session 10 |
| 20 | Impl | ~100k (est) | ⏸️ Pending | WebSocket infrastructure |
| 25 | Test | ~60k (est) | ⏸️ Pending | Integration Tests for Session 20 |
| 30 | Impl | ~100k (est) | ⏸️ Pending | Work items (backend + frontend) |
| 35 | Test | ~60k (est) | ⏸️ Pending | Integration Tests for Session 30 |
| 40 | Impl | ~100k (est) | ⏸️ Pending | Sprints + comments |
| 45 | Test | ~60k (est) | ⏸️ Pending | Integration Tests for Session 40 |
| 50 | Impl | ~100k (est) | ⏸️ Pending | Time tracking + dependencies |
| 55 | Test | ~60k (est) | ⏸️ Pending | Integration Tests for Session 50 |
| 60 | Impl | ~80k (est) | ⏸️ Pending | REST for LLMs + bulk load |
| 65 | Test | ~40k (est) | ⏸️ Pending | Integration Tests for Session 60 |
| 70 | Impl | ~80k (est) | ⏸️ Pending | Activity log + polish + docs |
| 75 | Test | ~60k (est) | ⏸️ Pending | Final E2E Tests |
| **Total** | | **~1240k** | 2/14 done | 12 sessions remaining |

**Pattern**: Each x0 session (implementation) is followed by x5 session (testing) to protect against context compaction and maintain detailed test documentation.

---

## Key Advantages of WebSocket-First

1. **Real-time from day one** - No need to retrofit
2. **Simpler architecture** - One protocol for everything
3. **Better UX** - Optimistic updates with confirmation
4. **Production-ready** - Built for collaboration
5. **REST as enhancement** - Add later for specific needs

---

## Implementation Progress

**Session 10**: ✅ Complete (2026-01-10, ~120k tokens)
- Phases 1-6 delivered: workspace, migrations, models, connection manager, repositories, protobuf
- Production-grade code with zero TODOs and proper error handling
- SQLx offline mode configured

**Session 15**: ✅ Complete (2026-01-10, ~120k tokens)
- Phase 7: 60 integration tests passing with Given/When/Then structure
- Test infrastructure with reusable fixtures and helpers
- googletest assertions for expressive test output
- **Bonus**: Race condition bug discovered and fixed in TenantConnectionManager

**Next Session**: Session 20 - WebSocket Infrastructure (Implementation)
- Ready to begin when needed
- Will build on the solid foundation from Sessions 10 & 15
- Session 25 will follow for WebSocket integration testing
