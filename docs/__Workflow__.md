# Efficient Prompt Workflow

### Init
```txt
Read `docs/implementation-plan-v2.md.` We just finished impl'ing session 40
```

### Plan
```txt
Create a step-by-step plan for how to implement Session 50.
```

### Challenge
```txt
Evaluate your plan and rate it based on how production grade it is.
```

### Product-Grade
```text
My quality bar is to be at least 9.25 out of 10 on a production grade score.

Iterating on your plan until you meet the target level.
```

### ordering
```text
Verify that the `50-Session-Plan.md` has proper dependency ordering so that when implementing sequentially, nothing references code that hasn't been written yet.
```

```text
please stop thinking in terms of sub-sessions and instead in dependency order
```

```text
Can you show a dependency graph?
```

### x
```text
before continuing, can I get you to perform a fresh-eyes review of `50-Session-Plan.md`. Identify strengths/weaknesses/issues and re-grade x.x/10 for production-grade
```

### Sub-Process
```text
read `docs/session-plans/42-Session-Plan.md` and break it down into multiple sub-sessions targeting <50k tokens each that can be presented to a future user as       
  teaching material. Take inspiration from the plan/sub-plans in `docs/session-plans/template`
```

### Summarize
```text
Restructure this session plan as teaching material:

  1. Add a "Teaching Focus" section (3-4 bullet points of concepts learned)
  2. Frame the work with "The Problem" and "The Solution"
  3. Add "Why X?" explanations after major code blocks
  4. Include a "Prerequisites Check" with verification commands
  5. End with "Key Concepts Learned" summary
  6. Add "Next Session" link if applicable

Follow the structure in docs/session-plans/42.1-Session-Plan.md as a template.
```

### Other
```text
Read `CRITICAL_OPERATING_CONSTRAINTS.md`, `docs/session-plans/42.5-Session-Plan.md`, `docs/session-plans/42-Session-Plan-Overview.md` and `docs/implementation-plan-v2.md`.

Please audit `docs/session-plans/42.5-Session-Plan.md` and cross reference it's contents against the current state of the project to avoid making poor assumptions.

I want you to teach me by presenting me with bite-sized chunks for me to write/edit keeping the commentary separate from the code snippets to make the code snippets easier for me to follow
```

### End of Session Sanity Check
```text
Builds clean and all tests pass. Please sanity check that everything in `docs/session-plans/42.5-Session-Plan.md` was implemented as expected.
```

### Update Docs
```text
Please update `docs/session-plans/42.5-Session-Plan.md` and `docs/session-plans/42-Session-Plan-Overview.md`
```

### Commit
```text
I have staged all files, please commit without a byline.
```

---

### Init
```txt
Read `CRITICAL_OPERATING_CONSTRAINTS.md`, `docs/session-plans/43-Session-Plan.md`, `docs/session-plans/42-Session-Plan-Overview.md` and `docs/implementation-plan-v2.md`.

Please audit `docs/session-plans/43-Session-Plan.md` and cross reference it's contents against the current state of the project to avoid making poor assumptions.

I want you to teach me by presenting me with bite-sized chunks for me to write/edit keeping the commentary separate from the code snippets to make the code snippets easier for me to follow
```

```text
Verify that the `41-Session-Plan.md` has proper dependency ordering so that when implementing sequentially, nothing references code that hasn't been written yet.
```

```text
read `docs/session-plans/41-Session-Plan.md` and break it down into multiple sub-sessions targeting <50k tokens each that can be presented to a future user as       
  teaching material. Take inspiration from the plan/sub-plans in `docs/session-plans/template`
```