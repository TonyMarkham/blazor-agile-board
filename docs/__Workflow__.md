# Efficient Prompt Workflow

### Init
```txt
Read `docs/implementation-plan-v2.md.` We just finished impl'ing session 10
```

### Plan
```txt
Create a step-by-step plan for how to implement Session 20.
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
Verify that the `session-20-plan.md` has proper dependency ordering so that when implementing sequentially, nothing references code that hasn't been written yet.
```

### x
```text
before continuing, can I get you to perform a fresh-eyes review of the session 20 plan. Identify strengths/weaknesses/issues and re-grade x.x/10 for production-grade
```

### Other
```text
Read `CRITICAL_OPERATING_CONSTRAINTS.md`, `docs/session-plans/session-20.5-plan.md`, `docs/session-plans/session-20-plan.md` and `docs/implementation-plan-v2.md`

I want you to teach me by presenting me with bite-sized chunks for me to write/edit keeping the commentary separate from the code snippets to make the code snippets easier for me to follow
```

---

## Scrum Master Agent Session (Research)

### Init
```txt
We just finished implementing session 05
```

### Followup
```txt
Looks like we have finished Phase 2, if you agree, can you please update `SESSION_PLAN_FEATURE_BASED_FINAL.md` to reflect this
```

### Plan
```txt
Please look at planning how to implement Session 10 from `docs/implementation-plan-v2.md` to give the developer agent a solid foundation. Recognize, your role is to plan, not to code.
```

---

## Developer-Teacher Agent Session (Plan)

### Init
```txt
read NEXT_SESSION_PROMPT.md
```

### Followup
```txt
I think there was some required reading that you avoided
```

### Initial Plan
```txt
before you begin teaching me, I want you to fully think out and plan the material ahead of time and present this plan
`````

### Challenge
```txt
Now that it seems like you've researched what you want to do, I'd like you to evaluate your plan and rate it based on how production grade it is.
```

### Product-Grade
```text
I want you to plan to give me something that's over 9 out of 10 on a production grade score and don't stop iterating until you do.
```

### Document
```txt
Can you please serialize that to `session_20_prod_plan.md`?
```

---

## Developer-Teacher Agent Session (Implementation)

### Implement
```txt
I'd like you to read `CRITICAL_OPERATING_CONSTRAINTS.md`  And using those guidelines help me implement `SESSION-20-PLAN.md`

 You won't be writing or editing any code. You'll be presenting code snippets to me, for me to implement, so that I understand what's going on incrementally.

 Please break the presentation to me up into digestible slices that won't over-tax my cognitive awareness.
```

### Other
```text
The doc should be a fairly comprehensive impl plan, I want you to teach me by presenting me with bite-sized chunks for      
  me to write/edit keeping the commentary seperate from the code snippets to make the code snippets easier for me to          
  follow  
```