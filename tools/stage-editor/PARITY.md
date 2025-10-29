# Stage Editor - Feature Parity

Comparison between the existing Bevy/Rust editor (`tools/editor`) and the new web-based stage-editor (`tools/stage-editor`).

## Current Bevy Editor Features

### File I/O

- Loads/saves `.sg.ron` (stages) and `.cs.ron` (cutscenes)
- Recent file tracking
- Native file picker (rfd crate)

### Data Types

- **StageData**: name, background, music, skybox, spawns, steps
- **CutsceneData**: name, acts (images, animations, music)
- **Spawns**: Object, Destructible, Pickup, Enemy (positions, depths, behaviors)
- **Steps**: Movement, Stop, Cinematic
- **Depth Layers**: Nine through Zero (z-ordering)

### UI

- Stage Controls: elapsed time slider (0-999s), layer visibility toggles
- Inspector: bevy-inspector-egui for editing properties
- Canvas: Bevy rendering with visual previews
- Mouse controls: Alt+drag to pan, mouse wheel zoom

### Limitations

- Desktop only (no web)
- No palette for drag-drop
- No hierarchy tree view
- No undo/redo
- No visual transform gizmos
- Limited timeline visualization

## Stage Editor Parity Matrix

| Feature             | Bevy Editor  | Stage Editor | Status      |
| ------------------- | ------------ | ------------ | ----------- |
| **File Operations** |
| Load .ron files     | ✅ Native    | ✅ Browser   | Done        |
| Save .ron files     | ✅ Native    | ✅ Download  | Done        |
| Recent files        | ✅           | ⏳           | TODO        |
| **Editing**         |
| View map data       | ✅           | ⚠️ Text only | In Progress |
| Visual viewport     | ✅ Bevy      | ⏳ Canvas    | TODO        |
| Edit properties     | ✅ Inspector | ⏳           | TODO        |
| Move entities       | ✅ Inspector | ⏳ Drag      | TODO        |
| Add/delete          | ⏳           | ⏳           | TODO        |
| **Timeline**        |
| Elapsed time slider | ✅           | ✅ Scaffold  | TODO        |
| Spawn visualization | ⚠️ Basic     | ⏳ Enhanced  | TODO        |
| Per-scene timeline  | ⏳           | ⏳           | TODO        |
| **UI Panels**       |
| Scenes panel        | ⏳           | ✅ Scaffold  | TODO        |
| Palette             | ⏳           | ✅ Scaffold  | TODO        |
| Hierarchy           | ⏳           | ✅ Scaffold  | TODO        |
| Inspector           | ✅           | ✅ Scaffold  | TODO        |
| Layer visibility    | ✅           | ⏳           | TODO        |
| **UX**              |
| Undo/redo           | ⏳           | ⏳           | TODO        |
| Validation          | ⏳           | ⏳           | TODO        |
| Transform gizmos    | ⏳           | ⏳           | TODO        |

**Legend**: ✅ Complete | ⚠️ Partial | ⏳ Not started

## Improvement Opportunities

Stage Editor aims to address Bevy editor limitations:

1. **Web-based** - Runs in browser, no native install
2. **Palette** - Visual library of monsters/objects
3. **Hierarchy** - Tree view of all entities
4. **Timeline** - Better spawn event visualization
5. **Undo/Redo** - Command stack for all operations
6. **Validation** - Check data before save

## Migration Strategy

1. **Phase 1** (Current): Basic foundation, load/save works
2. **Phase 2**: Add visual rendering and editing
3. **Phase 3**: Achieve core feature parity
4. **Phase 4**: Add improvements (palette, hierarchy, undo/redo)
5. **Phase 5**: Stage Editor becomes primary, Bevy editor optional

Both editors will coexist indefinitely—no forced migration.
