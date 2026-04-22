# Post-mortem: Perspective Grid and Lateral Shift Refactor

**Module:** stage projection (`projection.rs` and dependents)
**Period:** April 2026
**Status:** Landed; runtime integration in progress
**Authors:** design and review collaboration; implementation by Claude Code

---

## Executive summary

The stage projection module had a mathematically correct core (`ProjectionProfile`, the depth-to-Y curve) wrapped in a guide-ray sampling layer that was structurally broken. The brokenness was visible (clumped rays, gaps near the horizon, instability under lateral shift) but its root cause was not: the sampling layer fused two independent parameterisations and applied a distortion knob whose semantics couldn't be made coherent.

The refactor replaced the sampling layer with a single principled algorithm — uniform world-lane sampling, projected to viewport-boundary exit points, filtered by greedy perimeter spacing — that derives the right ray distribution from one geometric primitive. The artistic surface shrank from six knobs (some fighting each other) to three (orthogonal, each with clear visual meaning).

Along the way: a perimeter-coordinate sign bug surfaced under lateral shift, the colour gradient floor needed loosening to read as a fade, a `horizon_fade` toggle was added, and the runtime integration uncovered a pre-existing question about whether sprites and the grid should share a projection model. Conclusion: they should not, and the divergence is documented as intentional.

This post-mortem captures the reasoning, the failure modes, the mathematics that survived, and the possibilities the cleaned-up surface enables.

---

## 1. Initial state and what was broken

The `ProjectionProfile` curve was correct: a power-bias lerp between horizon Y and foreground Y, with monotonic odd extension through the horizon for past-horizon extrapolation. This part stayed.

The grid sampling layer had several entangled problems, the most consequential of which is the first:

### 1.1 Two independent ray families fused without reconciliation

`build_guide_ray_candidates` generated guide rays in two passes:

- A bottom-edge sampling pool, uniform in viewport-bottom X.
- Two side-edge sampling pools (left and right), clustered toward the horizon by a power exponent.

These were unioned and deduplicated by lane proximity. There was no single ordered parameterisation of the visible ray family — instead, the field was assembled from three sub-families with three different sampling rules, then merged. Any artistic distortion knob had no coherent variable to act on, because there was no single ordered visible variable.

The author of the original code recognised this and left a multi-paragraph `TODO(perspective-grid)` comment laying out the structural critique. The comment was correct; no one had had time to act on it.

### 1.2 Major/minor cadence indexed locally per family

The `family_alpha(sample_idx, major_interval, …)` function was called separately within each sub-family, with `sample_idx` reset to zero in each pool. After dedup merged candidates from different pools, the global cadence pattern was an artifact of which candidates happened to survive the merge — not a designed rhythm.

### 1.3 Lateral shift composition incorrect

`lateral_view_offset` was applied only at endpoint resolution time, not at candidate generation. The candidate pool was always centred on the unshifted vanishing column; under shift, the *visible* set of rays was whatever survived dedup of an off-centre projection of an on-centre pool. This is structurally indistinguishable from "noise."

### 1.4 Horizon corner gaps

The bottom-only main family naturally left visible gaps near the horizon corners of the viewport. The side-edge pools existed to fill those gaps. But because they were independent samplings rather than continuations of the main family, the resulting density distribution was uneven and the cadence was scrambled across the boundary.

### 1.5 Bias interpolation

`ProjectionProfile::lerp` interpolated `bias_power` linearly. Linear interpolation of an exponent does not produce a perceptually intermediate curve — a 50% lerp between `bias=1` and `bias=4` is closer to `bias=4` perceptually than to `bias=2.5`, because the curve family responds nonlinearly to the exponent.

### 1.6 Off-screen vanishing point edge case

`resolve_runtime_ray_endpoint` had a fallback branch for the degenerate case `eff_lane ≈ 0` that returned `(vanish_x, vp.min.y)` — silently incorrect if `vanish_x` lay outside the viewport's X range. The function had no precondition asserting otherwise.

---

## 2. Why the brokenness wasn't obvious

Some of these issues had no easy visual signature:

