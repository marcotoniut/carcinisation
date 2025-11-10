# Stage Editor - Feature Parity & Project Status

A browser-based stage editor for Carcinisation, designed to run alongside the existing Bevy editor with improved UX and web-native features.

---

## Project Overview

**Current State**: Foundation complete with full file I/O and type generation. Ready for visual rendering phase.

**Architecture**:
- **Stack**: Vite + React 19 + TypeScript + Zustand
- **Type Generation**: ts-rs generates TypeScript types from Rust at compile-time
- **RON Bridge**: Rust binary (`ron_bridge`) converts RON ↔ JSON via Vite middleware
- **File I/O**: File System Access API (Chrome/Edge) with download fallback

---

## ✅ Completed Features

### Core Infrastructure (100%)

- [x] **Vite + React + TypeScript** - Modern dev stack with HMR
- [x] **Biome** - Linting and formatting configured
- [x] **pnpm** - Package manager with proto integration
- [x] **Zustand** - State management for file content and dirty tracking
- [x] **File System Access API** - Browser-native file loading/saving with fallbacks
- [x] **Build targets** - `make dev-stage-editor`, `make build-stage-editor`, `make ci-stage-editor`

### Type System (100%)

- [x] **ts-rs integration** - Compile-time TypeScript type generation from Rust
- [x] **32 TypeScript types** - Full StageData type coverage
- [x] **Type validation** - Basic structural validation for loaded data
- [x] **Import fixing** - Automatic import injection for generated types

### File Operations (100%)

- [x] **Load .ron files** - Browser file picker (File System Access API)
- [x] **Parse RON → JSON** - via `ron_bridge` binary + Vite middleware
- [x] **Validate StageData** - Basic structural checks on load
- [x] **Save JSON → RON** - Proper RON formatting with feature flags
- [x] **File handle preservation** - Direct save to original file (Chrome/Edge)
- [x] **Download fallback** - For browsers without File System Access API
- [x] **Console logging** - Success/error messages

### UI Scaffold (100%)

- [x] **Toolbar** - Load/Save buttons (fully functional)
- [x] **Viewport** - Canvas placeholder for rendering
- [x] **Timeline** - Screen position slider component
- [x] **Scenes Panel** - Scaffold for scene/step navigation
- [x] **Palette Panel** - Scaffold for entity library
- [x] **Hierarchy Panel** - Scaffold for entity tree view
- [x] **Inspector Panel** - Scaffold for property editing
- [x] **Console Panel** - Status messages and errors

---

## 🚧 In Progress / TODO

### Phase 2: Visual Rendering (P0) - 0%

- [ ] **Canvas library** - Install PixiJS or Konva.js
- [ ] **Background rendering** - Draw stage background image
- [ ] **Spawn rendering** - Draw entities (Objects, Destructibles, Enemies, Pickups)
- [ ] **Grid overlay** - Coordinate system and grid lines
- [ ] **Camera controls** - Pan (drag) and zoom (mouse wheel)
- [ ] **Layer system** - Depth-based layering (Nine → Zero)
- [ ] **Layer visibility** - Toggle depth layers on/off
- [ ] **Asset loading** - Fetch sprites from game assets

### Phase 3: Interactive Editing (P0) - 0%

- [ ] **Entity selection** - Click to select spawns
- [ ] **Entity dragging** - Move entities with mouse
- [ ] **Inspector integration** - Edit selected entity properties
- [ ] **Add entities** - Palette drag-drop or context menu
- [ ] **Delete entities** - Delete key or context menu
- [ ] **Property forms** - Type-specific property editors
- [ ] **Real-time updates** - Reflect changes immediately in viewport
- [ ] **Coordinate snapping** - Optional grid snapping

### Phase 4: Timeline (P1) - 0%

