# Carcinisation — Game Design Document

## Identity

- **Genre**: Stage-based retro action shooter
- **Display**: 160x144 px (Game Boy form factor), 4-colour indexed palette
- **Engine**: Bevy 0.18 + carapace (custom pixel rendering)
- **Perspective**: Discrete depth lanes (1-9) — pseudo-3D "into the screen"
- **Version**: 0.2.0

## Core Pillars

1. **Depth-lane perspective** — enemies occupy discrete depth planes; the player attacks across lanes, creating multi-plane pressure
2. **Dual-attack combat** — melee pincer + ranged guns serve distinct tactical roles (close control vs. depth reach)
3. **Performance-driven pressure** 💡 — efficient play feeds rank, which escalates encounter intensity (see [Gameplay Systems](gameplay.md#scoring-))
4. **Authored stage scripting** — encounters are choreographed via RON data files, not procedural
5. **Pixel-perfect rendering** — palette-indexed visuals, per-depth sprite variants, composed multi-part animations

## Document Index

| Document | Scope |
|----------|-------|
| [Gameplay Systems](gameplay.md) | Movement, depth, combat, scoring, enemies |
| [Stage Structure](stages.md) | Stage progression, scripting format, campaign |
| [Specs: Chain / Rank / Pressure](specs/chain_rank_pressure.md) | First-pass implementation spec for performance loop |
| [Specs: Vacuum Window](specs/vaccum_window_mechanic.md) | Proposed environmental mechanic (spaceship stage) |

## Status Markers

- ✅ **Implemented** — in the codebase and functional
- 🚧 **In Progress** — partially built or actively developed
- 💡 **Proposed** — designed but no implementation exists

## Design History

Major design shifts recorded inline:

```
[v0.x — YYYY-MM-DD] Description
```

Sparingly. Minor changes belong in git history.

## Conventions

- This GDD is a **reference document**, not a brainstorm notebook
- Short sections, explicit headings, minimal repetition
- Open questions separated from settled decisions
- Implementation details live in code — link to source where useful

## Related

- [Technical Debt](../TECH_DEBT.md) — known issues and improvement backlog
