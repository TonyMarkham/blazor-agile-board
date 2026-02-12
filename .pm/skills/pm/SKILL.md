# pm-cli - Blazor Agile Board CLI

Command-line interface for programmatic interaction with the Blazor Agile Board project management system. All operations sync in real-time with the Blazor UI through WebSocket broadcasts.

## Quick Reference

**Binary location:** `.pm/bin/pm`

**How to invoke:**
```bash
.pm/bin/pm <command> [--pretty]
```

**Global options:**
- `--server <URL>` - Server URL (default: auto-discovered from `.pm/server.json`)
- `--user-id <UUID>` - User ID for operations (default: LLM user from config)
- `--pretty` - Pretty-print JSON output (always recommended for readable output)

## When to Use This Skill

Use the pm-cli when you need to:

1. **Manage projects** - Create, read, update, delete, and list projects
2. **Manage work items** - Full CRUD on tasks, stories, and epics (create, update, delete, list, get)
3. **Manage sprints** - Full CRUD on sprint planning cycles
4. **Manage comments** - Create, update, delete, and list comments on work items
5. **Track time** - Start/stop timers and manage time entries on work items
6. **Manage dependencies** - Create and delete dependency links between work items
7. **Query data** - Read-only access to swim lanes, or filtered queries on work items
8. **Bulk operations** - Export/import entire project data as JSON
9. **Launch desktop app** - Start the Tauri desktop application

**DO NOT use this skill for:**
- Building or compiling code
- Modifying source code or configuration files
- Tasks that are better suited for direct file manipulation

## Organizing Development Work with Agile Methodology

This section explains HOW to structure your development work using pm-cli, beyond just the command syntax.

### Understanding the Work Item Hierarchy

The pm-cli supports a three-level hierarchy for organizing work:

**Epic** → **Story** → **Task**

#### Epics (High-Level Features)
- Represent major system components, feature areas, or large initiatives
- Examples: "User Authentication System", "Payment Processing", "Mobile App", "API v2"
- Typically span multiple sprints
- Use `--type epic` when creating

#### Stories (Implementation Groupings)
- Represent user-facing capabilities or logical implementation units within an epic
- Should be completable within a sprint
- Include context about WHY this work matters and WHEN it should be done
- Examples: "User Login Flow", "Credit Card Payment Integration", "Profile Management"
- Use `--type story` and `--parent-id <epic-id>` when creating

#### Tasks (Specific Work Items)
- Represent concrete, actionable work items
- Should be completable in days or less
- Examples: "Implement password hashing", "Create login API endpoint", "Write integration tests"
- Use `--type task` and `--parent-id <story-id>` when creating

### Translating Implementation Plans into Work Items

When you have a detailed implementation plan (architecture document, technical design, etc.), organize it systematically:

#### 1. Create Epics for Major Components

Identify the high-level system components or feature areas from your plan:

```bash
# Example: Create an epic for a major feature
EPIC=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type epic \
  --title "Authentication System" \
  --description "Complete user authentication with OAuth2, JWT, and session management" \
  --priority high \
  --pretty)

EPIC_ID=$(echo "$EPIC" | jq -r '.work_item.id')
```

#### 2. Create Stories to Organize Implementation Order

Break down each epic into stories that represent logical implementation phases. **Include implementation context in the story description**:

```bash
# Example: Create a story with context about sequencing
STORY=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type story \
  --parent-id "$EPIC_ID" \
  --title "Foundation - Error Handling & Security Setup" \
  --description "Set up security infrastructure and error handling patterns. This MUST be done first as all auth components depend on proper error types and security utilities." \
  --priority high \
  --pretty)

STORY_ID=$(echo "$STORY" | jq -r '.work_item.id')
```

**What to include in story descriptions:**
- **Summary**: What capability this story delivers
- **Why it matters**: The business or technical reason for this work
- **Implementation order**: Dependencies on other stories, what must be done first
- **Success criteria**: How to know when this story is complete

#### 3. Create Tasks with Implementation Details

Tasks should contain **concrete implementation details** that make them actionable. This is where you include:
- Code snippets and examples
- API contracts and schemas
- Configuration requirements
- File paths and module locations
- Key patterns or conventions to follow