- **The two-family fusion produced an OK-looking grid at zero shift.** The badness was a structural property of how the grid would behave under shift or under parameter changes. At rest, the dedup balanced things enough to look approximately right.
- **Major/minor cadence noise was below the threshold of visual annoyance.** You could see the rhythm wasn't perfectly even, but unless you were looking for it, you'd attribute it to the underlying perspective rather than to indexing logic.
- **Linear bias interpolation only produced visible jank during projection tweens between profiles with very different bias powers.** Most authored stages didn't tween bias significantly.
- **Lateral shift was rarely exercised at extreme values** during initial development; the fusion's failure modes only became apparent when shift was treated as a first-class runtime input.

The original author's TODO comment was the artefact that made the structural problems addressable. Without that explicit critique, the "looks mostly OK at default settings" property would have continued to mask the underlying issue indefinitely.

---

## 3. The mathematics that survived

Two pieces of math were correct from the start and were preserved unchanged.

### 3.1 The depth-to-Y curve

For depth `d ∈ [1, 9]`, normalised progress `t = (9 - d) / 8 ∈ [0, 1]`, with `t = 0` at the horizon and `t = 1` at the foreground floor:

```
y(t) = horizon_y + sign(t) · |t|^p · (floor_base_y - horizon_y)
```

The `sign(t) · |t|^p` form (via `t.abs().powf(p).copysign(t)` in Rust) produces a monotonic odd extension through the horizon, which means depth values past 9 (extrapolating beyond the horizon) and past 1 (extrapolating below the foreground) behave geometrically sensibly. This was a deliberate and correct design choice in the original code.

### 3.2 The exit-point formula

A ray from the vanishing point `(v_x, v_y)` at effective lane `L_eff` reaches screen X coordinate `v_x + L_eff · w` at depth weight `w`, where `w` ranges from 0 at the horizon to `w_b` at the bottom of the viewport:

```
w_b = (vp.min.y - v_y) / depth_span
```

To find where the ray exits the viewport boundary going downward, take the minimum of:
- `w_bottom`: the weight at which y reaches the viewport bottom.
- `w_side`: the weight at which the ray's screen X reaches the relevant side edge.

This is straightforward 2D linear math but it is the foundation of everything that follows. The grid math is correct because this primitive is correct.

### 3.3 Sticky-carry-forward projection evaluation

`evaluate_projection_at` walks step indices, finds the most recent step with a projection override, and lerps between consecutive overrides during tween steps. This was correct and remained untouched.

---

## 4. The replacement algorithm

The new algorithm is one parameterisation, one filter. It is small enough to state in full.

### 4.1 Generate candidates

For integer `k` in `[-K_search, +K_search]`:

1. World lane: `world_lane = k * lane_spacing`.
2. Effective lane (lateral shift composed at generation, not at render): `eff_lane = world_lane - lateral_view_offset`.
3. Exit point on viewport boundary: `compute_exit(eff_lane, ...)`.
4. Perimeter coordinate of exit point along viewport boundary: `perimeter_coord(exit_point, ...)`.
5. Candidate flags: `is_center = (k == 0)`, `is_major = (k != 0 && |k| % major_ray_interval == 0)`.

`K_search` is derived to ensure the outermost sampled ray approaches within `horizon_fill` of the horizon:

```
K_search = ⌈(halfwidth · |depth_span|) / (horizon_fill · lane_spacing)⌉
        + ⌈|lateral_view_offset| / lane_spacing⌉
        + 8                              (cushion for shift edge cases)
```

with a floor of 32 for degenerate small viewports.

### 4.2 Greedy perimeter filter

Sort candidates by their perimeter coordinate. Locate the centre candidate (`k = 0`, guaranteed to exist). Walk forward and backward from centre, keeping each candidate iff its perimeter distance to the last-kept candidate is at least `horizon_fill` pixels.

This produces a globally ordered, evenly-spaced (in viewport-perimeter terms) ray fan, with no duplicate rays and no special treatment for majors. Major styling is assigned from `is_major` at render time, not at filter time — so the cadence is world-anchored and stable under shift.

