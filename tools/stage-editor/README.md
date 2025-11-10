# Stage Editor

A simple browser-based map editor for Carcinisation. Load, edit, and save `.ron` stage and cutscene files directly in your browser.

## Quick Start

```bash
cd tools/stage-editor
pnpm install
pnpm dev  # Runs Vite + cargo watch in parallel (generates types on startup and Rust changes)
```

Opens at http://localhost:5173

Or from the repo root:

```bash
make dev-stage-editor
```

## Current Features

- ✅ Load .ron files via file picker
- ✅ Save files (download or File System Access API)
- ✅ Dirty tracking (shows \* when modified)
- ✅ Modern UI with multiple panels
- ✅ Fast development with Vite HMR

## UI Layout

```
┌──────────────────────────────────────────────────────┐
│ Toolbar: Load | Save | Undo | Redo                   │
├──────────┬──────────────────────────┬────────────────┤
│ Scenes   │                          │ Hierarchy      │
│ -------- │                          │ ----------     │
│          │       Viewport           │                │
│ Palette  │       (Main Canvas)      │ Inspector      │
│ -------- │                          │ ----------     │
│          ├──────────────────────────┤                │
│          │ Timeline (Slider)        │                │
├──────────┴──────────────────────────┴────────────────┤
│ Console (Errors/Messages)                            │
└──────────────────────────────────────────────────────┘
```

## Architecture

**Simple & Local**

- Pure client-side React app (no backend)
- No WASM, no complexity
- File System Access API (Chrome/Edge) or download fallback (Firefox/Safari)
- Zustand for state management

**Key Files**

- `src/state/store.ts` - Global state (file content, dirty flag)
- `src/utils/fileSystem.ts` - Load/save helpers
- `src/components/Toolbar/` - Load/Save buttons
- `src/components/Viewport/` - Main editing canvas
- `src/components/Timeline/` - Screen position slider

## Commands

```bash
pnpm dev          # Start dev server
pnpm build        # Production build
pnpm lint         # Run linter
pnpm lint:fix     # Fix linting issues
pnpm format       # Format code
pnpm test         # Run tests
```

## Roadmap

**Current Status**: Foundation complete, basic load/save working

**Next Steps**:

1. Parse RON → structured data (JSON representation)
2. Render entities in Viewport canvas
3. Make entities interactive (select, move, edit properties)
4. Inspector forms for entity properties
5. Timeline scrubbing (show/hide spawns over time)
6. Palette drag-drop for placing entities
7. Hierarchy tree view
8. Undo/redo implementation

## Browser Support

| Browser         | File Loading   | File Saving             |
| --------------- | -------------- | ----------------------- |
| Chrome/Edge 86+ | ✅ File picker | ✅ Re-save to same file |
| Firefox         | ✅ File picker | ⚠️ Download only        |
| Safari 15.2+    | ✅ File picker | ⚠️ Download only        |

## Development

The editor is designed to run alongside the existing Bevy editor (`tools/editor`). Both editors can coexist—this is an incremental migration, not a replacement.

**Workflow**:

1. Make changes to React components
2. Hot reload updates instantly
3. Run `pnpm lint:fix` to format
4. Test by loading an actual .ron file from `assets/stages/` or open another `.ron` file from disk

## Type Generation

TypeScript types and Zod schemas are auto-generated from Rust types:

- `src/types/generated/*.ts` - TypeScript definitions (via serde reflection + serde_generate)
- `src/types/schemas/*.zod.ts` - Zod validation schemas (via ts-to-zod)
- Auto-regenerates when running `pnpm dev` (via cargo watch)
- Manual regeneration: `make gen-types` from repo root

Note: Complex discriminated unions fail ts-to-zod - hand-write these in `src/schemas/manual/` as needed.

## Documentation

- [PARITY.md](./PARITY.md) - Feature comparison with Bevy editor
- [PROJECT_STATUS.md](./PROJECT_STATUS.md) - Current implementation status

## License

Same as parent repository (Carcinisation).