**Example task with implementation details:**

````bash
# Example: Create a task with code snippets
pm work-item create \
  --project-id "$PROJECT_ID" \
  --type task \
  --parent-id "$STORY_ID" \
  --title "Implement AuthError enum with structured logging" \
  --description "$(cat <<'EOF'
Create a comprehensive error type for authentication failures.

**File**: `src/auth/error.rs`

**Implementation**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired at {expired_at}")]
    TokenExpired { expired_at: String },

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, AuthError>;
```

**Requirements**:
- Use thiserror for error derivation
- Include context in error variants
- Implement From traits for error conversion
- Add structured logging fields

**Testing**: Verify error messages are clear and include relevant context
EOF
)" \
  --priority high \
  --pretty
````

**Benefits of detailed task descriptions:**
- ✅ Implementation context preserved across sessions
- ✅ Reduces ambiguity and misunderstandings
- ✅ Serves as documentation for future reference
- ✅ Makes tasks immediately actionable
- ✅ Enables better time estimation

### Managing Dependencies

Use dependencies to represent blocking relationships between work items:

```bash
# Example: Story B depends on Story A being complete
pm dependency create \
  --blocking "$STORY_A_ID" \
  --blocked "$STORY_B_ID" \
  --type blocks \
  --pretty
```

**When to use dependencies:**
- One work item cannot start until another is complete
- Shared infrastructure or APIs need to be built first
- Testing depends on implementation being finished
- Integration work depends on component work

**Best practices:**
- Use story descriptions to indicate soft dependencies ("best done after X")
- Use dependency links for hard blockers only
- Create dependency links between stories, not individual tasks (cleaner graph)
- Review dependencies regularly to ensure they're still accurate

### Organizing Work by Phases

For complex projects, organize work into logical implementation phases:

1. **Identify dependency layers**: Group work items by what they depend on
2. **Create stories for each phase**: Foundation → Core Components → Integration → Polish
3. **Use sprint planning**: Assign stories to sprints based on phase order
4. **Track progress**: Use sprint goals to align team on phase objectives

### Best Practices for LLM Collaboration

When working with AI assistants (like Claude), structure your work items to maximize effectiveness:

1. **Include code examples**: Put actual code snippets in task descriptions (not "implement X", but "implement X like this: ```code```")
2. **Be explicit about patterns**: Specify conventions, error handling patterns, logging standards
3. **Preserve context**: Use comments on work items to capture decisions, blockers, or discoveries
4. **Use time tracking**: Start timers when beginning work to provide visibility into effort
5. **Update status proactively**: Set status to `in_progress` when starting, `blocked` when stuck, `done` when complete
6. **Link related work**: Use comments to reference related tasks, documentation, or external resources

### Workflow Example: From Plan to Execution

Here's a complete workflow for organizing a feature implementation:

````bash
# 1. Create the epic for the feature
EPIC=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type epic \
  --title "Payment Processing System" \
  --description "Integrate Stripe payment processing with webhook handling and receipt generation" \
  --pretty)
EPIC_ID=$(echo "$EPIC" | jq -r '.work_item.id')

# 2. Create stories for implementation phases
# Story 1: Foundation
STORY_1=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type story \
  --parent-id "$EPIC_ID" \
  --title "Foundation - Payment Models & Error Handling" \
  --description "Set up payment domain models and error types. Must be done first." \
  --priority high \
  --pretty)
STORY_1_ID=$(echo "$STORY_1" | jq -r '.work_item.id')

# Story 2: Integration
STORY_2=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type story \
  --parent-id "$EPIC_ID" \
  --title "Stripe API Integration" \
  --description "Integrate with Stripe payment APIs. Depends on Story 1 completion." \
  --priority high \
  --pretty)
STORY_2_ID=$(echo "$STORY_2" | jq -r '.work_item.id')

# Create dependency link
pm dependency create \
  --blocking "$STORY_1_ID" \
  --blocked "$STORY_2_ID" \
  --type blocks \
  --pretty

# 3. Create detailed tasks under Story 1
pm work-item create \
  --project-id "$PROJECT_ID" \
  --type task \
  --parent-id "$STORY_1_ID" \
  --title "Create Payment domain models" \
  --description "$(cat <<'EOF'
File: `src/payments/models.rs`

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub amount: Decimal,
    pub currency: Currency,
    pub status: PaymentStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PaymentStatus {
    Pending,
    Processing,
    Succeeded,
    Failed { reason: String },
}
```
EOF
)" \
  --priority high \
  --pretty

