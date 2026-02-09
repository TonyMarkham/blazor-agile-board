# Git Hooks for Automated Sync

This document explains the git hooks that automate the agile board data sync workflow.

## Overview

Binary SQLite files (`data.db`) cannot be merged by git. To enable team collaboration, we use a **SQLite + JSON sync pattern**:

- **SQLite** (`data.db`) - Fast local queries, gitignored
- **JSON** (`data.json`) - Git-tracked export, human-readable, mergeable

Git hooks automate the export/import cycle so you never have to remember to sync manually.

## Available Hooks

### Pre-Commit Hook

**File:** `docs/hooks/pre-commit`

**When it runs:** Before every `git commit`

**What it does:**
1. Checks if `.pm/data.db` exists
2. Exports data to `.pm/data.json` using `pm sync export`
3. Stages `data.json` for the commit
4. Aborts commit if export fails

**Why it matters:** Ensures every commit includes the latest data state. Your teammates always get up-to-date project information.

**Bypass:** Use `git commit --no-verify` to skip the hook (not recommended).

---

### Post-Merge Hook

**File:** `docs/hooks/post-merge`

**When it runs:** After every `git pull` or `git merge`

**What it does:**
1. Checks if `data.json` was modified in the merge
2. If yes, imports it using `pm sync import --merge`
3. Never aborts (warnings only)

**Why it matters:** Automatically updates your local database with teammates' changes. No manual import needed.

**Merge strategy:** Uses `--merge` flag (last-write-wins, preserves local-only data).

---

### Post-Checkout Hook

**File:** `docs/hooks/post-checkout`

**When it runs:** After `git checkout` or `git switch` (branch switches only)

**What it does:**
1. Detects branch checkout (not file checkout)
2. Imports `data.json` for the new branch
3. Never aborts (warnings only)

**Why it matters:** Each branch can have different project state. Switching branches automatically loads the correct data.

---

## Installation

### Manual Installation

```bash
# Copy templates to .git/hooks/
cp docs/hooks/pre-commit .git/hooks/
cp docs/hooks/post-merge .git/hooks/
cp docs/hooks/post-checkout .git/hooks/

# Make them executable
chmod +x .git/hooks/pre-commit
chmod +x .git/hooks/post-merge
chmod +x .git/hooks/post-checkout
```

### Verification

After installation, test each hook:

**Test pre-commit:**
```bash
# Make a data change
pm work-item create --project-id <uuid> --type task --title "Test hook"

# Commit - should auto-export
git commit -am "Test pre-commit hook"

# Verify data.json was included
git log -1 --name-only | grep data.json
```

**Test post-merge:**
```bash
# Pull changes (or create a test merge)
git pull

# If data.json changed, you should see:
# "Importing updated agile board data..."
# "✓ Imported .pm/data.json"
```

**Test post-checkout:**
```bash
# Create and switch to a new branch
git checkout -b test-branch

# Should see:
# "Importing agile board data for this branch..."
# "✓ Imported .pm/data.json"
```

---

## Hook Requirements

All hooks gracefully handle missing dependencies:

- If `.pm/data.db` doesn't exist → silently skip (project doesn't use pm-server)
- If `pm` CLI not installed → warning message (pre-commit) or silent skip (post hooks)
- If pm-server not running → warning message with instructions

**Pre-commit** is the only hook that aborts on failure (data export is critical for sync).

**Post-merge** and **post-checkout** never abort (the merge/checkout already happened).

---

## Developer Workflow with Hooks

### Developer A: Make Changes

```bash
# Work on features
pm work-item create --project-id <uuid> --type story --title "New feature"
pm work-item update <id> --status in_progress

# Commit - pre-commit hook auto-exports
git commit -am "Add new feature work item"

# Push
git push
```

Behind the scenes:
1. Pre-commit hook runs `pm sync export`
2. Stages `.pm/data.json`
3. Commit includes updated data
4. Push sends data to team

---

### Developer B: Pull Updates

