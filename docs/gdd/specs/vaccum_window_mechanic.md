# Vacuum Window Mechanic — Design Spec

## Overview

Destructible ship windows create a **temporary vacuum event** that affects the entire scene.

- Player = screen (camera)
- Effect is **global**, not positional
- Uses **directional force + subtle Mode 7-style distortion**
- Primary purpose: environmental combat + dynamic moment-to-moment gameplay

---

## Core Loop

1. Player damages window shutters
2. Window breaks → vacuum event triggered
3. Enemies are pulled, spun, and ejected into space
4. Temporary gameplay distortion occurs
5. Shutters close → system returns to normal

---

## Window States

### 1. Closed

- Fully intact
- No effect

### 2. Damaged

- Visual cracks / leaks
- Minor particle drift toward window
- No gameplay impact (or very subtle)

### 3. Broken (Open)

- Full vacuum effect active
- Window interior = darkest value (alpha region)

---

## Vacuum Behaviour

### Phase 1 — Burst (Immediate)

- Triggered on break
- Short impulse toward window
- Nearby enemies instantly pulled

### Phase 2 — Sustain (1–3s)

- Continuous directional pull toward window
- Enemies:
  - drift toward opening
  - spin / enter death state
  - exit screen
- Bullets / particles:
  - slight directional bias

### Phase 3 — Recovery

- Shutters close automatically
- World returns to neutral state

---

## Camera / Screen Behaviour

Player is not moved. Instead:

### Mode 7-style effect (subtle)

- slight rotation toward window (≈ 3–8° max)
- minor scaling / skew (≤ 1.1)
- directional bias (not centred)

### Additional feedback

- brief screen shake on burst
- directional particle flow
- no full rotation or disorientation

---

## Player Interaction

- Player cannot be physically displaced
- Effect is **readable and global**
- Optional:
  - player can trigger early shutter close
  - leaving window open longer increases effect duration

---

## Enemy Behaviour

Default:

- pulled toward window
- spin + “frozen” state
- ejected off-screen → death

Variants (future):

- resistant enemies (slower pull)
- anchored enemies (ignore or partially resist)
- special reactions (explode, cling, etc.)

---

## Design Goals

- Immediate, satisfying payoff
- Clear cause → effect relationship
- Adds risk/reward without frustration
- Maintains readability at all times

---

## Constraints

- No heavy camera disorientation
- No long-duration distortion
- Effect must remain readable at low resolution
- Must work within “player = screen” model

---

## Visual Language

- strong directional particles toward window
- enemies exaggerate motion (spin/stretch)
- clear contrast between:
  - interior (structure)
  - exterior (space void)
- vacuum reads instantly without UI

---

## Tuning Parameters

- pull strength (per phase)
- duration (1–3s target)
- rotation angle (max ~8°)
- scaling factor (~1.05–1.1)
- shutter close timing (auto vs manual override)

---

## Notes

- Default behaviour: **auto-close after short duration**
- Mode 7 effect is **supporting**, not dominant
- Mechanic should feel like:

  > “the world is being pulled into space”

- Use sparingly for maximum impact
- Can be expanded into:
  - level-specific puzzles
  - combo systems
  - boss interactions
