# Session 70: Activity Logging & Polish - Production Plan

## Production-Grade Score Target: 9.25/10

This session completes the application with activity feed UI, toast notifications, LLM context, and comprehensive documentation.

Production-grade features:
- Pagination for activity history
- Real-time activity broadcasts
- Toast notification queue management
- Connection quality indicators
- LLM query endpoint with seed data
- Comprehensive user and deployment documentation
- Full accessibility (ARIA, keyboard nav, screen readers)
- Error recovery UI (retry buttons, graceful degradation)

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[70.1](70.1-Session-Plan.md)** | Backend Foundation (Activity Log + LLM Context) | ~40-45k | ✅ Complete |
| **[70.2](70.2-Session-Plan.md)** | Frontend Polish (Toast + Activity UI) | ~40-45k | ⏳ Pending |
| **[70.3](70.3-Session-Plan.md)** | Integration & Documentation | ~35-40k | ⏳ Pending |

---

## Session 70.1: Backend Foundation

**Files Created (10):**

Source files (5):
- `backend/crates/pm-ws/src/handlers/activity_log.rs` - Paginated query handler
- `backend/crates/pm-core/src/models/llm_context.rs` - LLM context model
- `backend/crates/pm-db/src/repositories/llm_context_repository.rs` - Query methods
- `backend/crates/pm-db/migrations/20260127000001_seed_llm_context.sql` - 28 seed entries
- `backend/crates/pm-ws/src/handlers/llm_context.rs` - Query endpoint handler

Test files (5):
- `backend/crates/pm-db/src/repositories/tests/activity_log_repository_tests.rs`
- `backend/crates/pm-db/src/repositories/tests/llm_context_repository_tests.rs`
- `backend/crates/pm-ws/tests/activity_log_handler_tests.rs`
- `backend/crates/pm-ws/tests/llm_context_handler_tests.rs`
- (Test module declarations in existing `mod.rs` files)

**Files Modified (10):**
- `proto/messages.proto` - Activity log + LLM context messages (fields 140-145)
- `backend/crates/pm-db/src/repositories/activity_log_repository.rs` - Add pagination
- `backend/crates/pm-config/src/config.rs` - Activity log retention config
- `backend/crates/pm-core/src/models/mod.rs` - Export LlmContext
- `backend/crates/pm-db/src/repositories/mod.rs` - Export LlmContextRepository
- `backend/crates/pm-db/src/repositories/tests/mod.rs` - Declare test modules
- `backend/crates/pm-ws/src/handlers/mod.rs` - Export new handlers
- `backend/crates/pm-ws/src/handlers/dispatcher.rs` - Wire new handlers
- `backend/crates/pm-ws/src/handlers/response_builder.rs` - Add response builders
- `backend/crates/pm-ws/src/handlers/work_item.rs` - Add activity broadcast

**Verification:** `just check-backend && just test-backend`

---

## Session 70.2: Frontend Polish

**Files Created:**
- `frontend/ProjectManagement.Services/Notifications/IToastService.cs`
- `frontend/ProjectManagement.Services/Notifications/ToastService.cs`
- `frontend/ProjectManagement.Components/Shared/OperationSpinner.razor`
- `frontend/ProjectManagement.Components/Activity/ActivityFeed.razor`
- `frontend/ProjectManagement.Components/Activity/ActivityItem.razor`
- `frontend/ProjectManagement.Components/Activity/ActivityFeedSkeleton.razor`
- `frontend/ProjectManagement.Core/Models/ActivityLog.cs`
- `frontend/ProjectManagement.Core/Models/ActivityLogPage.cs`

**Files Modified:**
- `frontend/ProjectManagement.Core/Converters/ProtoConverter.cs` - Activity log converters
- `frontend/ProjectManagement.Core/Interfaces/IWebSocketClient.cs` - Activity methods
- `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs` - Activity handlers
- `frontend/ProjectManagement.Components/wwwroot/css/app.css` - Toast/activity styles

**Verification:** `just test-frontend && just build-frontend`

---

## Session 70.3: Integration & Documentation

