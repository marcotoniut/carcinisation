# Carcinisation Glossary

Canonical vocabulary for agents working on this codebase.

## Acronyms

| Term | Meaning |
|------|---------|
| **ORS** | On-Rails Shooter — primary game mode. `crates/carcinisation_ors`. |
| **FPS** | First-Person Shooter — raycaster subgame. `crates/carcinisation_fps` + `_fps_core`. |
| **RON** | Rusty Object Notation — config and stage data format. |
| **PXI** | Pixel Indexed — compressed palette-indexed image format (0=transparent, 1-15=palette). |
| **DDA** | Digital Differential Analyzer — raycasting algorithm in `fps_core/raycast.rs`. |

## Crates

| Term | Meaning |
|------|---------|
| **carapace** | Internal pixel rendering engine (forked from `seldom_pixel`). |
| **cween** | Internal tween/animation library. |
| **carcinisation_core** | Shared utilities: despawn, volume, screen resolution. |
| **carcinisation_base** | Shared gameplay vocabulary: layers, directions, game state. |
| **carcinisation_net** | Network protocol: `ClientIntent`, `InputAck`, `NetPlayer`, tick system, prediction. |
| **carcinisation_fps_core** | Headless FPS simulation: movement, AI, raycasting, combat. Shared between SP and server. |
| **carcinisation_input** | Game Boy-style input abstraction via `leafwing-input-manager`. |
| **asset_pipeline** | Aseprite export pipeline: `.aseprite` → atlas packages. |

## Netcode

| Term | Meaning |
|------|---------|
| **ClientIntent** | Semantic player intent (movement, turn, fire, actions bitmask). Client→server, reliable ordered. |
| **InputAck** | Server acknowledgement with authoritative position/angle/snap state. Server→client. |
| **InputSequence** | Wrapping u32 input counter. Half-range comparison for ordering. |
| **TickCounter** | Server tick (wrapping u32, 30 Hz via `FixedUpdate`). |
| **PredictionHistory** | Ring buffer (max 60 entries) of predicted inputs for reconciliation replay. |
| **PlayerActions** | One-shot action bitmask: snap turns, weapon switch, melee. OR-accumulated per tick. |
| **NetPlayer / NetEnemy** | Replicated player/enemy components with position, angle, state. |
| **MovementSet / CombatSet / TickSet** | `FixedUpdate` ordering: Movement → Combat → Tick. |

## Game Design

| Term | Meaning |
|------|---------|
| **Stage** | Self-contained ORS level (`.sg.ron`). Contains spawns, steps, skybox, music. |
| **Depth Lane** | Discrete depth planes (0-9) creating pseudo-3D perspective in ORS mode. |
| **SnapTurn** | Instant turn animation: QuickTurn (180°), Left/Right (90°). Multi-tick at fixed angular speed. |
| **Hitscan** | Instant ray weapon (pistol). Flamethrower uses area burn ticks instead. |
| **BurnState** | Progressive fire damage state. Configurable tick rate and DPS. |
| **GroundFire** | Hazard spawned when enemy dies from burning. Contact damage with lifetime. |

## Enemies

| Term | Meaning |
|------|---------|
| **Mosquito** | Baseline fodder enemy. Simple sprite, contact damage. |
| **Mosquiton** | Composed multi-part enemy (wings + body). Ranged + melee. |
| **FpsEnemyKind** | FPS enemy types: `Basic`, `Mosquiton`. |

## Composed Sprites

| Term | Meaning |
|------|---------|
| **Composed Sprite** | Multi-part sprite built from semantic Aseprite layers, composed at runtime per frame. |
| **SpriteDirection** | 8-way direction enum. 5 physical (atlas-backed) + 3 virtual (mirrored). |
| **Directional Tag Prefix** | Tag convention: `{direction}_{action}` (e.g., `front_idle_stand`). |
