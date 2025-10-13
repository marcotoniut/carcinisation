# Editor v2 - Project Status

## Overview

A simple browser-based map editor for Carcinisation. The foundation is complete—basic file loading/saving works. Visual editing features are next.

## ✅ Completed

### Core Infrastructure
- **Vite + React + TypeScript** - Modern dev stack with HMR
- **Biome** - Linting and formatting configured
- **pnpm** - Package manager (proto integration)
- **Zustand** - State management for file content and dirty tracking
- **File System API** - Browser-native file loading/saving with fallbacks

### UI Components (Scaffolded)
- **Toolbar** - Load/Save/Undo/Redo buttons (Load & Save working)
- **Viewport** - Canvas element ready for rendering
- **Timeline** - Screen position slider component
- **Scenes Panel** - For scene/step navigation
- **Palette Panel** - For monster/object library
- **Hierarchy Panel** - For entity tree view
- **Inspector Panel** - For property editing
- **Console Panel** - For error messages

### Build & Development
- `make dev-editor-v2` - Start dev server
- `make build-editor-v2` - Production build
- `make ci-editor-v2` - Linting and tests
- `.gitignore` configured

## 🚧 In Progress / TODO

### Phase 1: Data Parsing (P0)
- [ ] Parse RON file content to structured data
- [ ] Type definitions for StageData/CutsceneData
- [ ] JSON representation of map entities

### Phase 2: Visual Rendering (P0)
- [ ] Canvas rendering setup
- [ ] Draw entities (spawns, objects, enemies)
- [ ] Grid and coordinate system
- [ ] Pan and zoom controls
- [ ] Layer visibility

### Phase 3: Editing (P0)
- [ ] Click to select entities
- [ ] Drag to move entities
- [ ] Inspector forms for properties
- [ ] Add/delete entities
- [ ] Save edited data back to RON format

### Phase 4: Timeline (P1)
- [ ] Screen position slider integration
- [ ] Show/hide spawns based on elapsed time
- [ ] Visual spawn markers on timeline
- [ ] Per-scene timeline visualization

### Phase 5: UX Features (P1)
- [ ] Palette drag-drop for placing entities
- [ ] Hierarchy tree view
- [ ] Undo/redo command stack
- [ ] Validation before save
- [ ] Console error messages

### Phase 6: Polish (P2)
- [ ] Keyboard shortcuts (Ctrl+S, Ctrl+Z, Delete, etc.)
- [ ] Multi-select entities
- [ ] Copy/paste entities
- [ ] Asset preview/thumbnails
- [ ] Animation preview

## Progress Metrics

| Category | Completed | Total | % |
|----------|-----------|-------|---|
| Infrastructure | 6/6 | 6 | 100% |
| UI Scaffold | 8/8 | 8 | 100% |
| File Operations | 2/2 | 2 | 100% |
| **Phase 1 (Parsing)** | 0/3 | 3 | 0% |
| **Phase 2 (Rendering)** | 0/5 | 5 | 0% |
| **Phase 3 (Editing)** | 0/5 | 5 | 0% |
| **Phase 4 (Timeline)** | 0/4 | 4 | 0% |
| **Phase 5 (UX)** | 0/5 | 5 | 0% |
| **Phase 6 (Polish)** | 0/5 | 5 | 0% |
| **Total** | **16/43** | **43** | **37%** |

## Current Status

**What Works**:
- Dev server runs
- Load button opens file picker
- Save button downloads modified file
- Dirty tracking shows `*` when file is modified
- All UI panels are visible and styled

**What's Next**:
1. Parse `.ron` file content into structured data
2. Render entities in the Viewport canvas
3. Make entities clickable and draggable
4. Add property editing via Inspector

## How to Test

```bash
cd tools/editor-v2
pnpm dev
```

Then:
1. Click "Load"
2. Select `assets/stages/debug.sg.ron`
3. File content loads (currently just stored as text)
4. Click "Save" to download

## Migration Strategy

This editor runs **alongside** the existing Bevy editor (`tools/editor`):

1. ✅ **Current**: Both editors work independently
2. 🚧 **Next**: Add visual editing to editor-v2
3. ⏳ **Future**: Achieve feature parity
4. ⏳ **Long-term**: editor-v2 becomes primary tool

## Notes

- No WASM - pure JavaScript/TypeScript
- No backend - runs entirely in browser
- File System Access API for Chrome/Edge (re-save to same file)
- Download fallback for Firefox/Safari
- Simple architecture, easy to extend
