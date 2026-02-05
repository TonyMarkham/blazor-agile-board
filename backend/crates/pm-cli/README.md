# pm CLI - Complete Command Reference

Command-line interface for Blazor Agile Board. All changes made via CLI appear instantly in the Blazor UI through WebSocket broadcasts.

## Installation

```bash
# Build release binary
just build-rs-cli-release

# Binary location
./target/release/pm

# Or run directly with cargo
cargo run -p pm-cli -- <command>
```

## Global Options

These options work with all commands:

| Option | Description | Default |
|--------|-------------|---------|
| `--server <URL>` | Server URL | `http://127.0.0.1:8000` |
| `--user-id <UUID>` | User ID for operations | LLM user |
| `--pretty` | Pretty-print JSON output | false |
| `-h, --help` | Show help | - |
| `-V, --version` | Show CLI version | - |

## Project Commands

### `pm project list`

List all projects.

**Usage:**
```bash
pm project list [OPTIONS]
```

**Example:**
```bash
pm project list --pretty
```

**Output:**
```json
{
  "projects": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "key": "PROJ",
      "title": "My Project",
      "description": "A sample project",
      "created_at": 1704067200,
      "updated_at": 1704067200
    }
  ]
}
```

---

### `pm project get`

Get a project by ID.

**Usage:**
```bash
pm project get [OPTIONS] <ID>
```

**Arguments:**
- `<ID>` - Project ID (UUID)

**Example:**
```bash
pm project get 550e8400-e29b-41d4-a716-446655440000 --pretty
```

**Output:**
```json
{
  "project": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "key": "PROJ",
    "title": "My Project",
    "description": "A sample project",
    "created_at": 1704067200,
    "updated_at": 1704067200
  }
}
```

---

## Work Item Commands

### `pm work-item create`

Create a new work item (epic, story, or task).

**Usage:**
```bash
pm work-item create [OPTIONS] \
  --project-id <PROJECT_ID> \
  --type <TYPE> \
  --title <TITLE>
```

**Required Options:**
- `--project-id <PROJECT_ID>` - Project ID (UUID)
- `--type <TYPE>` - Item type: `epic`, `story`, or `task`
- `--title <TITLE>` - Work item title

**Optional Parameters:**
- `--description <DESCRIPTION>` - Work item description
- `--parent-id <PARENT_ID>` - Parent work item ID (UUID) for hierarchies
- `--status <STATUS>` - Initial status (default: `backlog`)
  - Valid: `backlog`, `todo`, `in_progress`, `review`, `done`, `blocked`
- `--priority <PRIORITY>` - Priority (default: `medium`)
  - Valid: `low`, `medium`, `high`, `critical`

**Examples:**

Create a simple task:
```bash
pm work-item create \
  --project-id 550e8400-e29b-41d4-a716-446655440000 \
  --type task \
  --title "Implement user authentication" \
  --pretty
```

Create a task with all options:
```bash
pm work-item create \
  --project-id 550e8400-e29b-41d4-a716-446655440000 \
  --type task \
  --title "Add OAuth2 login" \
  --description "Implement OAuth2 authentication with Google provider" \
  --status todo \
  --priority high \
  --pretty
```

Create a story with a parent epic:
```bash
pm work-item create \
  --project-id 550e8400-e29b-41d4-a716-446655440000 \
  --type story \
  --title "User authentication flow" \
  --parent-id 660e8400-e29b-41d4-a716-446655440001 \
  --pretty
```

**Output:**
```json
{
  "work_item": {
    "id": "770e8400-e29b-41d4-a716-446655440002",
    "display_key": "PROJ-1",
    "item_type": "task",
    "title": "Implement user authentication",
    "description": null,
    "status": "backlog",
    "priority": "medium",
    "parent_id": null,
    "project_id": "550e8400-e29b-41d4-a716-446655440000",
    "assignee_id": null,
    "sprint_id": null,
    "story_points": null,
    "item_number": 1,
    "position": 1,
    "version": 1,
    "created_at": 1704067200,
    "updated_at": 1704067200,
    "created_by": "880e8400-e29b-41d4-a716-446655440003",
    "updated_by": "880e8400-e29b-41d4-a716-446655440003"
  }
}
```

---

### `pm work-item get`

Get a work item by ID.

**Usage:**
```bash
pm work-item get [OPTIONS] <ID>
```

**Arguments:**
- `<ID>` - Work item ID (UUID)

**Example:**
```bash
pm work-item get 770e8400-e29b-41d4-a716-446655440002 --pretty
```

---

### `pm work-item list`

List work items in a project with optional filters.