- [ ] **Elapsed time integration** - Wire up timeline slider
- [ ] **Spawn visibility** - Show/hide based on elapsed time
- [ ] **Timeline markers** - Visual spawn events
- [ ] **Per-step timeline** - Separate timeline for each stage step
- [ ] **Scrubbing preview** - Preview spawns as slider moves

### Phase 5: UX Enhancements (P1) - 0%

- [ ] **Undo/redo** - Command pattern with history stack
- [ ] **Keyboard shortcuts** - Ctrl+S (save), Ctrl+Z (undo), Delete, etc.
- [ ] **Recent files** - Track recently opened files
- [ ] **Validation** - Check data integrity before save
- [ ] **Error handling** - User-friendly error messages
- [ ] **Dirty tracking** - Visual indicator for unsaved changes

### Phase 6: Advanced Features (P2) - 0%

- [ ] **Multi-select** - Select multiple entities
- [ ] **Copy/paste** - Duplicate entities
- [ ] **Transform gizmos** - Visual transform handles
- [ ] **Asset previews** - Thumbnails in palette
- [ ] **Animation preview** - Show sprite animations
- [ ] **Search/filter** - Find entities by type/name

---

## Bevy Editor Parity Matrix

Comparison with the existing Bevy/Rust editor (`tools/editor`):

| Feature                    | Bevy Editor  | Stage Editor  | Status       |
| -------------------------- | ------------ | ------------- | ------------ |
| **File Operations**        |
| Load .ron files            | ✅ Native    | ✅ Browser    | **Complete** |
| Save .ron files            | ✅ Native    | ✅ Browser    | **Complete** |
| Parse RON → data           | ✅ Rust      | ✅ ron_bridge | **Complete** |
| Serialize data → RON       | ✅ Rust      | ✅ ron_bridge | **Complete** |
| Recent files               | ✅           | ⏳            | TODO         |
| **Data Types**             |
| StageData types            | ✅           | ✅ ts-rs      | **Complete** |
| Type validation            | ✅ Rust      | ✅ Basic      | **Complete** |
| **Visual Rendering**       |
| Viewport canvas            | ✅ Bevy      | ⏳ PixiJS     | TODO         |
| Draw background            | ✅           | ⏳            | TODO         |
| Draw spawns                | ✅           | ⏳            | TODO         |
| Layer visibility toggles   | ✅           | ⏳            | TODO         |
| Grid overlay               | ⏳           | ⏳            | TODO         |
| Pan & zoom controls        | ✅ Alt+drag  | ⏳ Drag+wheel | TODO         |
| **Editing**                |
| Select entities            | ✅           | ⏳            | TODO         |
| Move entities              | ✅ Inspector | ⏳ Drag       | TODO         |
| Edit properties            | ✅ Inspector | ⏳ Forms      | TODO         |
| Add/delete entities        | ⏳           | ⏳            | TODO         |
| **Timeline**               |
| Elapsed time slider        | ✅           | ⏳            | TODO         |
| Spawn visualization        | ⚠️ Basic     | ⏳            | TODO         |
| Per-step timeline          | ⏳           | ⏳            | TODO         |
| **UI Panels**              |
| Scenes panel               | ⏳           | ✅ Scaffold   | Partial      |
| Palette panel              | ⏳           | ✅ Scaffold   | Partial      |
| Hierarchy panel            | ⏳           | ✅ Scaffold   | Partial      |
| Inspector panel            | ✅           | ✅ Scaffold   | Partial      |
| Console panel              | ⏳           | ✅ Working    | **Complete** |
| **UX Features**            |
| Undo/redo                  | ⏳           | ⏳            | TODO         |
| Transform gizmos           | ⏳           | ⏳            | TODO         |
| Multi-select               | ⏳           | ⏳            | TODO         |
| Keyboard shortcuts         | ⏳           | ⏳            | TODO         |
| Validation before save     | ⏳           | ⏳            | TODO         |

**Legend**: ✅ Complete | ⚠️ Partial | ⏳ Not started

---

## Progress Metrics