# 4. As work progresses, update status and add comments
pm work-item update "$TASK_ID" \
  --version "$VERSION" \
  --status in_progress \
  --pretty

pm comment create \
  --work-item-id "$TASK_ID" \
  --content "Implemented base models, now adding validation logic" \
  --pretty
````

## Available Commands

### Project Commands

```bash
# List all projects
.pm/bin/pm project list [--pretty]

# Get a specific project by ID
.pm/bin/pm project get <project-id> [--pretty]

# Create a new project
.pm/bin/pm project create \
  --title "Project Title" \
  --key "PROJ" \
  [--description "Project description"] \
  [--pretty]

# Update a project
.pm/bin/pm project update <project-id> \
  --expected-version <current-version> \
  [--title "New title"] \
  [--description "New description"] \
  [--status <active|archived>] \
  [--pretty]

# Delete a project
.pm/bin/pm project delete <project-id> [--pretty]
```

**Valid statuses:** `active`, `archived`

### Work Item Commands

```bash
# List work items in a project (with optional filters)
.pm/bin/pm work-item list <project-id> [--type <epic|story|task>] [--status <status>] [--parent-id <uuid>] [--orphaned] [--include-done] [--pretty]

# Get a specific work item
.pm/bin/pm work-item get <work-item-id> [--pretty]

# Create a new work item
.pm/bin/pm work-item create \
  --project-id <uuid> \
  --type <epic|story|task> \
  --title "Title" \
  [--description "Description"] \
  [--parent-id <uuid>] \
  [--status <backlog|todo|in_progress|review|done|blocked>] \
  [--priority <low|medium|high|critical>] \
  [--pretty]

# Update a work item
.pm/bin/pm work-item update <work-item-id> \
  --version <current-version> \
  [--title "New title"] \
  [--description "New description"] \
  [--status <status>] \
  [--priority <priority>] \
  [--assignee-id <uuid>] \
  [--sprint-id <uuid>] \
  [--story-points <0-100>] \
  [--parent-id <uuid>] \
  [--update-parent] \
  [--position <int>] \
  [--pretty]

# Delete a work item
.pm/bin/pm work-item delete <work-item-id> [--pretty]
```

**Valid statuses:** `backlog`, `todo`, `in_progress`, `review`, `done`, `blocked`
**Valid priorities:** `low`, `medium`, `high`, `critical`
**Valid types:** `epic`, `story`, `task`

### Sprint Commands

```bash
# List sprints in a project
.pm/bin/pm sprint list <project-id> [--pretty]

# Get a specific sprint
.pm/bin/pm sprint get <sprint-id> [--pretty]

# Create a new sprint
.pm/bin/pm sprint create \
  --project-id <uuid> \
  --name "Sprint Name" \
  --start-date <unix-timestamp> \
  --end-date <unix-timestamp> \
  [--goal "Sprint goal"] \
  [--pretty]

# Update a sprint
.pm/bin/pm sprint update <sprint-id> \
  --expected-version <current-version> \
  [--name "New name"] \
  [--goal "New goal"] \
  [--start-date <unix-timestamp>] \
  [--end-date <unix-timestamp>] \
  [--status <planned|active|completed>] \
  [--pretty]

# Delete a sprint
.pm/bin/pm sprint delete <sprint-id> [--pretty]
```

### Comment Commands

```bash
# List comments on a work item
.pm/bin/pm comment list <work-item-id> [--pretty]

# Create a comment
.pm/bin/pm comment create \
  --work-item-id <uuid> \
  --content "Comment text" \
  [--pretty]

# Update a comment
.pm/bin/pm comment update <comment-id> \
  --content "Updated text" \
  [--pretty]

# Delete a comment
.pm/bin/pm comment delete <comment-id> [--pretty]
```

