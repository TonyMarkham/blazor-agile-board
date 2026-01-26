# Drag-and-Drop Fix Session Prompt

## Context

This is the 5th session attempting to fix Kanban board drag-and-drop. The custom HTML5 drag-and-drop implementation is broken - ondragover events don't fire, even on empty columns. Previous attempts to fix it have all failed.

The user wants to replace the custom implementation with Radzen's drag-and-drop components. The project already uses Radzen Blazor for UI components.

---

## Problem Statement

The Kanban board in the Blazor WASM frontend has a drag-and-drop bug: **cards can be dragged but cannot be dropped into different columns**

### Expected Behavior
- User drags card from any column to any other column
- Card moves to new column
- Status updates on backend

---

## Current Implementation Details

### Files Involved
- `frontend/ProjectManagement.Components/WorkItems/KanbanBoard.razor` - Main board component
- `frontend/ProjectManagement.Components/WorkItems/KanbanColumn.razor` - Individual column component
- `frontend/ProjectManagement.Components/WorkItems/KanbanCard.razor` - Individual card component
- `frontend/ProjectManagement.Components/wwwroot/css/kanban.css` - Styles

### Current Implementation Uses
- Custom HTML5 drag-and-drop (`draggable="true"`, `@ondragstart`, `@ondragend`, `@ondragover`, `@ondrop`)
- Manual state management (`_draggedItem`, `_dragTargetColumn`, `_isCurrentlyDragging`)
- `ShouldRender()` override to block renders during drag
- Event callbacks between Card → Column → Board

---

## Radzen DropZone Documentation

**Official Radzen components for drag-and-drop:**

### Key Components
- `RadzenDropZoneContainer<TItem>` - Parent container managing all drop zones
- `RadzenDropZone<TItem>` - Individual drop zone (each column)
- `RadzenDropZoneItem<TItem>` - Automatically created for each item

### Key Properties/Callbacks

**RadzenDropZoneContainer:**
- `Data` - Collection of all items (List<WorkItemViewModel>)
- `ItemSelector` - Func that determines which items belong in which zone
  - Example: `(item, zone) => item.Status == zone.Value?.ToString()`
- `ItemRender` - Callback to customize item rendering and draggability
  - Type: `RadzenDropZoneItemRenderEventArgs<TItem>`
  - Properties: `args.Draggable`, `args.Visible`, `args.Attributes`
- `Drop` - Callback when item is dropped
  - Type: `RadzenDropZoneItemEventArgs<TItem>`
  - Properties: `args.Item`, `args.DropZone`, `args.DraggedItem`, etc.

**RadzenDropZone:**
- `Value` - Identifier for this zone (e.g., "backlog", "todo", "in_progress")
- `ChildContent` - Visual container (column header, empty state, etc.)
- `Template` - How to render each item in this zone

### CSS Classes (Automatic)
- `.rz-can-drop` - Applied when zone can accept the dragged item
- `.rz-no-drop` - Applied when zone cannot accept the dragged item

### Documentation Links
- Component demo: https://blazor.radzen.com/dropzone
- API docs: https://blazor.radzen.com/docs/api/Radzen.RadzenDropZoneItemEventArgs-1.html
- Blog tutorial: https://www.radzen.com/blog/blazor-drag-and-drop
- GitHub source: https://github.com/radzenhq/radzen-blazor/blob/master/Radzen.Blazor/RadzenDropZone.razor

---

## Testing Checklist (When Implementation is Complete)

1. **Build succeeds**
   ```bash
   just build-cs-components
   ```

2. **App runs**
   ```bash
   just dev
   ```

3. **Drag and drop works**
   - Drag card from "Backlog" to "To Do" - card moves
   - Drag card from "To Do" to "In Progress" - card moves
   - Drag card from "In Progress" to "Done" - card moves
   - Card status updates on backend (check console logs)
   - Card appears in new column immediately
   - Success notification appears

4. **Edge cases work**
   - Cannot drag cards when disconnected
   - Cannot drag cards that are pending sync (IsPendingSync = true)
   - Dragging to same column doesn't trigger unnecessary updates
   - Console logs show proper dragover events on columns

---

## Success Criteria

- User can drag cards between ANY columns
- Card visually moves to new column
- Backend status updates correctly
- No console errors
- Builds without errors
- Works in both dev mode and production build

---

## Note on Working Style

The user has been frustrated by multiple sessions where many changes were made before building, resulting in hard-to-debug compilation errors. They prefer incremental changes with builds after each step to catch errors early.