**Files Created:**
- `docs/GETTING_STARTED.md` - Setup and first project walkthrough
- `docs/USER_GUIDE.md` - Features, keyboard shortcuts, workflows
- `docs/DEPLOYMENT_GUIDE.md` - Build, deploy, configuration reference
- `docs/API_DOCUMENTATION.md` - WebSocket protocol, error codes
- `docs/TROUBLESHOOTING.md` - Common issues and solutions
- `frontend/ProjectManagement.Components/Shared/ConnectionStatus.razor` - Connection quality UI

**Files Modified:**
- All store files (WorkItemStore, SprintStore, etc.) - Add toast notifications
- `frontend/ProjectManagement.Components/Pages/WorkItemDetail.razor` - Add activity sidebar
- `README.md` - Update with feature list and quick start
- `CLAUDE.md` - Mark Session 70 complete
- `docs/implementation-plan-v2.md` - Update status

**Verification:** `just check && just dev` (full integration test)

---

## Current State Summary

### Already Complete (from Sessions 10-60)
- Activity logging backend: `pm_activity_log` table, repository, all mutation handlers log changes
- Error handling infrastructure: `AppErrorBoundary`, `ConnectionStatus`, `OfflineBanner`, `LoadingButton`, `CircuitBreaker`, `ReconnectionService`
- LLM context schema: `pm_llm_context` table exists (empty, needs seeding)
- Complete architecture documentation and ADRs

### Needs Implementation (Session 70)
1. Activity log query handler with pagination + frontend UI
2. Toast notification service with queue management
3. LLM context seed data (28 entries) + query endpoint
4. User documentation (5 markdown files)

---

## Production-Grade Checklist (9.25/10 Target)

### Core Functionality
- [ ] Pagination for activity log (offset/limit with total_count, has_more)
- [ ] Real-time activity updates via WebSocket broadcast
- [ ] LLM query endpoint (context is accessible, not just stored)
- [ ] Toast queue management (max concurrent, auto-dismiss)

### Error Handling
- [ ] Validation errors with specific field context
- [ ] Not found errors with entity type and ID
- [ ] Access denied without leaking entity existence
- [ ] Error recovery UI (retry buttons)
- [ ] Graceful degradation (cached data when offline)

### Security
- [ ] Project-level access control on activity queries
- [ ] Correlation IDs in all responses for debugging
- [ ] No LLM context auth required (public documentation)

### Performance
- [ ] Lazy loading with "load more" pagination
- [ ] Skeleton loaders for initial load states
- [ ] Debouncing on load more button
- [ ] Activity log retention policy (configurable cleanup)
- [ ] Circuit breaker integration for database operations

### Accessibility
- [ ] ARIA roles (feed, listitem, status, progressbar)
- [ ] Keyboard navigation (focus, shortcuts)
- [ ] Screen reader announcements (aria-live)
- [ ] Focus visible indicators
- [ ] Semantic HTML (time, article, aside)

### Resilience
- [ ] Toast queue management (prevents spam)
- [ ] Error toasts bypass queue (always show)
- [ ] Component disposal (cleanup subscriptions)
- [ ] Real-time subscription cleanup on unmount

### Testing
- [ ] Integration tests for handlers
- [ ] Property-based fuzz tests for edge cases
- [ ] Component tests for UI states
- [ ] Accessibility assertions in tests

### Documentation
- [ ] Getting started guide with prerequisites
- [ ] User guide with feature walkthrough
- [ ] Deployment guide with platform specifics
- [ ] Complete API documentation
- [ ] Troubleshooting guide

---

## Files Summary

### Create (31 files)

**Backend Source (5 files):**
- `pm-ws/src/handlers/activity_log.rs`
- `pm-core/src/models/llm_context.rs`
- `pm-db/src/repositories/llm_context_repository.rs`
- `pm-db/migrations/20260127000001_seed_llm_context.sql`
- `pm-ws/src/handlers/llm_context.rs`

**Backend Tests (5 files):**
- `pm-db/src/repositories/tests/activity_log_repository_tests.rs`
- `pm-db/src/repositories/tests/llm_context_repository_tests.rs`
- `pm-ws/tests/activity_log_handler_tests.rs`
- `pm-ws/tests/llm_context_handler_tests.rs`
- Test module declarations in `mod.rs` files