### Dependency Commands

```bash
# List dependencies for a work item
.pm/bin/pm dependency list <work-item-id> [--pretty]

# Create a dependency link
.pm/bin/pm dependency create \
  --blocking <work-item-id> \
  --blocked <work-item-id> \
  --type <blocks|relates_to> \
  [--pretty]

# Delete a dependency
.pm/bin/pm dependency delete <dependency-id> [--pretty]
```

### Time Entry Commands

```bash
# List time entries for a work item
.pm/bin/pm time-entry list <work-item-id> [--pretty]

# Get a specific time entry
.pm/bin/pm time-entry get <time-entry-id> [--pretty]

# Start a timer on a work item
.pm/bin/pm time-entry create \
  --work-item-id <uuid> \
  [--description "What you're working on"] \
  [--pretty]

# Stop a running timer or update description
.pm/bin/pm time-entry update <time-entry-id> \
  [--stop] \
  [--description "Updated description"] \
  [--pretty]

# Delete a time entry
.pm/bin/pm time-entry delete <time-entry-id> [--pretty]
```

### Swim Lane Commands

```bash
# List swim lanes for a project (read-only)
.pm/bin/pm swim-lane list <project-id> [--pretty]

# Get a specific swim lane (read-only)
.pm/bin/pm swim-lane get <swim-lane-id> [--pretty]
```

**Note:** Swim lanes are read-only via CLI. Use the Blazor UI to create/update/delete swim lanes.

### Sync Commands

```bash
# Export all data to JSON
.pm/bin/pm sync export [--output <file>] [--pretty]

# Export a specific work item (scoped export)
.pm/bin/pm sync export [--output <file>] work-item <work-item-id> \
  [--descendant-levels <0-2>] \
  [--comments] \
  [--sprints] \
  [--dependencies] \
  [--time-entries] \
  [--pretty]

# Import data from JSON file
.pm/bin/pm sync import --file <json-file> [--pretty]
```

**Scoped export flags:**
- `--descendant-levels <N>` - Include N levels of children (0=just item, 1=children, 2=grandchildren)
- `--comments` - Include comments for matched work items
- `--sprints` - Include sprint data referenced by matched work items
- `--dependencies` - Include dependency links involving matched work items
- `--time-entries` - Include time entries for matched work items

Without flags, scoped export returns only the work item itself. Each flag opts in additional related data. The response uses the same `ExportData` format as full export (compatible with `sync import`).

### Desktop Command

```bash
# Launch the Tauri desktop application
.pm/bin/pm desktop
```

## Usage Patterns

### Complete Project Setup Workflow

```bash
# 1. Create a new project
PROJECT=$(pm project create \
  --title "My New Project" \
  --key "MNP" \
  --description "A fresh project" \
  --pretty)

PROJECT_ID=$(echo "$PROJECT" | jq -r '.project.id')
PROJECT_VERSION=$(echo "$PROJECT" | jq -r '.project.version')

# 2. Create a sprint
SPRINT=$(pm sprint create \
  --project-id "$PROJECT_ID" \
  --name "Sprint 1" \
  --start-date $(date +%s) \
  --end-date $(($(date +%s) + 1209600)) \
  --goal "Initial setup and core features" \
  --pretty)

SPRINT_ID=$(echo "$SPRINT" | jq -r '.sprint.id')

# 3. Create an epic
EPIC=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type epic \
  --title "User Authentication" \
  --description "Implement complete auth system" \
  --priority high \
  --pretty)

EPIC_ID=$(echo "$EPIC" | jq -r '.work_item.id')

# 4. Create a task under the epic
TASK=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type task \
  --title "Implement OAuth2 login" \
  --description "Add Google OAuth2 provider" \
  --parent-id "$EPIC_ID" \
  --status todo \
  --priority high \
  --pretty)

TASK_ID=$(echo "$TASK" | jq -r '.work_item.id')
TASK_VERSION=$(echo "$TASK" | jq -r '.work_item.version')

# 5. Assign to sprint and update status
TASK=$(pm work-item update "$TASK_ID" \
  --version $TASK_VERSION \
  --sprint-id "$SPRINT_ID" \
  --status in_progress \
  --story-points 5 \
  --pretty)

TASK_VERSION=$(echo "$TASK" | jq -r '.work_item.version')

# 6. Start a timer
TIMER=$(pm time-entry create \
  --work-item-id "$TASK_ID" \
  --description "Working on OAuth implementation" \
  --pretty)

TIMER_ID=$(echo "$TIMER" | jq -r '.time_entry.id')

# 7. Add a comment
.pm/bin/pm comment create \
  --work-item-id "$TASK_ID" \
  --content "Started implementation, setting up OAuth provider" \
  --pretty

# 8. Stop the timer later
.pm/bin/pm time-entry update "$TIMER_ID" \
  --stop \
  --pretty

# 9. Mark task as done
.pm/bin/pm work-item update "$TASK_ID" \
  --version $TASK_VERSION \
  --status done \
  --pretty

echo "Project created: $PROJECT_ID"
echo "Sprint created: $SPRINT_ID"
echo "Epic created: $EPIC_ID"
echo "Task created and completed: $TASK_ID"
```