**Usage:**
```bash
pm work-item list [OPTIONS] <PROJECT_ID>
```

**Arguments:**
- `<PROJECT_ID>` - Project ID (UUID)

**Optional Filters:**
- `--type <TYPE>` - Filter by type: `epic`, `story`, or `task`
- `--status <STATUS>` - Filter by status

**Examples:**

List all work items:
```bash
pm work-item list 550e8400-e29b-41d4-a716-446655440000 --pretty
```

List only tasks:
```bash
pm work-item list 550e8400-e29b-41d4-a716-446655440000 \
  --type task \
  --pretty
```

List tasks in progress:
```bash
pm work-item list 550e8400-e29b-41d4-a716-446655440000 \
  --type task \
  --status in_progress \
  --pretty
```

**Output:**
```json
{
  "work_items": [
    {
      "id": "770e8400-e29b-41d4-a716-446655440002",
      "display_key": "PROJ-1",
      "item_type": "task",
      "title": "Implement user authentication",
      "status": "in_progress",
      "priority": "medium",
      "version": 2
    }
  ]
}
```

---

### `pm work-item update`

⚠️ **KNOWN BUG:** This command currently panics due to a conflict between `--version` (optimistic locking) and clap's auto-generated `--version` flag. **Needs to be fixed.**

Update a work item's properties.

**Usage:**
```bash
pm work-item update [OPTIONS] \
  --version <VERSION> \
  <ID>
```

**Arguments:**
- `<ID>` - Work item ID (UUID)

**Required:**
- `--version <VERSION>` - Current version number (for optimistic locking)

**Optional Updates:**
- `--title <TITLE>` - New title
- `--description <DESCRIPTION>` - New description
- `--status <STATUS>` - New status
  - Valid: `backlog`, `todo`, `in_progress`, `review`, `done`, `blocked`
- `--priority <PRIORITY>` - New priority
  - Valid: `low`, `medium`, `high`, `critical`
- `--assignee-id <UUID>` - Assign to user
- `--sprint-id <UUID>` - Add to sprint
- `--story-points <0-100>` - Story points estimate

**Examples:**

Update status:
```bash
pm work-item update 770e8400-e29b-41d4-a716-446655440002 \
  --status in_progress \
  --version 1 \
  --pretty
```

Assign and set priority:
```bash
pm work-item update 770e8400-e29b-41d4-a716-446655440002 \
  --assignee-id 990e8400-e29b-41d4-a716-446655440004 \
  --priority high \
  --version 2 \
  --pretty
```

Add story points:
```bash
pm work-item update 770e8400-e29b-41d4-a716-446655440002 \
  --story-points 5 \
  --version 3 \
  --pretty
```

**Optimistic Locking:**

The `--version` parameter is **required** and must match the current version. If someone else updated the work item, you'll get a conflict error:

```json
{
  "error": {
    "code": "CONFLICT",
    "message": "Version mismatch (current version: 4)"
  }
}
```

You must then fetch the latest version and retry with the correct version number.

---

### `pm work-item delete`

Delete a work item.

**Usage:**
```bash
pm work-item delete [OPTIONS] <ID>
```

**Arguments:**
- `<ID>` - Work item ID (UUID)

**Example:**
```bash
pm work-item delete 770e8400-e29b-41d4-a716-446655440002 --pretty
```

**Output:**
```json
{
  "deleted_id": "770e8400-e29b-41d4-a716-446655440002"
}
```

---

## Comment Commands

### `pm comment list`

List all comments on a work item.

**Usage:**
```bash
pm comment list [OPTIONS] <WORK_ITEM_ID>
```

**Arguments:**
- `<WORK_ITEM_ID>` - Work item ID (UUID)

**Example:**
```bash
pm comment list 770e8400-e29b-41d4-a716-446655440002 --pretty
```

**Output:**
```json
{
  "comments": [
    {
      "id": "aa0e8400-e29b-41d4-a716-446655440005",
      "work_item_id": "770e8400-e29b-41d4-a716-446655440002",
      "content": "This looks good to me",
      "created_at": 1704067200,
      "updated_at": 1704067200,
      "created_by": "990e8400-e29b-41d4-a716-446655440004",
      "updated_by": "990e8400-e29b-41d4-a716-446655440004"
    }
  ]
}
```

---

### `pm comment create`

Create a comment on a work item.

**Usage:**
```bash
pm comment create [OPTIONS] \
  --work-item-id <WORK_ITEM_ID> \
  --content <CONTENT>
```

**Required Options:**
- `--work-item-id <WORK_ITEM_ID>` - Work item ID (UUID)
- `--content <CONTENT>` - Comment text