### 4.3 Three artistic knobs

The three knobs that survived the redesign:

- **`lane_spacing`** (world pixels at depth 1, default 80–160): the *amplitude*. Wider spacing = bolder structural cadence with fewer total rays. Narrower = denser, finer fan.
- **`horizon_fill`** (screen pixels, default 4–6): the undersampling threshold. Larger = sparser horizon (rays are skipped more aggressively where they'd visually overlap). Smaller = denser horizon (overdraw acceptable).
- **`major_ray_interval`** (positive integer, default 4): every Nth world lane is styled as a major ray. Cadence rhythm.

These three are *orthogonal* — changing one does not necessitate retuning another. This is the property the original sampling layer could not provide.

### 4.4 The two formulas the grid quietly reconciles

Two perspective formulas appear in the algorithm, both correct, neither obviously redundant:

- Ray geometry: `x(w) = v_x + L_eff · w` (linear in depth weight).
- Floor Y placement (in `ProjectionProfile`): `y(t) = y_h + sign(t)|t|^p (y_f - y_h)` (power-biased in depth progress).

These are different things: one is "where in screen X does a ray sit at depth weight `w`," the other is "what screen Y is depth `d`'s floor at." The ray is straight in screen space; the floor lines are spaced according to the bias curve. The grid renders rays as straight lines and floor lines at biased Y positions, and the *intersection* of a ray with a floor line therefore inherits the bias spacing automatically. The bias and the projection compose correctly without either needing to know about the other.

---

## 5. Failures during the refactor

The refactor wasn't clean on the first pass. Several failures occurred, each instructive.

### 5.1 The widget-as-spec failure

Early in the design conversation, an interactive SVG widget was used to demonstrate the proposed algorithm. The widget had a draw-loop that filtered rays by "is the bottom-edge hit inside the viewport?" — silently dropping rays whose bottom hit was off-screen but whose visible segment exited the side. The result was visible horizon-corner gaps in the demo widget, which were then read as a property of the algorithm.

The user spotted this immediately. The fix was straightforward (rays exit at the *first* boundary hit, bottom or side, both are part of the same family) but it revealed a real risk in using interactive demos to develop an algorithm spec: the demo's bugs become indistinguishable from the algorithm's bugs.

**Lesson:** demos used as spec must be cross-checked against the spec text, especially around boundary cases. The widget should be implementing the spec, not deriving it.

### 5.2 The `K_max` formula confusion

A subsequent widget version derived `K_max` from a *non-redundancy* criterion ("stop when adjacent rays' exit points are closer than a pixel apart") rather than a *fill* criterion ("ensure the outermost ray approaches within ε of the horizon"). The widget produced too few rays and the user spotted a horizon gap.

The root error was conflating two different problems: avoiding overdraw versus filling the aperture. The fix was to recognise the criterion needed was fill-based, then derive `K_search` and combine with the greedy perimeter filter to get fill *without* overdraw.

**Lesson:** when a derived bound is set, name explicitly which optimisation it performs. "Stop sampling when X" requires saying what X is in service of.

### 5.3 The perimeter-coordinate sign bug

After the refactor landed, the user reported visible gaps in the middle of the grid that were "especially noticeable when shifting." The bug was in `perimeter_coord`:

```rust
2.0 * halfwidth + (viewport.min.y - exit.y).abs() * sign
```

where `sign = depth_span.signum()`. In the project's coordinate convention, `depth_span < 0`, so `sign = -1`. The right-edge perimeter coordinate started at `2*halfwidth` and *decreased* as rays approached the horizon, colliding with the bottom-edge range `[0, 2h]`. The greedy filter saw colliding perimeter coordinates as duplicates and suppressed rays accordingly.

The fix was to remove the `sign` multiplication entirely. The perimeter coordinate's job is to be monotonic along the boundary; using `up_from_bottom = (exit.y - viewport.min.y).abs()` directly accomplishes this in any Y convention. The function no longer needs to know about `depth_span` at all.

**Lesson:** when a coordinate transformation depends on a sign convention from elsewhere in the code, that sign needs to be unit-tested explicitly. The perimeter monotonicity property could and should have had a dedicated test before the integration revealed the bug under shift.

This bug also wasn't caught by the certify_ tests in the spec because the tests used the same buggy formula in their local `peri` closure as production did. The tests were tautological — they verified that production matched a wrong reference. After the perimeter fix, the test was updated to call production `perimeter_coord` directly, eliminating the duplication.

### 5.4 The perceived weak fade

After the refactor, the colour gradient from horizon to foreground was barely visible. The function `grid_color_rgba` had:

```
b = 0.4 + 0.6 * brightness
alpha = base_alpha * brightness^0.8
```

The 0.4 floor on `b` and the softening exponent on alpha combined to keep the horizon end of each ray bright enough that the per-segment gradient didn't read as a meaningful fade. The fix was to drop the floor to 0.15 and remove the alpha softening (`alpha * brightness` directly):

```
b = 0.15 + 0.85 * brightness
alpha = base_alpha * brightness
```

This produced a ~3:1 brightness ratio and ~5:1 alpha ratio between depth 1 and depth 9, which reads correctly as a fade.

The horizontal floor lines, which use the same colour function, now also visibly fade — addressing the user's correct observation that the fade should apply to both layer types.

**Lesson:** colour ramps need the actual visual range tested against the actual rendering backend before settling on coefficients. A formula that produces "different colours" mathematically can still produce visually identical output if the differences fall below perceptual thresholds in context.

### 5.5 The Spidey lateral-shift gap

After enabling runtime lateral shift in the depth_traverse example, sprites updated correctly *during* movement phases but not during *idle* phases. Investigation revealed that `SpideyIdle` and `SpideyLanding` simply didn't touch `pos.0` — only `SpideyJumping` called `project_lateral_x`. So when the camera shifted while Spidey was at rest, his stored screen X stayed stale until the next jump began.

The fix was a two-line addition: `pos.0.x = project_lateral_x(...)` in both phases.

**Lesson:** when adding a runtime input that affects positions, audit every system that owns positions to ensure each one reads the input. The bug was a sin of omission, not commission, and not catchable by any test that didn't explicitly verify "position updates while idle."

---

## 6. The grid vs sprite projection question

Late in the work, while integrating runtime lateral shift, a question surfaced about whether `project_lateral_x` (sprite projection) should match the grid's projection model.

### 6.1 The two formulas

- **Sprite formula (current):** `screen_x = world_x - lateral_view_offset · w`
- **Grid formula:** `screen_x = vanish_x + (world_x - vanish_x - lateral_view_offset) · w`

These produce different results except when `world_x == vanish_x`. The sprite formula treats `world_x` as a screen anchor that gets parallax-modulated. The grid formula treats it as a 3D world coordinate that gets projected.

### 6.2 The decision

The decision was to keep the sprite formula and document the divergence as intentional.

Rationale:

- The genre (Space Harrier / Panzer Dragoon style pseudo-3D scrolling shooter) authors entities at screen anchors. Players expect a depth-9 enemy at world X = 500 to *appear* at screen X = 500, not collapse to the vanishing column.
- The sprite formula gives perfect X stability at depth 9 (`w = 0` zeroes the shift entirely). This is a desirable game-feel property.
- The grid is a debug/authoring aid that visualises floor-plane geometry. It serves a different purpose from sprite positioning, and is allowed to use a different model.

### 6.3 What was rejected and why

Path 2 — switch to the grid's formula and reinterpret depth 9 as "near horizon, w > 0" — was considered. It would unify the projection models at the cost of:

- Migrating all existing authored enemy positions (world X meanings change).
- Introducing sub-pixel drift at depth 9 under shift.
- Making depth-9 placement less direct for authoring (artist can't type "screen X = 500" directly).

The trade was unattractive given that the divergence is invisible during gameplay (grid is debug-only). If a future cinematic effect needs sprites to converge to the vanishing point (a "warp in from infinity" beat), it can be implemented as an opt-in per-entity transform on top of the current model rather than committing the whole system.

### 6.4 Documentation

A doc comment was added above `project_lateral_x` explaining the divergence, its rationale, and its consequences. This is an explicit anti-trap for future maintainers: the divergence is *intentional* and not something to "fix."

---

## 7. Possibilities the cleaned surface enables

Several things become straightforward now that the algorithm is principled.

### 7.1 Player-visible perspective grid as game art

Currently the grid is a debug overlay rendered with gizmos. The output type (`PerspectiveGrid` of `GridLineSegment`) is rendering-backend-agnostic. A future pixel-line variant could re-render the same geometry as proper sprite-aligned pixel lines for a player-facing TRON / Rez / vector-arcade aesthetic. The math doesn't need to change; only the consumer of `GridLineSegment`.

### 7.2 Animated grid parameters

`lane_spacing`, `horizon_fill`, and `major_ray_interval` could be promoted from `GridParams` to `ProjectionProfile` (with log-space interpolation for `lane_spacing`, like `bias_power` already does). This would let stages tween between "wide structural" and "dense ambient" grids during cinematic beats. Currently parked as a future possibility per the design discussion — the toolling-only nature of the current grid makes it not worth the schema bloat yet.

### 7.3 Off-centre vanishing point

The current code asserts `vanish_x ∈ [vp.min.x, vp.max.x]`. Removing this and allowing the vanishing point to walk off-screen would enable angled-camera cinematic shots (looking down a corridor at an angle, dutched compositions). The algorithm itself does not depend on centred vanishing points; the assertion is a precondition, not a load-bearing invariant. Lift it when a use case appears.

### 7.4 Per-entity projection mode

The sprite-vs-grid divergence (§6) could be inverted as a per-entity opt-in: most entities use the screen-anchor parallax model, but entities flagged "true projection" use the grid's converging formula. Useful for "ghost" or "warp" effects. This is a one-flag change to `project_lateral_x` rather than a system-wide rewrite.

### 7.5 Camera lateral-shift driven by player input

Currently `lateral_view_offset` is a global resource. A future "player can lean" mechanic, where pressing left/right pans the camera within limits, plugs into the same resource without further refactoring. The runtime camera writer becomes additive rather than authoritative, or the resource becomes a sum of (camera-driven) and (input-driven) components.

### 7.6 Stage-load validation upgrade

`validate_stage_projections` currently checks profile invariants. It could be extended to check `GridParams` validity per stage (if grids ever become per-stage configurable), or to verify that `lane_spacing` and `horizon_fill` produce a non-degenerate ray count given the stage's viewport. Cheap to add, catches a class of "looks wrong but no error" bugs at load time.

---

## 8. Process notes

The work spanned roughly two weeks and involved:

- One file's worth of discussion-driven design (this conversation).
- Three planned commit tracks (Track A: independent local fixes; Track B: main refactor; Track C: integration).
- Several follow-up commits as visual issues surfaced (perimeter sign bug, weak fade, Spidey idle gap).
- A large body of certification tests that successfully caught regressions during the refactor and several integration commits, while failing to catch the perimeter sign bug because they shared the bug's source formula.

A few process observations:

### 8.1 The TODO comment was the unblocker

The original `TODO(perspective-grid)` block was three paragraphs of structural critique with no fix attached. It was the artefact that turned "the grid looks slightly off sometimes" into an actionable refactor. Without it, the structural problems would have been re-discovered by every contributor and individually patched in incompatible ways.

**Pattern:** when you see a problem you can't fix, write the critique anyway. Future contributors with more context can act on it.

### 8.2 Iterative widget-driven design caught real bugs

The interactive widgets used to develop the algorithm caught two design errors (the bottom-only filter, the wrong `K_max` criterion) before any code was written. They also introduced two presentational errors (the spec-vs-widget discrepancy in §5.1, the diagram bug in the sprite-projection comparison) that could have been load-bearing if the user hadn't spotted them.

**Pattern:** widgets are useful for design conversations but should be explicitly flagged as approximations. Their bugs are not the algorithm's bugs, and their correctness is not the algorithm's correctness.

### 8.3 Tests that share their reference formula are tautologies

`certify_perimeter_spacing_respects_horizon_fill` initially had a local `peri` closure that duplicated production's `perimeter_coord`. When production had a sign bug, the test was updated to mirror it without the bug being noticed. The fix was to call production directly from the test rather than re-implement.

**Pattern:** tests should reach for the production function under test, not re-implement it. Re-implementation creates a tautology and silently masks the bug class the test was meant to detect.

### 8.4 Splitting the work into Tracks A and B was load-bearing

Track A (log-space bias lerp, off-screen-vanish assertion, alpha hierarchy comment, dedup epsilon parameterisation) was four small independent commits that landed safely before the main refactor. Each was reviewable in isolation and any of them could be reverted without affecting the others. The main refactor then landed as a single semantically-complete commit.

Without this split, a regression in any one piece would have been hard to bisect. With the split, every commit has a clear semantic purpose and review burden was distributed.

**Pattern:** when refactoring a complex module, identify the orthogonal local fixes first and land them as their own commits before the big change. The big change becomes smaller and more reviewable, and the local fixes' value is realised even if the big change is delayed or revised.

### 8.5 The handoff prompt structure worked

The Claude Code prompts used for execution had a consistent structure: scope, deletions, additions, tests (added/updated/unchanged/new), commit sequence, anti-patterns. The anti-patterns section in particular pre-empted several mistakes the iterative design conversation had discovered along the way ("don't apply lateral_view_offset twice," "don't reintroduce a supplemental family," etc.).

**Pattern:** when handing off a non-trivial change, document the things-not-to-do as explicitly as the things-to-do. The wrong path is often the path most contributors will be tempted toward.

---

## 9. Summary table

| Aspect | Before | After |
|---|---|---|
| Sampling layer | Two families (bottom + sides), unioned and deduped | One family (uniform world-lane), greedy perimeter filter |
| Artistic knobs | `bottom_ray_count`, `side_ray_count_per_side`, `side_horizon_cluster_power`, `major_ray_interval`, alphas | `lane_spacing`, `horizon_fill`, `major_ray_interval`, alphas |
| Lateral shift composition | Applied at render time only | Applied at candidate generation, removed from render |
| Major cadence | Indexed locally per family | Indexed globally, world-anchored |
| Bias interpolation in lerp | Linear (perceptually wrong) | Log-space (geometric mean at midpoint) |
| Off-screen vanish | Silent fallback | Debug assert with clear message |
| Dedup logic | Required (lane-proximity) | Removed (uniform sampling produces no duplicates) |
| Perimeter coordinate | Sign-contaminated, broke under non-default coordinate convention | Sign-agnostic, monotonic by construction |
| Horizon fade | Ramp coefficients made fade barely visible | Looser floor and unsoftened alpha for clear gradient; opt-in via `horizon_fade` |
| Sprite projection | Depth-weighted parallax (correct for genre) | Unchanged; documented as intentionally different from grid model |
| Runtime camera integration | Did not exist | `ProjectionView.lateral_view_offset` driven by camera position; CMD+P toggles debug grid in game |

---

## 10. What's left as known follow-ups

- Runtime camera integration commits (per-stage anchor, CMD+P toggle, runtime sprite parallax) — currently in flight.
- Sprite projection model documented as intentionally divergent from grid; if a future use case requires unification, it's a deliberate choice with known consequences (§6.3).
- Per-stage anchor for `lateral_view_offset` vs per-step anchor — chose per-stage for simplicity; revisit if pacing demands otherwise.
- Player-visible pixel-line grid variant — possible without further refactoring; just a new consumer of `PerspectiveGrid`.
- `lane_spacing` and `horizon_fill` could become per-`ProjectionProfile` if grids ever animate; currently parked as future possibility.

---

*This post-mortem is a snapshot of design reasoning at landing time. If a future contributor finds themselves disagreeing with a decision documented here, they should treat the disagreement as a signal to read the conversation that produced it before reverting — the constraints that drove the choice may not be visible from the code alone.*
