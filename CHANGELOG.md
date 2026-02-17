# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-02-17

### Added
- Epic detail page now shows tasks nested under each child story, grouped with a left-border accent and subtle background for clear visual hierarchy
- Each comment collapses to 100px by default with a fade and "Show more" toggle; page scroll handles expanded comments with no internal scrolling
- Kanban board search box filters cards by key, title, or description
- Kanban board sort by key (ascending or descending) in addition to position

### Fixed
- Work item detail page now scrolls vertically; breadcrumbs and page header remain fixed
- Work item detail scroll region uses flexbox so variable-height titles don't break the layout
- Sprint date range display no longer shows locale-dependent period (e.g. `Jan.` vs `Jan`) by pinning to InvariantCulture
- Comment editor textarea now fills available horizontal space

## [0.1.1] - 2026-02-16

### Added
- Display server port in connection status widget (top-right header)
- Display repo root directory path in header bar
- Installation instructions in README for Windows, Linux, and macOS

### Fixed
- WebSocket client now uses the actual server port instead of hardcoded `ws://localhost:8000/ws`; fixes connection failure when server binds to a non-default port (e.g., WSL running alongside Windows)
- Remove duplicate dashboard icon in header

### Changed
- Allow multiple Tauri desktop instances to run simultaneously (one per repo); removed single-instance plugin
- Improve header contrast: white text and bright status colors against purple background
- Move connection status CSS to scoped component stylesheet

## [0.1.0] - 2026-02-16

Initial release with full agile board functionality and cross-platform desktop app.

### Added
- **Work Item Management**: Hierarchical work items (Project > Epic > Story > Task) with drag-and-drop Kanban board
- **Sprint Management**: Create, start, and complete sprints with velocity tracking
- **Comments**: Threaded comment system with markdown support on work items
- **Time Tracking**: Running timers and manual time entry per work item
- **Dependencies**: Task dependency tracking with cycle detection
- **Activity Log**: Full audit trail for all changes
- **Real-time Collaboration**: WebSocket + Protocol Buffers for live updates across clients
- **Desktop App**: Tauri-based desktop app for Windows, macOS, and Linux (including Raspberry Pi ARM64)
- **PM CLI**: Command-line interface for project management operations (`pm` command)
- **Data Sync**: Git-based data sync with `pm sync export/import` and git hooks
- **Markdown Support**: Rich text editing for work item descriptions and comments
- **Breadcrumb Navigation**: Hierarchical breadcrumb trail on work item detail pages
- **Work Item Keys**: Human-readable keys (e.g., `PROJ-42`) for work items
- **Ancestor/Descendant Tracking**: Query work items by hierarchy relationships
- **Configurable Validation**: Validation limits (title length, description size, etc.) configurable via pm-config
- **Install Scripts**: One-liner install for Linux/macOS (`install.sh`) and Windows (`install.ps1`)
- **LLM-Friendly Database**: Self-documenting schema with `pm_llm_context` table

### Architecture
- Blazor WebAssembly frontend with Radzen UI components
- Rust + Axum backend with per-tenant SQLite databases
- WebSocket-first communication protocol
- Tauri desktop shell with managed server lifecycle
- Cargo workspace with centralized dependency management

[Unreleased]: https://github.com/TonyMarkham/blazor-agile-board/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/TonyMarkham/blazor-agile-board/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/TonyMarkham/blazor-agile-board/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/TonyMarkham/blazor-agile-board/releases/tag/v0.1.0
