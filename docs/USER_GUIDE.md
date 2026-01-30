# User Guide

This guide describes how to use the current UI screens in the Blazor Agile Board client.

> Scope: This is based on the current UI pages and components in `frontend/ProjectManagement.Wasm/Pages`.

---

## Home

**Route:** `/`

What you’ll see:
- A welcome header
- A quick stats row (Projects, Active Items, Completed Today, Active Sprints)
- A list of recent projects

Actions:
- **Create Project**: opens the project dialog from the Home page
- **Select Project**: click a project row to open its detail view

---

## Project Detail

**Route:** `/project/{ProjectId}`

Tabs:
- **List**: list view of work items
- **Board**: Kanban board view
- **Sprints**: sprint planning view

Actions:
- **New Work Item** (top right)
- **New Sprint** (Sprints tab)
- **Start/Complete Sprint** (Sprints tab)

Notes:
- The page uses real-time updates from the WebSocket client.
- Connection state can disable actions when offline.

---

## Work Item Detail

**Route:** `/workitem/{WorkItemId}`

Sections:
- **Header**: title, type, status, priority, story points
- **Description**
- **Child Items** (if any)
- **Comments**
- **Details sidebar** (type, status, priority, sprint, timestamps, version)
- **Activity** (audit feed for the current work item)

Actions:
- **Edit** and **Delete** work item
- **Edit/Delete child items**

---

## Comments

Comments appear on the Work Item Detail page:
- Add new comments
- Edit existing comments
- Delete comments

The comment list is live‑updated via WebSocket events.

---

## Activity Feed

Activity shows recent changes to the current work item:
- Field changes
- Create/update/delete actions

The feed is loaded via WebSocket and updates in real time.