**Frontend (16 files):**
- `ProjectManagement.Services/Notifications/IToastService.cs`
- `ProjectManagement.Services/Notifications/ToastService.cs`
- `ProjectManagement.Components/Shared/OperationSpinner.razor`
- `ProjectManagement.Components/Shared/ConnectionStatus.razor`
- `ProjectManagement.Components/Activity/ActivityFeed.razor`
- `ProjectManagement.Components/Activity/ActivityItem.razor`
- `ProjectManagement.Components/Activity/ActivityFeedSkeleton.razor`
- `ProjectManagement.Core/Models/ActivityLog.cs`
- `ProjectManagement.Core/Models/ActivityLogPage.cs`
- `ProjectManagement.Core/Models/GetActivityLogRequest.cs`
- (+ 6 test files)

**Documentation (5 files):**
- `docs/GETTING_STARTED.md`
- `docs/USER_GUIDE.md`
- `docs/DEPLOYMENT_GUIDE.md`
- `docs/API_DOCUMENTATION.md`
- `docs/TROUBLESHOOTING.md`

### Modify (23 files)

**Backend (10 files):**
- `proto/messages.proto`
- `pm-db/src/repositories/activity_log_repository.rs`
- `pm-config/src/config.rs`
- `pm-core/src/models/mod.rs`
- `pm-db/src/repositories/mod.rs`
- `pm-db/src/repositories/tests/mod.rs`
- `pm-ws/src/handlers/mod.rs`
- `pm-ws/src/handlers/dispatcher.rs`
- `pm-ws/src/handlers/response_builder.rs`
- `pm-ws/src/handlers/work_item.rs`

**Frontend:**
- `ProjectManagement.Core/Converters/ProtoConverter.cs`
- `ProjectManagement.Core/Interfaces/IWebSocketClient.cs`
- `ProjectManagement.Services/WebSocket/WebSocketClient.cs`
- `ProjectManagement.Services/State/*.cs` (5 stores)
- `ProjectManagement.Components/Pages/WorkItemDetail.razor`
- `ProjectManagement.Components/wwwroot/css/app.css`
- `ProjectManagement.Wasm/Program.cs`

**Project:**
- `README.md`
- `CLAUDE.md`
- `docs/implementation-plan-v2.md`

---

## Production-Grade Scoring

| Category | Score | Justification |
|----------|-------|---------------|
| Error Handling | 9.5/10 | Comprehensive errors, retry buttons, graceful degradation |
| Validation | 9.0/10 | Input validation, XSS sanitization |
| Authorization | 9.5/10 | Project-level access control, permission checks |
| Data Integrity | 9.0/10 | Pagination prevents unbounded queries, retention policy |
| Performance | 9.0/10 | Lazy loading, skeleton loaders, debouncing |
| Testing | 9.0/10 | Integration + component + property tests |
| Observability | 9.5/10 | Activity log, correlation IDs, structured logging |
| Accessibility | 9.5/10 | ARIA, keyboard nav, screen readers, semantic HTML |
| Resilience | 9.0/10 | Toast queue, component cleanup, offline fallback |
| Documentation | 9.5/10 | 5 comprehensive docs, examples, troubleshooting |

**Overall Score: 9.25/10**

### What Would Make It 9.5/10
- Metrics/telemetry export (Prometheus)
- Advanced LLM features (semantic search, embeddings)
- Automated screenshot generation for docs
- Load testing benchmarks
- Automated accessibility testing in CI

---

## Final Verification

After all three sub-sessions are complete:

```bash
# Backend
just check-backend
just test-backend
just clippy-backend

# Frontend
just check-frontend
just test-frontend

# Integration
just dev

# Manual Testing Checklist:
# 1. Create work item → verify toast + activity in sidebar
# 2. Edit work item → verify activity shows field change
# 3. Load more activity → verify pagination
# 4. Second browser tab edit → verify real-time update
# 5. Disconnect network → verify error state with retry
# 6. Press R in activity feed → verify refresh
# 7. Tab through activity → verify focus indicators
# 8. Screen reader → verify announcements

# LLM Context:
# SELECT COUNT(*) FROM pm_llm_context;  -- Should be 28
# WebSocket GetLlmContextRequest with filters

# Security:
# Request activity for unauthorized project → verify 403

# Documentation:
# Follow docs/GETTING_STARTED.md on fresh clone
```

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check` passes (all code compiles)
- [ ] Session 60 is complete (REST API, WebSocket handlers)
- [ ] `pm_activity_log` table exists with data
- [ ] `pm_llm_context` table exists (empty is OK)
