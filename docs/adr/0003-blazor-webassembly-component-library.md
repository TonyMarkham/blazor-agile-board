# ADR-0003: Blazor WebAssembly Component Library

## Status
Accepted

## Context
The project management application needs to be deployable both as a standalone application and as a plugin integrated into an existing Blazor-based coaching SaaS platform. We need to choose a frontend technology and architecture that supports both deployment scenarios.

Options considered:
1. Blazor Server (server-side rendering with SignalR)
2. Blazor WebAssembly (client-side execution)
3. Blazor United (hybrid SSR + WASM)
4. Separate frontend technology (React, Vue, etc.)

## Decision
We will build the frontend as a Blazor WebAssembly Razor Class Library (RCL) using Radzen Blazor Components as the UI component library and design system.

Architecture:
- Core UI components in a reusable Razor Class Library
- Radzen Blazor Components for all UI elements (grids, forms, dialogs, etc.)
- Thin WASM host project for standalone deployment
- Plugin integration imports the RCL into host platform
- Data access abstracted via interfaces for flexibility

UI Technology:
- **Radzen.Blazor** NuGet package for professional component library
- Radzen theming system for consistent design language
- Built-in components: DataGrid, Scheduler, Dialog, Tree, etc.

## Consequences

### Positive
- **Plugin portability**: RCL can be imported directly into the coaching platform
- **Component reusability**: Same UI components work in both standalone and plugin modes
- **Client-side execution**: No additional server infrastructure needed for UI rendering
- **Offline capability**: Can work without constant server connection (useful for coaches on the go)
- **Technology alignment**: Matches the existing coaching platform stack
- **Code sharing**: Share models, validation logic, and utilities between client and server
- **Professional UI**: Radzen provides enterprise-grade components out of the box
- **Rich data grids**: RadzenDataGrid perfect for task lists, sprint boards, time tracking
- **Theming consistency**: Can match coaching platform's theme if it also uses Radzen
- **Productivity**: Pre-built complex components (scheduler, tree, upload) accelerate development
- **Active development**: Radzen is well-maintained with regular updates
- **No design work needed**: Components follow consistent design language

### Negative
- **Initial load time**: WASM bundle download can be slower than server-rendered content
- **Binary size**: .NET runtime + Radzen library must be downloaded (though cached after first load)
- **Browser requirements**: Requires modern browser with WASM support
- **Debugging complexity**: Client-side debugging is less straightforward than server-side
- **Library dependency**: Tied to Radzen's component API and update cycle
- **License consideration**: Radzen is free for commercial use, but customization may require understanding their component internals
- **Bundle size increase**: Radzen adds to the overall WASM bundle size

### Implementation Structure
```
/ProjectManagement.Core           - Shared models, interfaces, DTOs
/ProjectManagement.Components     - Razor Class Library (UI components)
/ProjectManagement.Wasm          - Standalone WASM host
```

### Integration Strategy
- **Standalone mode**: WASM host directly references the component library
- **Plugin mode**: Host platform imports component library and provides data access implementation
- **Data abstraction**: `IProjectManagementService` interface allows different backends
- **State management**: Use services/Fluxor to keep state separate from components

### Key Radzen Components for Project Management
- **RadzenDataGrid**: Task lists, issue tracking, sprint backlogs
- **RadzenScheduler**: Sprint timelines, milestone planning, resource calendars
- **RadzenTree**: Project hierarchy, task breakdown structure
- **RadzenDialog**: Task details, quick-create modals, confirmation dialogs
- **RadzenSplitter**: Resizable panels for task detail views
- **RadzenTabs**: Organize project views (backlog, board, timeline, reports)
- **RadzenChart**: Burndown charts, velocity tracking, time reports
- **RadzenBadge**: Task status indicators, priority markers
- **RadzenProgressBar**: Sprint progress, task completion visualization
