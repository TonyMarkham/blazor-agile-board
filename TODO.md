# Future Enhancements & Technical Debt

This file tracks improvements that are nice-to-have but not required for MVP.

## Desktop UX Enhancements

### Blazor Startup Progress UI (Low Priority)
**Context**: Session 40.4 implemented minimal JS startup (30 lines). The original plan had an elaborate 567-line JS startup experience with progress indicators.

**Proposed**: Replace the simple "Loading..." spinner with a Blazor component that shows:
- Animated progress bar with 4 steps (Initialize → Start Server → Check Health → Load UI)
- Error screen with retry button (all in C#)
- Reconnection overlay when server restarts
- Export diagnostics button

**Benefits**:
- Better user feedback during 30-second startup
- Easier debugging (users can see which step failed)
- More polished desktop app experience

**Implementation**:
- Create `Components/Desktop/StartupScreen.razor`
- C# state machine for progress tracking
- CSS animations
- Zero additional JavaScript (stays true to minimal-JS philosophy)

**Estimated Effort**: ~4 hours
**Priority**: Low (current simple loading works fine)
**Session**: Post-MVP

---

## Known Issues (Safe to Ignore)

### Source Map Warning in Development
**Warning**: `Source Map "http://127.0.0.1:1430/_framework/dotnet.runtime.js.map" has SyntaxError: JSON Parse error: Unrecognized token '<'`
**Cause**: Blazor debug builds + Tauri dev server source map handling
**Impact**: None (cosmetic browser console warning only)
**Fix**: Not required - goes away in Release builds
**Priority**: Ignore

---

## Session 40.5 (Next Up)

- [ ] Build scripts (dev.sh, build.sh)
- [ ] CI/CD pipeline (GitHub Actions)
- [ ] Unit tests for desktop integration
- [ ] Integration tests (end-to-end)
- [ ] Manual test checklist

---

## Future Sessions (Post-MVP)

### Session 50: Sprints & Comments
- Sprint planning UI
- Comment threads
- Real-time collaboration

### Session 60: Time Tracking & Dependencies
- Running timers
- Dependency management
- Circular dependency detection

### Session 70: Activity Logging & Polish
- Activity feed
- Error handling polish
- Loading states
- Documentation

### Session 80+: Advanced Features
- REST API for LLM integration
- Offline support with sync
- Multi-tenant SaaS deployment
- Advanced reporting & analytics
- Import/export (JIRA, CSV)