### Querying Project Status

```bash
# Get all in-progress tasks
.pm/bin/pm work-item list "$PROJECT_ID" \
  --type task \
  --status in_progress \
  --pretty

# Get all high-priority items
.pm/bin/pm work-item list "$PROJECT_ID" \
  --priority high \
  --pretty
```

### Quick Task Creation

```bash
# Get existing project
PROJECT_ID=$(pm project list --pretty | jq -r '.projects[0].id')

# Create a simple task
TASK=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type task \
  --title "Fix login bug" \
  --priority high \
  --pretty)

TASK_ID=$(echo "$TASK" | jq -r '.work_item.id')

# Add a comment
.pm/bin/pm comment create \
  --work-item-id "$TASK_ID" \
  --content "Investigating the issue" \
  --pretty
```

### Managing Project Lifecycle

```bash
# Create project
PROJECT=$(pm project create \
  --title "Q1 Initiative" \
  --key "Q1I" \
  --pretty)

PROJECT_ID=$(echo "$PROJECT" | jq -r '.project.id')
VERSION=$(echo "$PROJECT" | jq -r '.project.version')

# Update project description
PROJECT=$(pm project update "$PROJECT_ID" \
  --expected-version $VERSION \
  --description "Focus on customer experience improvements" \
  --pretty)

VERSION=$(echo "$PROJECT" | jq -r '.project.version')

# Archive project when done
.pm/bin/pm project update "$PROJECT_ID" \
  --expected-version $VERSION \
  --status archived \
  --pretty
```

### Sprint Planning and Updates

```bash
# Create a 2-week sprint
START_DATE=$(date +%s)
END_DATE=$((START_DATE + 1209600))  # 2 weeks in seconds

SPRINT=$(pm sprint create \
  --project-id "$PROJECT_ID" \
  --name "Sprint 1" \
  --start-date $START_DATE \
  --end-date $END_DATE \
  --goal "Implement core features" \
  --pretty)

SPRINT_ID=$(echo "$SPRINT" | jq -r '.sprint.id')
VERSION=$(echo "$SPRINT" | jq -r '.sprint.version')

# Start the sprint
.pm/bin/pm sprint update "$SPRINT_ID" \
  --expected-version $VERSION \
  --status active \
  --pretty
```

### Tracking Dependencies

```bash
# Create two tasks
TASK_A=$(pm work-item create --project-id "$PROJECT_ID" --type task --title "Setup database" --pretty)
TASK_B=$(pm work-item create --project-id "$PROJECT_ID" --type task --title "Create migrations" --pretty)

TASK_A_ID=$(echo "$TASK_A" | jq -r '.work_item.id')
TASK_B_ID=$(echo "$TASK_B" | jq -r '.work_item.id')

# Create a blocking dependency (A blocks B)
.pm/bin/pm dependency create \
  --blocking "$TASK_A_ID" \
  --blocked "$TASK_B_ID" \
  --type blocks \
  --pretty

# List dependencies for task B
.pm/bin/pm dependency list "$TASK_B_ID" --pretty
```

### Reparenting Work Items

