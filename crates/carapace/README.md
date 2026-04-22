# carapace

Pixel rendering engine for limited-palette games built on [Bevy](https://bevyengine.org/).
Forked from [Seldom-SE/seldom_pixel](https://github.com/Seldom-SE/seldom_pixel) and adapted
for the carcinisation project.

## Features

- Palette-indexed sprite rendering with CPU compositing
- Sprite atlases (`.px_atlas.ron`) with region-based sprite sheets
- Composite sprites for multi-part characters with per-part animation
- Layer-based rendering with filter effects (per-entity or per-layer)
- Tilemaps with per-tile filters
- Typeface-based text rendering
- Particle system (`particle` feature)
- Line drawing (`line` feature)
- Pixel-perfect picking
- Camera and UI system
- GPU palette pass (`gpu_palette` feature, experimental)
- BRP integration for tooling (`brp_extras` feature)
- Tracing instrumentation (`profiling_spans` feature)

## Usage

This crate is vendored as a workspace member. Add it as a path dependency:

```toml
[dependencies]
carapace = { path = "../carapace", features = ["line", "reflect"] }
```

Then add `CxPlugin` to your app:

```rust
use carapace::prelude::*;

app.add_plugins(CxPlugin::<Layer>::new(UVec2::new(320, 180), "palette/palette.palette.png"));
```

See the `examples/` directory for usage patterns.

## Bevy compatibility

| Bevy | carapace |
| ---- | -------- |
| 0.18 | current  |

## License

Dual-licensed under MIT and Apache 2.0. See `LICENSE-MIT` and `LICENSE-APACHE`.