| Category                     | Completed | Total | %       |
| ---------------------------- | --------- | ----- | ------- |
| **Core Infrastructure**      | 6/6       | 6     | 100%    |
| **Type System**              | 4/4       | 4     | 100%    |
| **File Operations**          | 7/7       | 7     | 100%    |
| **UI Scaffold**              | 8/8       | 8     | 100%    |
| **Phase 2: Rendering**       | 0/8       | 8     | 0%      |
| **Phase 3: Editing**         | 0/8       | 8     | 0%      |
| **Phase 4: Timeline**        | 0/5       | 5     | 0%      |
| **Phase 5: UX**              | 0/6       | 6     | 0%      |
| **Phase 6: Polish**          | 0/6       | 6     | 0%      |
| **Overall Progress**         | **25/58** | **58**| **43%** |

---

## Stage Editor Advantages

The web-based editor addresses limitations of the Bevy editor:

1. **Web-based** - Runs in any modern browser, no native install required
2. **Type safety** - ts-rs ensures TypeScript types match Rust types at compile-time
3. **Better UX** - Modern web UI patterns (drag-drop, undo/redo, etc.)
4. **Palette panel** - Visual library of all placeable entities
5. **Hierarchy panel** - Tree view of all spawns with search/filter
6. **Timeline visualization** - Clearer spawn event timeline
7. **Validation** - Check data integrity before save
8. **File System Access API** - Direct save to original file (Chrome/Edge)

---

## How to Test

```bash
cd tools/stage-editor
pnpm dev
```

Then:
1. Click "Load"
2. Select `assets/stages/debug.sg.ron`
3. File parses and loads into structured StageData
4. Modify data (future: via Inspector/viewport)
5. Click "Save" to write back to RON format

**What currently works**:
- Load `.sg.ron` files from filesystem
- Parse RON → JSON → TypeScript StageData
- Display console messages
- Save StageData → RON with proper formatting
- File handle preservation (Chrome/Edge re-saves to same file)

**Next milestone**: Render entities visually in the viewport canvas using PixiJS or Konva.js

---

## Migration Strategy

Both editors will **coexist indefinitely**:

1. ✅ **Phase 1 (Current)**: Foundation complete, file I/O works
2. 🚧 **Phase 2 (Next)**: Add visual rendering with PixiJS/Konva
3. ⏳ **Phase 3**: Interactive editing (drag entities, edit properties)
4. ⏳ **Phase 4**: Timeline integration
5. ⏳ **Phase 5**: UX polish (undo/redo, keyboard shortcuts)
6. ⏳ **Phase 6**: Stage editor becomes primary tool

No forced migration. Developers can use whichever editor suits their workflow.

---

## Technical Notes

### Type Generation Flow

```
Rust types (stage/data.rs)
  ↓ [cargo build --features derive-ts]
ts-rs generates types → apps/carcinisation/bindings/
  ↓ [gen_types.rs]
Copy to → tools/stage-editor/src/types/generated/
  ↓ [import fix pass]
✅ 32 TypeScript types ready
```

### RON Conversion Flow

```
Load: .ron file → ron_bridge (RON → JSON) → StageData → Zustand store
Save: StageData → ron_bridge (JSON → RON) → File System Access API → .ron file
```

### RON Bridge

The `ron_bridge` binary (`apps/carcinisation/src/bin/ron_bridge.rs`):
- Accepts stdin: `{"mode":"ron-to-json"|"json-to-ron", "payload":"..."}`
- Uses actual `StageData` type (not generic JSON)
- Outputs pretty-printed RON with feature flags
- Called via Vite middleware during development
- Runs via `cargo run` to avoid dynamic library issues

### Build Commands

- `make gen-types` - Regenerate TypeScript types from Rust
- `make dev-stage-editor` - Start dev server (port 5173)
- `make build-stage-editor` - Production build
- `make ci-stage-editor` - Lint and typecheck