Move work items between parents or orphan them using `--parent-id` and `--update-parent`:

```bash
# Move an orphan task under a story
.pm/bin/pm work-item update "$TASK_ID" \
  --version "$VERSION" \
  --parent-id "$STORY_ID" \
  --update-parent \
  --pretty

# Move a task from one story to another
.pm/bin/pm work-item update "$TASK_ID" \
  --version "$VERSION" \
  --parent-id "$NEW_STORY_ID" \
  --update-parent \
  --pretty

# Orphan a task (remove its parent)
.pm/bin/pm work-item update "$TASK_ID" \
  --version "$VERSION" \
  --parent-id "" \
  --update-parent \
  --pretty
```

**Why `--update-parent` is required:** Without the flag, omitting `--parent-id` means "don't change the parent." With the flag, omitting `--parent-id` (or passing empty string) means "clear the parent." The flag disambiguates intent.

### Filtering by Parent and Finding Orphans

```bash
# List all children of a specific epic
.pm/bin/pm work-item list "$PROJECT_ID" \
  --parent-id "$EPIC_ID" \
  --pretty

# List only orphaned items (no parent) — useful for finding misplaced work items
.pm/bin/pm work-item list "$PROJECT_ID" \
  --orphaned \
  --pretty

# List orphaned stories specifically
.pm/bin/pm work-item list "$PROJECT_ID" \
  --orphaned \
  --type story \
  --pretty

# Combine filters: children of an epic that are in_progress
.pm/bin/pm work-item list "$PROJECT_ID" \
  --parent-id "$EPIC_ID" \
  --status in_progress \
  --pretty
```

**Note:** `--parent-id`, `--orphaned`, and `--descendants-of` are mutually exclusive (enforced by the CLI).

### Listing All Descendants (Recursive)

```bash
# All descendants of an epic (stories + their tasks)
.pm/bin/pm work-item list "$PROJECT_ID" \
  --descendants-of "$EPIC_ID" \
  --pretty

# Just the tasks in an epic's entire tree
.pm/bin/pm work-item list "$PROJECT_ID" \
  --descendants-of "$EPIC_ID" \
  --type task \
  --pretty

# All in-progress items under an epic's tree
.pm/bin/pm work-item list "$PROJECT_ID" \
  --descendants-of "$EPIC_ID" \
  --status in_progress \
  --pretty
```

### Scoped Export (Work Item with Context)

```bash
# Export an epic with its full tree and all related data
.pm/bin/pm sync export --output epic-export.json work-item "$EPIC_ID" \
  --descendant-levels 2 \
  --comments \
  --sprints \
  --dependencies \
  --time-entries \
  --pretty

# Export just a single work item (no related data)
.pm/bin/pm sync export work-item "$WORK_ITEM_ID" --pretty

# Export a story with its child tasks and comments
.pm/bin/pm sync export --output story.json work-item "$STORY_ID" \
  --descendant-levels 1 \
  --comments \
  --pretty
```

## Response Format

All commands return JSON responses with consistent structure:

**Success response:**
```json
{
  "work_item": { ... },
  "comment": { ... },
  "sprint": { ... },
  // etc.
}
```

**Error response:**
```json
{
  "error": {
    "code": "NOT_FOUND | VALIDATION_ERROR | CONFLICT | INTERNAL_ERROR",
    "message": "Human-readable error message"
  }
}
```

**Exit codes:**
- `0` - Success
- `1` - Error (check stderr and JSON response)

## Critical Rules

### NEVER Delete and Recreate Work Items — Always Update

**NEVER use `work-item delete` followed by `work-item create` to fix a work item.** Always use `work-item update` instead.

Deleting a work item destroys:
- Dependency links (blocks/blocked-by relationships)
- Comments and discussion history
- Time entries and tracked effort
- Sprint assignments
- Activity log references

It also leaves gaps in display key numbering (e.g., PROJ-124 disappears, replacement becomes PROJ-125).

```bash
# WRONG — destroys all associated data
.pm/bin/pm work-item delete "$TASK_ID"
.pm/bin/pm work-item create --project-id "$PROJECT_ID" --type task --title "Fixed title" ...

# CORRECT — preserves all relationships and history
.pm/bin/pm work-item update "$TASK_ID" --version "$VERSION" --title "Fixed title" --description "Fixed description"
```

