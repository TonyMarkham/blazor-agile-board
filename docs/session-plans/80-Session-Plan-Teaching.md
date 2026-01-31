# Session 80: Dependency Management UI (Teaching Sub-Sessions)

## Goal
Deliver a production-grade dependency management UI in small, teachable sessions (<50k tokens each), with explicit API verification, accessibility-first interaction patterns, and testable increments.

---

## Sub-Session Breakdown

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[80.1](80.1-Session-Plan.md)** | Contracts + CSS Integration | ~20-30k | ⏳ Pending |
| **[80.2](80.2-Session-Plan.md)** | DependencyRow Component + Tests | ~35-45k | ⏳ Pending |
| **[80.3](80.3-Session-Plan.md)** | AddDependencyDialog + A11y + Tests | ~40-50k | ⏳ Pending |
| **[80.4](80.4-Session-Plan.md)** | DependencyManager + Page Wiring + Indicator + Tests | ~40-50k | ⏳ Pending |

---

## Pre-Implementation Checklist

Before starting any sub-session:

- [ ] `just restore-frontend`
- [ ] `just build-cs-components`

---

## Final Verification (After 80.4)

```bash
just build-cs-components
just test-cs-components
```

Manual verification checklist lives in `docs/session-plans/80-Session-Plan-Initial.md`.