```bash
# Pull changes
git pull

# Post-merge hook auto-imports
# You immediately see Developer A's work items
pm work-item list
```

Behind the scenes:
1. Git downloads updated `data.json`
2. Post-merge hook detects the change
3. Runs `pm sync import --merge`
4. Your `data.db` now has the latest data

---

### Branch-Based Workflows

```bash
# Main branch: production projects
git checkout main
# Post-checkout imports main's data.json

# Feature branch: experimental projects
git checkout -b feature/new-board
# Post-checkout imports feature branch's data.json

# Each branch has independent agile board state
```

---

## Conflict Resolution

The `--merge` flag uses **timestamp-based conflict resolution** ("last write wins"):

| Scenario | Behavior |
|----------|----------|
| Record only in JSON | Insert into local DB |
| Record only in local DB | Keep (not deleted) |
| Record in both, JSON newer | Update local DB |
| Record in both, local newer | Keep local version |

**UUIDs prevent ID collisions** - two developers can create work items offline without conflict.

---

## Troubleshooting

### Hook Not Running

**Check if executable:**
```bash
ls -l .git/hooks/pre-commit
# Should show: -rwxr-xr-x (x = executable)

# If not executable:
chmod +x .git/hooks/pre-commit
```

**Check if it exists:**
```bash
cat .git/hooks/pre-commit
# Should show the hook script
```

---

### Pre-Commit Fails

**Symptom:** `git commit` aborts with "Error: Failed to export data"

**Common causes:**
1. pm-server not running → Start it: `pm desktop` or `cargo run -p pm-server`
2. pm CLI not in PATH → Install it: see `install.sh`
3. Database corruption → Check `pm project list` works

**Bypass (emergency only):**
```bash
git commit --no-verify -m "Message"
```

**Warning:** This skips the export. Your commit won't include data changes. Fix the issue and re-commit properly.

---

### Post-Merge Import Fails

**Symptom:** Warning message after `git pull`

**Common causes:**
1. pm-server not running → Start it and run manually: `pm sync import --merge`
2. Invalid JSON in data.json → Check for merge conflicts in the file
3. Schema version mismatch → Update pm CLI to latest version

**Fix:**
```bash
# Start server if not running
pm desktop

# Import manually
pm sync import --merge

# Verify it worked
pm project list
```

---

## Security Considerations

### Hook Safety

- Hooks run locally - no remote code execution
- Templates are copy-ready (not auto-installed)
- User explicitly installs them (informed consent)
- All hooks check for prerequisites before running

### Git Hook Risks

**Hooks can be malicious.** Always review hook contents before installing:

```bash
# Review before installing
cat docs/hooks/pre-commit

# Then install
cp docs/hooks/pre-commit .git/hooks/
```

**Never blindly copy hooks from untrusted sources.**

---

## Future Enhancements

### Optional: `pm sync init` Command

Add a CLI command to install hooks automatically:

```bash
pm sync init

# Output:
# Installing git hooks...
# ✓ .git/hooks/pre-commit
# ✓ .git/hooks/post-merge
# ✓ .git/hooks/post-checkout
#
# Hooks installed! Your agile board will now auto-sync with git.
```

Implementation would:
1. Copy templates from embedded resources or `docs/hooks/`
2. Set executable permissions
3. Verify installation
4. Provide rollback: `pm sync init --uninstall`

---

## Alternatives to Hooks

If you prefer manual sync:

```bash
# Before commit
pm sync export --output .pm/data.json
git add .pm/data.json
git commit -m "Message"

# After pull
git pull
pm sync import --merge
```

Hooks just automate these manual steps.

---

## See Also

- `docs/session-plans/121.4-Session-Plan.md` - Sync implementation details
- `backend/pm-server/src/api/sync/` - Server-side export/import endpoints
- `backend/crates/pm-cli/src/sync_commands.rs` - CLI sync commands
- `.pm/.gitignore` - Git tracking rules for `.pm/` directory
