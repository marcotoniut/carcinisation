# Scene Editor

A standalone Bevy app for visually authoring stage (`.sg.ron`) and cutscene (`.cs.ron`) files.

## Running

```sh
make launch-editor
```

Or directly:

```sh
cargo run -p editor --features full_editor
```

## Loading a scene

Click **Select file** in the top-left bar to open a file picker. The editor loads `.sg.ron` (stages) and `.cs.ron` (cutscenes). The last opened file is remembered across sessions.

## Controls

### Camera navigation

| Input | Action |
|---|---|
| Two-finger scroll (trackpad) | Pan |
| Middle mouse drag | Pan |
| Pinch (trackpad) | Zoom at cursor |
| Mouse wheel | Zoom at cursor |
| Cmd/Ctrl + trackpad scroll | Zoom at cursor |
| Alt + mouse drag (vertical) | Zoom |

Pan and zoom scale with the current zoom level so they feel consistent at any magnification.

### Selection and movement

| Input | Action |
|---|---|
| Left click | Select entity (pixel-perfect hit test) |
| Left drag | Move selected entity |
| Right click drag | Snap selected entity to cursor |
| Delete / Backspace | Delete selected entity |
| Escape | Cancel placement mode |

### Input ownership

The editor uses a gesture-ownership model to prevent input conflicts between the viewport and UI panels:

- **Scroll/drag gestures are owned by whoever the pointer was over when the gesture started.** If you start scrolling over a menu, the menu keeps the scroll for the whole gesture -- even if the cursor drifts over the viewport.
- **Tool interactions (entity drag, placement) take priority over camera movement** while active.
- Ownership resets when all mouse buttons are released or after a brief scroll idle gap (~120ms).

This means scrolling in the inspector or timeline never accidentally pans the camera, and dragging an entity never accidentally moves the viewport.

## Spawn Palette

The **Spawn Palette** panel lists all placeable spawn types grouped by category:

- **Objects** -- benches, trees, signs
- **Destructibles** -- lamps, trashcans, crystals, mushrooms
- **Pickups** -- health packs
- **Enemies** -- mosquito, mosquiton, tardigrade, etc.

Click a type to enter placement mode. The next click on the canvas creates the spawn at that position with the selected depth. A depth selector is available at the top of the palette.

## UI Panels

| Panel | Location | Content |
|---|---|---|
| Path bar | Top-left | File path, Save button, Select file button |
| Stage Controls | Left | Layer visibility toggles, timeline elapsed |
| Spawn Palette | Near controls | Spawn creation, depth selector |
| Scene Inspector | Right | Selected entity components, or full scene data |
| World Inspector | Bottom-left | All ECS entities and resources (collapsed by default) |
| Timeline | Bottom-center | Elapsed time slider with step labels |

## Stage editing workflow

1. Open a `.sg.ron` file
2. Use the timeline slider to scrub through the stage -- spawns appear as the camera reaches each step
3. Two-finger scroll or middle-mouse drag to pan around the stage
4. Mouse wheel or Cmd+scroll to zoom in/out
5. Select and drag entities to reposition them
6. Use the spawn palette to add new entities
7. Delete unwanted spawns with Delete/Backspace
8. Click **Save** to write changes back to the `.sg.ron` file

Coordinate and depth feedback is shown below the cursor while dragging.

## File format

The editor reads and writes the same `.sg.ron` format the game uses at runtime. There is no separate editor format. Changes are saved with `ron::ser::to_string_pretty`.

## Architecture notes

- The editor is a separate binary from the game -- it does not run gameplay systems
- Scene entities are fully rebuilt whenever scene data changes (full rebuild on change model)
- Camera navigation uses a custom input system with gesture-ownership tracking (no external camera crate)
- BRP (Bevy Remote Protocol) is enabled in debug builds for remote inspection
