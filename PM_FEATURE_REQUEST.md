# PM-CLI Feature Requests

Based on experience importing a 47-task implementation plan into the PM system.

## High Priority

### 1. Update parent_id After Creation

**Current Issue**: Cannot change a work item's parent after creation. Must delete and recreate to move items between epics/stories.

**Proposed Solution**:
```bash
pm work-item update <id> --version <v> --parent-id <new-parent-id> --pretty
```

**Use Case**: Move stories between epics, or tasks between stories without losing history, comments, or time entries.

---

### 2. Dedicated Move Command

**Current Issue**: No explicit command for moving items in the hierarchy.

**Proposed Solution**:
```bash
pm work-item move <id> --to-parent <parent-id> --version <v> --pretty
```

**Use Case**: Clearer semantics for reorganizing project structure. Could handle version conflicts and validation internally.

---

### 3. Query Orphaned Items

**Current Issue**: No way to find work items with `parent_id = NULL` using CLI. Must query database directly.

**Proposed Solution**:
```bash
pm work-item list <project-id> --orphaned --pretty
pm work-item list <project-id> --orphaned --type story --pretty
```

**Use Case**:
- Quality assurance - find items that accidentally lost their parent
- Cleanup operations before sprints
- Validation during import operations

---

## Medium Priority

### 4. Bulk Import from Structured File

**Current Issue**: Must create each work item individually. Importing a 47-task plan required ~100+ CLI commands.

**Proposed Solution**:
```bash
pm work-item import --file plan.json --project-id <id> --pretty
pm work-item import --file plan.yaml --project-id <id> --dry-run --pretty
```

**File Format Example** (JSON):
```json
{
  "epics": [
    {
      "title": "Core Library",
      "description": "...",
      "priority": "high",
      "stories": [
        {
          "title": "Foundation",
          "description": "...",
          "tasks": [
            {
              "title": "PONE-29: Update Cargo.toml",
              "description": "...",
              "priority": "critical"
            }
          ]
        }
      ]
    }
  ]
}
```

**Use Case**:
- Import implementation plans from plan mode
- Migrate projects between instances
- Batch creation during project setup

---

### 5. Get Item with Descendants

**Current Issue**: Must make multiple API calls to get an epic with all its stories and tasks.

**Proposed Solution**:
```bash
pm work-item get <id> --with-children --pretty
pm work-item get <id> --with-descendants --pretty  # Include grandchildren
```

**Output**: Nested JSON showing full hierarchy tree.

**Use Case**:
- View entire epic structure at once
- Export sections of project
- Generate reports

---

### 6. Project Validation Command

**Current Issue**: No way to check project health. Found orphaned items only by manual investigation.

**Proposed Solution**:
```bash
pm project validate <project-id> --pretty
```

**Checks**:
- Orphaned work items (null parent_id)
- Circular dependencies
- Invalid parent references (parent doesn't exist)
- Version conflicts
- Work items in deleted sprints
- Invalid status transitions

**Output**:
```json
{
  "valid": false,
  "errors": [
    {
      "type": "orphaned_item",
      "item_id": "...",
      "display_key": "PONE-23",
      "title": "Configuration Management"
    }
  ],
  "warnings": [
    {
      "type": "no_sprint",
      "count": 12,
      "message": "12 work items not assigned to any sprint"
    }
  ]
}
```

**Use Case**:
- Pre-sprint health checks
- Post-import validation
- Continuous quality monitoring

---

## Nice to Have

### 7. Built-in Output Formatting

**Current Issue**: JSON output requires external tools (jq, python) for readable viewing. Piping often fails on Windows.

**Proposed Solution**:
```bash
pm work-item list <project-id> --format table --pretty
pm work-item list <project-id> --format tree --pretty
pm work-item list <project-id> --format csv --pretty
```

**Table Format Example**:
```
╔═══════════╤════════════════════════════╤══════════╤══════════╗
║ Key       │ Title                      │ Type     │ Status   ║
╠═══════════╪════════════════════════════╪══════════╪══════════╣
║ PONE-1    │ Core STT Library           │ epic     │ backlog  ║
║ PONE-2    │ Foundation - Workspace     │ story    │ backlog  ║
║ PONE-29   │ Update root Cargo.toml     │ task     │ backlog  ║
╚═══════════╧════════════════════════════╧══════════╧══════════╝
```

**Tree Format Example**:
```
PONE-1: Core STT Library [epic]
├── PONE-2: Foundation - Workspace Setup [story]
│   ├── PONE-29: Update root Cargo.toml [task]
│   ├── PONE-70: Create install.ps1 [task]
│   └── PONE-71: Create install.sh [task]
└── PONE-6: Crate Structure [story]
    ├── PONE-30: Create auto-scribe-core [task]
    └── PONE-31: Create auto-scribe binary [task]
```

**Use Case**:
- Quick terminal visualization
- Human-readable output without post-processing
- Better cross-platform compatibility

---

### 8. Support for Multi-line Descriptions with Code Blocks

**Current Issue**: Passing descriptions with markdown code blocks (containing backticks) to `--description` parameter causes shell escaping issues. Bash interprets backticks as command substitution, breaking the CLI workflow.

**Problem Example**:
```bash
# This fails because bash tries to execute `rust` as a command
pm work-item update <id> --version 1 --description "
Implement Config struct.

```rust
pub struct Config { ... }
```
" --pretty
```

**Current Workarounds** (all suboptimal):
1. Write description to temp file, then use `--description "$(cat temp.txt)"` (adds extra steps, leaves temp files)
2. Escape all backticks with backslashes (extremely error-prone, hard to maintain)
3. Use language-specific subprocess libraries (defeats purpose of CLI, not portable)

**Proposed Solutions**:

**Option A: Read from File**
```bash
pm work-item update <id> --version 1 --description-file description.md --pretty
```

**Option B: Read from stdin**
```bash
cat description.md | pm work-item update <id> --version 1 --description-stdin --pretty
# Or with heredoc:
pm work-item update <id> --version 1 --description-stdin --pretty <<'EOF'
Implement Config struct.

```rust
pub struct Config { ... }
```
EOF
```

**Option C: Auto-detect heredoc/file**
```bash
# If description starts with @, read from file
pm work-item update <id> --version 1 --description @description.md --pretty
```

**Use Case**:
- Importing implementation plans with production-grade code snippets
- Automated task creation from markdown documentation
- Programmatic updates without fighting shell escaping
- Maintaining readable code examples in task descriptions

**Impact**: Currently, creating 47 tasks with code snippets required either:
- Manual workarounds with temp files (fragile, leaves artifacts)
- Falling back to language-specific tooling (defeats CLI purpose)
- Significant time spent debugging shell escaping issues

---

## Summary

**Top 3 Most Impactful**:
1. **Update parent_id** - Critical for maintaining data integrity during reorganization
2. **Query orphaned items** - Essential for quality assurance and validation
3. **Bulk import** - Massive time saver for project setup and plan imports

**Implementation Priority**: High Priority items would eliminate the need for direct database access and significantly improve CLI usability for complex project management workflows.

---

## Context

These requests emerged from importing a 71-item implementation plan (5 epics, 18 stories, 47 tasks) into the PM system. The process revealed several workflow gaps:
- 5 orphaned stories discovered mid-import (no CLI way to detect them)
- Unable to move stories between epics using CLI (required SQL access)
- 100+ individual CLI commands to create hierarchy (bulk import would reduce to 1 command)
- Multiple duplicate tasks created due to error recovery limitations

All requests aim to make the CLI more robust for large-scale project management operations while maintaining the excellent optimistic locking and real-time sync features already present.
