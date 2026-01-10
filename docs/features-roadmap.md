# Features Roadmap

This document outlines the features for the Project Management application, organized by release version.

## Version 1.0 - Core Features

### Essential Entities
- [x] **Projects** - Top-level containers for work
- [x] **Epics** - Large bodies of work that span multiple sprints
- [x] **Stories** - User-facing features or requirements
- [x] **Tasks** - Individual work items
- [x] **Sprints** - Time-boxed iterations for planning and execution
- [x] **Swim Lanes** - Workflow states (Backlog, In Progress, In Review, Done, Custom)
- [x] **Comments** - Discussion threads on work items
- [x] **Time Tracking** - Log time spent on tasks
- [x] **Dependencies** - 2-way blocking relationships between tasks

### Essential Features
- [x] **Hierarchy Management** - Parent/child relationships (Project → Epic → Story → Task)
- [x] **Assignment** - Delegate work items to users
- [x] **Activity Log** - Audit trail of all changes (critical for LLM context)
- [x] **Basic Search** - Find tasks by title, description, assignee
- [x] **WebSocket Real-time Updates** - Live collaboration with Protobuf messages

### Core UI Views
- [x] **Project Dashboard** - Overview of project health and progress
- [x] **Backlog View** - Prioritized list of work items
- [x] **Sprint Board** - Kanban-style board with swim lanes
- [x] **Task Detail View** - Full task information with comments and time logs
- [x] **Sprint Planning** - Drag-and-drop task assignment to sprints

### LLM Integration (v1)
- [x] **Read-only Context API** - Endpoint providing full project context for LLM analysis
- [x] **Direct SQLite Access** - LLMs can read the database file directly
- [x] **Activity History** - Complete audit log for understanding project evolution

---

## Version 1.1 - Enhanced Workflow

### Additional Entities
- [ ] **Labels/Tags** - Flexible categorization (bug, feature, tech-debt, etc.)
- [ ] **Priorities** - Explicit priority levels (P0-P3, Critical/High/Medium/Low)
- [ ] **Estimates** - Story points or time estimates (separate from actual time)
- [ ] **Watchers** - Users following items without being assigned

### Enhanced Features
- [ ] **Custom Swim Lanes** - User-defined workflow states beyond defaults
- [ ] **Blocked Status** - Explicit "blocked" state with blocking reason
- [ ] **Bulk Operations** - Multi-select and bulk update tasks
- [ ] **Advanced Filters** - Complex queries across multiple fields
- [ ] **Export** - Export projects/sprints to CSV, JSON, or PDF

### UI Enhancements
- [ ] **Timeline View** - Gantt-chart style visualization
- [ ] **Burndown Charts** - Sprint and release burndown tracking
- [ ] **Velocity Tracking** - Team velocity over time
- [ ] **My Work View** - Personal dashboard of assigned tasks

---

## Version 1.2 - Collaboration & Extensibility

### Additional Entities
- [ ] **Attachments/Files** - Link documents, images, designs to tasks
- [ ] **Milestones** - Key dates/deliverables spanning multiple sprints
- [ ] **Custom Fields** - Extensible metadata (dropdown, text, number, date)
- [ ] **Saved Views/Filters** - Save custom queries and layouts
- [ ] **Task Templates** - Pre-configured task structures for common workflows

### Collaboration Features
- [ ] **Notifications System** - Configurable notifications for changes
- [ ] **@Mentions** - Tag users in comments
- [ ] **Presence Indicators** - See who's viewing/editing what
- [ ] **Typing Indicators** - Real-time typing in comments

### UI Enhancements
- [ ] **Customizable Dashboards** - Drag-and-drop widgets
- [ ] **Dark Mode** - Theme switching
- [ ] **Keyboard Shortcuts** - Power user productivity features

---

## Version 2.0 - Advanced Features

### Structure & Organization
- [ ] **Workspaces/Portfolios** - Group projects hierarchically
- [ ] **Versions/Releases** - Track release planning and version targeting
- [ ] **Unlimited Task Nesting** - Sub-tasks at any depth
- [ ] **Cross-Project Dependencies** - Dependencies spanning multiple projects

### Advanced LLM Integration
- [ ] **Embeddings Search** - Semantic search via vector embeddings
- [ ] **AI Task Creation** - LLM can create and update tasks via natural language
- [ ] **Sprint Planning Assistant** - LLM helps estimate, prioritize, and organize
- [ ] **AI Suggestions Log** - Track and learn from LLM suggestions

### Reporting & Analytics
- [ ] **Custom Reports** - Build custom analytical views
- [ ] **Time Reports** - Detailed time tracking analysis
- [ ] **Resource Allocation** - Team capacity and utilization
- [ ] **Forecasting** - Predictive completion dates based on velocity

### Integrations
- [ ] **Git Integration** - Link commits/PRs to tasks
- [ ] **Calendar Sync** - Sync deadlines to external calendars
- [ ] **Webhook API** - External system integrations
- [ ] **Import from Jira/GitHub** - Migration tools

---

## Design Principles for All Versions

1. **LLM-First Design** - Every feature considers how LLMs will interact with the data
2. **Plugin Architecture** - All features must work both standalone and as SaaS plugin
3. **Audit Everything** - Complete change history for AI context and compliance
4. **Performance** - Real-time updates must be instant, SQLite queries optimized
5. **Extensibility** - Design for future custom fields and integrations
6. **Progressive Enhancement** - Core features work without WebSocket, enhanced with it

---

## Migration Strategy Between Versions

Each version will include:
- Database migrations (SQLx) to add new tables/columns
- Protobuf message updates (maintaining backward compatibility via field numbers)
- Feature flags to enable/disable new features during rollout
- Data migration scripts for any structural changes
