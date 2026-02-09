# pm-cli - Blazor Agile Board CLI

Command-line interface for programmatic interaction with the Blazor Agile Board project management system. All operations sync in real-time with the Blazor UI through WebSocket broadcasts.

## Quick Reference

**Binary name:** `pm` (located at `./target/release/pm` after build)

**Build command:** `just build-rs-cli-release`

**Global options:**
- `--server <URL>` - Server URL (default: auto-discovered from `.pm/server.json`)
- `--user-id <UUID>` - User ID for operations (default: LLM user from config)
- `--pretty` - Pretty-print JSON output (recommended for development)

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
- Building or compiling the CLI (use `just` commands directly)
- Modifying the CLI source code (use regular development workflow)

## Available Commands

### Project Commands

```bash
# List all projects
pm project list [--pretty]

# Get a specific project by ID
pm project get <project-id> [--pretty]

# Create a new project
pm project create \
  --title "Project Title" \
  --key "PROJ" \
  [--description "Project description"] \
  [--pretty]

# Update a project
pm project update <project-id> \
  --expected-version <current-version> \
  [--title "New title"] \
  [--description "New description"] \
  [--status <active|archived>] \
  [--pretty]

# Delete a project
pm project delete <project-id> [--pretty]
```

**Valid statuses:** `active`, `archived`

### Work Item Commands

```bash
# List work items in a project (with optional filters)
pm work-item list <project-id> [--type <epic|story|task>] [--status <status>] [--pretty]

# Get a specific work item
pm work-item get <work-item-id> [--pretty]

# Create a new work item
pm work-item create \
  --project-id <uuid> \
  --type <epic|story|task> \
  --title "Title" \
  [--description "Description"] \
  [--parent-id <uuid>] \
  [--status <backlog|todo|in_progress|review|done|blocked>] \
  [--priority <low|medium|high|critical>] \
  [--pretty]

# Update a work item
pm work-item update <work-item-id> \
  --version <current-version> \
  [--title "New title"] \
  [--description "New description"] \
  [--status <status>] \
  [--priority <priority>] \
  [--assignee-id <uuid>] \
  [--sprint-id <uuid>] \
  [--story-points <0-100>] \
  [--pretty]

# Delete a work item
pm work-item delete <work-item-id> [--pretty]
```

**Valid statuses:** `backlog`, `todo`, `in_progress`, `review`, `done`, `blocked`
**Valid priorities:** `low`, `medium`, `high`, `critical`
**Valid types:** `epic`, `story`, `task`

### Sprint Commands

```bash
# List sprints in a project
pm sprint list <project-id> [--pretty]

# Get a specific sprint
pm sprint get <sprint-id> [--pretty]

# Create a new sprint
pm sprint create \
  --project-id <uuid> \
  --name "Sprint Name" \
  --start-date <unix-timestamp> \
  --end-date <unix-timestamp> \
  [--goal "Sprint goal"] \
  [--pretty]

# Update a sprint
pm sprint update <sprint-id> \
  --expected-version <current-version> \
  [--name "New name"] \
  [--goal "New goal"] \
  [--start-date <unix-timestamp>] \
  [--end-date <unix-timestamp>] \
  [--status <planned|active|completed>] \
  [--pretty]

# Delete a sprint
pm sprint delete <sprint-id> [--pretty]
```

### Comment Commands

```bash
# List comments on a work item
pm comment list <work-item-id> [--pretty]

# Create a comment
pm comment create \
  --work-item-id <uuid> \
  --content "Comment text" \
  [--pretty]

# Update a comment
pm comment update <comment-id> \
  --content "Updated text" \
  [--pretty]

# Delete a comment
pm comment delete <comment-id> [--pretty]
```

### Dependency Commands

```bash
# List dependencies for a work item
pm dependency list <work-item-id> [--pretty]

# Create a dependency link
pm dependency create \
  --blocking <work-item-id> \
  --blocked <work-item-id> \
  --type <blocks|relates_to> \
  [--pretty]

# Delete a dependency
pm dependency delete <dependency-id> [--pretty]
```

### Time Entry Commands