**Example:**
```bash
pm comment create \
  --work-item-id 770e8400-e29b-41d4-a716-446655440002 \
  --content "Looks good! Approved for merge." \
  --pretty
```

**Output:**
```json
{
  "comment": {
    "id": "aa0e8400-e29b-41d4-a716-446655440005",
    "work_item_id": "770e8400-e29b-41d4-a716-446655440002",
    "content": "Looks good! Approved for merge.",
    "created_at": 1704067200,
    "updated_at": 1704067200,
    "created_by": "990e8400-e29b-41d4-a716-446655440004",
    "updated_by": "990e8400-e29b-41d4-a716-446655440004"
  }
}
```

---

### `pm comment update`

Update a comment's content.

**Usage:**
```bash
pm comment update [OPTIONS] \
  --content <CONTENT> \
  <ID>
```

**Arguments:**
- `<ID>` - Comment ID (UUID)

**Required Options:**
- `--content <CONTENT>` - New comment text

**Example:**
```bash
pm comment update aa0e8400-e29b-41d4-a716-446655440005 \
  --content "Updated: Looks great! Ready to merge." \
  --pretty
```

---

### `pm comment delete`

Delete a comment.

**Usage:**
```bash
pm comment delete [OPTIONS] <ID>
```

**Arguments:**
- `<ID>` - Comment ID (UUID)

**Example:**
```bash
pm comment delete aa0e8400-e29b-41d4-a716-446655440005 --pretty
```

**Output:**
```json
{
  "deleted_id": "aa0e8400-e29b-41d4-a716-446655440005"
}
```

---

## Error Handling

All errors return JSON with structured error information:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message"
  }
}
```

**Common Error Codes:**
- `NOT_FOUND` - Resource doesn't exist
- `VALIDATION_ERROR` - Invalid input (check field constraints)
- `CONFLICT` - Version mismatch (optimistic locking failure)
- `INTERNAL_ERROR` - Server error

**Exit Codes:**
- `0` - Success
- `1` - Error (check stderr and JSON response)

**Example Error:**
```bash
$ pm work-item get 00000000-0000-0000-0000-000000000000 --pretty
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Work item not found"
  }
}
```

---

## LLM Integration

The CLI is designed for LLM and automation use:

✅ **All responses are valid JSON**
✅ **Structured error codes for programmatic handling**
✅ **Consistent response format**
✅ **Pretty-print option for development**

**Example Automation Script:**
```bash
#!/bin/bash
set -e

# Get project ID
PROJECT_ID=$(pm project list | jq -r '.projects[0].id')

# Create a task
RESPONSE=$(pm work-item create \
  --project-id "$PROJECT_ID" \
  --type task \
  --title "Automated task" \
  --description "Created by script")

# Extract task ID and version
TASK_ID=$(echo "$RESPONSE" | jq -r '.work_item.id')
VERSION=$(echo "$RESPONSE" | jq -r '.work_item.version')

# Update to in_progress
pm work-item update "$TASK_ID" \
  --status in_progress \
  --version "$VERSION"

# Add a comment
pm comment create \
  --work-item-id "$TASK_ID" \
  --content "Task started automatically"

echo "Created and started task: $TASK_ID"
```

---

## Real-time Synchronization

All CLI operations trigger WebSocket broadcasts to connected clients:

- ✅ **Create** → Appears instantly in Blazor UI
- ✅ **Update** → Updates instantly in Blazor UI
- ✅ **Delete** → Removes instantly in Blazor UI
- ✅ **Comment** → Shows instantly in Blazor UI

The CLI and UI are always synchronized through the WebSocket layer.

---

## Known Issues

1. **`pm work-item update` panic** - Conflict between `--version` parameter and clap's auto-generated `--version` flag. Needs `#[command(disable_version_flag = true)]` fix.

2. **No project CRUD** - Projects can only be listed/viewed via CLI. Use Blazor UI to create/update/delete projects.

---

## Development

**Build commands:**
```bash
just build-rs-cli              # Debug build
just build-rs-cli-release      # Release build
just check-rs-cli              # Fast compile check
just clippy-rs-cli             # Lint
just test-rs-cli               # Run all tests
just watch-rs-cli              # Auto-rebuild on changes
```

**Run without installing:**
```bash
just run-cli project list --pretty
```

---

## Testing

```bash
# All tests (12 tests: 4 unit + 8 integration)
just test-rs-cli

# Unit tests only
cargo test -p pm-cli --lib

# Integration tests only
cargo test -p pm-cli --test client_integration_tests
```

All integration tests use `wiremock` to mock the REST API server.