Only delete a work item if it was created in error and has no dependencies, comments, or other references.

## Important Notes

### Done Items Excluded by Default

`work-item list` excludes items with status `done` by default. Add `--include-done` to see them:
```bash
# Active items only (default)
.pm/bin/pm work-item list "$PROJECT_ID" --pretty

# Include completed items
.pm/bin/pm work-item list "$PROJECT_ID" --include-done --pretty
```

### Multi-line Descriptions with Special Characters

Descriptions containing backticks, quotes, or markdown code blocks require proper shell quoting. **Double quotes will break** because bash interprets backticks as command substitution.

**Simple descriptions (no backticks)** - double quotes are fine:
```bash
.pm/bin/pm work-item create \
  --project-id "$PROJECT_ID" \
  --type task \
  --title "My task" \
  --description "A simple multi-line
description works fine in double quotes
as long as there are no backticks" \
  --pretty
```

**Descriptions with backticks/code blocks** - use single quotes:
````bash
.pm/bin/pm work-item create \
  --project-id "$PROJECT_ID" \
  --type task \
  --title "My task" \
  --description 'Implement the handler:

```rust
pub fn handle_request(req: Request) -> Response {
    Response::ok()
}
```

See `README.md` for details.' \
  --pretty
````

**Descriptions with backticks AND shell variables** - use heredoc with single-quoted delimiter:
````bash
.pm/bin/pm work-item create \
  --project-id "$PROJECT_ID" \
  --type task \
  --title "My task" \
  --description "$(cat <<'EOF'
Implement the handler in `src/api.rs`:

```rust
pub fn handle_request(req: Request) -> Response {
    Response::ok()
}
```

Must pass all existing tests.
EOF
)" \
  --pretty
````

The `<<'EOF'` (single-quoted) delimiter prevents bash from interpreting backticks, `$variables`, and other special characters inside the heredoc. The `$(cat ...)` wrapper captures the heredoc content as a string for the `--description` argument.

### Known Issues

1. **No swim lane CRUD** - Swim lanes are read-only via CLI. Create/update/delete operations must be done through the Blazor UI.

2. **No dependency update** - Dependencies can only be created or deleted. There is no update operation (dependencies are immutable links between work items).

### Optimistic Locking

Work item, project, and sprint updates require a version parameter (`--version` or `--expected-version`) matching the current version number. If the version doesn't match (concurrent update), you'll get a `CONFLICT` error. Always fetch the latest version before updating:

```bash
# Get current version
CURRENT=$(pm work-item get "$TASK_ID" --pretty)
VERSION=$(echo "$CURRENT" | jq -r '.work_item.version')

# Update with correct version
.pm/bin/pm work-item update "$TASK_ID" \
  --status in_progress \
  --version "$VERSION" \
  --pretty
```

### Real-time Synchronization

All CLI operations trigger WebSocket broadcasts to connected clients. Changes made via CLI appear **instantly** in:
- Blazor WebAssembly UI
- Tauri Desktop App
- Any other connected WebSocket clients

The CLI and UI are always synchronized through the WebSocket layer.

### LLM Integration

The CLI is designed for LLM and automation:
- ✅ All responses are valid JSON
- ✅ Structured error codes for programmatic handling
- ✅ Consistent response format across all commands
- ✅ Pretty-print option for human-readable output
- ✅ Global options for server URL and user ID configuration

### Server Auto-discovery

The CLI automatically discovers the server URL from `.pm/server.json` (written by the server on startup). You can override this with `--server <URL>` if needed.

### User Identity

By default, the CLI uses the "LLM user" configured in `.pm/config.toml`. Override with `--user-id <UUID>` to perform operations as a different user.

## Additional Information

**Configuration files:**
- `.pm/config.toml` - CLI configuration and LLM user settings
- `.pm/server.json` - Auto-discovered server URL and port

**Real-time synchronization:**
All CLI operations trigger WebSocket broadcasts, so changes appear instantly in the Blazor UI and desktop app.