```bash
# List time entries for a work item
pm time-entry list <work-item-id> [--pretty]

# Get a specific time entry
pm time-entry get <time-entry-id> [--pretty]

# Start a timer on a work item
pm time-entry create \
  --work-item-id <uuid> \
  [--description "What you're working on"] \
  [--pretty]

# Stop a running timer or update description
pm time-entry update <time-entry-id> \
  [--stop] \
  [--description "Updated description"] \
  [--pretty]

# Delete a time entry
pm time-entry delete <time-entry-id> [--pretty]
```

### Swim Lane Commands

```bash
# List swim lanes for a project (read-only)
pm swim-lane list <project-id> [--pretty]

# Get a specific swim lane (read-only)
pm swim-lane get <swim-lane-id> [--pretty]
```

**Note:** Swim lanes are read-only via CLI. Use the Blazor UI to create/update/delete swim lanes.

### Sync Commands

```bash
# Export all data to JSON
pm sync export [--output <file>] [--pretty]

# Import data from JSON file
pm sync import --file <json-file> [--pretty]
```

### Desktop Command

```bash
# Launch the Tauri desktop application
pm desktop
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
pm comment create \
  --work-item-id "$TASK_ID" \
  --content "Started implementation, setting up OAuth provider" \
  --pretty

# 8. Stop the timer later
pm time-entry update "$TIMER_ID" \
  --stop \
  --pretty

# 9. Mark task as done
pm work-item update "$TASK_ID" \
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
pm work-item list "$PROJECT_ID" \
  --type task \
  --status in_progress \
  --pretty

# Get all high-priority items
pm work-item list "$PROJECT_ID" \
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
pm comment create \
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
pm project update "$PROJECT_ID" \
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
pm sprint update "$SPRINT_ID" \
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
pm dependency create \
  --blocking "$TASK_A_ID" \
  --blocked "$TASK_B_ID" \
  --type blocks \
  --pretty

# List dependencies for task B
pm dependency list "$TASK_B_ID" --pretty
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

## Important Notes

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
pm work-item update "$TASK_ID" \
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

## Development

**Build commands:**
```bash
just build-rs-cli              # Debug build
just build-rs-cli-release      # Release build (recommended)
just check-rs-cli              # Fast compile check
just clippy-rs-cli             # Lint
just test-rs-cli               # Run all tests (12 tests: 4 unit + 8 integration)
just watch-rs-cli              # Auto-rebuild on changes
```

**Run without installing:**
```bash
cargo run -p pm-cli -- project list --pretty
# or
just run-cli project list --pretty
```

## Architecture

The CLI is a thin client over the REST API:

1. **HTTP client** - Uses `reqwest` for REST API calls
2. **JSON serialization** - Uses `serde_json` for request/response handling
3. **Error handling** - Structured error types with `thiserror`
4. **CLI framework** - Uses `clap` for argument parsing
5. **Configuration** - Reads from `.pm/config.toml` and `.pm/server.json`

**REST endpoints used:**
- `/api/llm/*` - LLM-friendly read-only endpoints
- All endpoints return JSON and trigger WebSocket broadcasts

## Files and Structure

```
backend/crates/pm-cli/
├── src/
│   ├── main.rs                    # CLI entry point
│   ├── lib.rs                     # Library exports
│   ├── cli.rs                     # Clap CLI definition
│   ├── commands.rs                # Command enum
│   ├── client/                    # HTTP client
│   │   ├── client.rs              # REST API client
│   │   └── error.rs               # Error types
│   ├── project_commands.rs        # Project subcommands
│   ├── work_item_commands.rs      # Work item subcommands
│   ├── sprint_commands.rs         # Sprint subcommands
│   ├── comment_commands.rs        # Comment subcommands
│   ├── dependency_commands.rs     # Dependency subcommands
│   ├── swim_lane_commands.rs      # Swim lane subcommands (read-only)
│   ├── time_entry_commands.rs     # Time entry subcommands
│   └── sync_commands.rs           # Bulk sync subcommands
├── tests/
│   └── client_integration_tests.rs  # Integration tests (wiremock)
├── Cargo.toml                     # Package manifest
└── README.md                      # Complete command reference

Binary output: ./target/release/pm
```

## See Also

- **Complete CLI reference:** `backend/crates/pm-cli/README.md`
- **REST API documentation:** `docs/rest-api.md` (if exists)
- **WebSocket protocol:** `docs/websocket-protocol.md`
- **Configuration:** `.pm/config.toml` and `.pm/server.json`
