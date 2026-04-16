//! Post-export analysis and deduplication reporting.
//!
//! This module computes advisory metrics about the exported atlas without
//! changing any output. The report helps identify structural dedup opportunities
//! such as monolithic parts that could be split into semantic sub-parts.

use crate::aseprite::{CompositionAtlas, InternerStats};
use image::RgbaImage;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

/// Complete analysis report for one exported entity/depth pair.
#[derive(Clone, Debug, Serialize)]
pub struct AnalysisReport {
    pub entity: String,
    pub depth: u8,
    pub atlas: AtlasStats,
    pub interner: InternerStatsReport,
    pub parts: Vec<PartAnalysis>,
}

/// Atlas-level aggregate statistics.
#[derive(Clone, Debug, Serialize)]
pub struct AtlasStats {
    /// Number of unique sprites in the packed atlas.
    pub unique_sprites: usize,
    /// Total fragment references across all animation frame poses.
    pub total_references: usize,
    /// Total logical part references across all animation frames.
    pub logical_part_references: usize,
    /// Unique-sprite ratio across emitted fragment references.
    pub unique_sprite_reference_ratio: f64,
    /// Reference savings: `1.0 - (unique / total)`. Higher is better.
    pub dedup_ratio: f64,
    /// Fragment growth versus logical part references.
    pub fragment_growth_ratio: f64,
    /// Total pixel area occupied by unique sprites (sum of w*h per sprite).
    pub used_pixels: u64,
    /// Atlas image dimensions (width, height).
    pub atlas_size: [u32; 2],
    /// Total atlas pixel area (width * height).
    pub atlas_pixels: u64,
    /// Packing efficiency: `used_pixels / atlas_pixels`.
    pub packing_efficiency: f64,
}

/// Interner-level dedup statistics.
#[derive(Clone, Debug, Serialize)]
pub struct InternerStatsReport {
    pub total_interns: u32,
    pub new_sprites: u32,
    pub exact_hits: u32,
    pub flip_x_hits: u32,
    pub flip_y_hits: u32,
    pub flip_xy_hits: u32,
    pub hit_rate: f64,
}

/// Per-part analysis with dedup metrics and split candidate detection.
#[derive(Clone, Debug, Serialize)]
pub struct PartAnalysis {
    pub part_id: String,
    /// Whether this part has a source layer (is visual).
    pub is_visual: bool,
    /// Number of distinct sprites used by this part across all animations.
    pub unique_sprites: usize,
    /// Total pose references across all animation frames.
    pub total_references: usize,
    /// Reuse ratio: `total_references / unique_sprites`.
    pub reuse_ratio: f64,
    /// Sum of (width * height) for all unique sprites used by this part.
    pub total_area_px: u64,
    /// Horizontal symmetry score: 0.0 (asymmetric) to 1.0 (perfect mirror).
    /// Only computed for visual parts; `None` for non-visual bridge markers.
    pub symmetry_score: Option<f64>,
    /// Bridge marker part IDs that could become visual sub-parts of this part.
    /// Detected by matching tag prefixes (e.g., `wings_visual` → `wing_l`, `wing_r`).
    pub bridge_markers: Vec<String>,
    /// Whether this part is a candidate for structural splitting.
    pub split_candidate: bool,
}

/// Minimum trimmed pixel area to consider a part worth analysing for splitting.
const MIN_AREA_FOR_SPLIT: u64 = 400;
/// Symmetry score threshold above which a split is likely to yield flip dedup.
const SYMMETRY_THRESHOLD: f64 = 0.85;

/// Build an analysis report from the exported atlas metadata and interner stats.
///
/// When `atlas_image` is provided, pixel-level horizontal symmetry is computed
/// for each visual part's sprites. When `None`, symmetry scores are omitted.
pub fn build_report(
    atlas: &CompositionAtlas,
    interner_stats: &InternerStats,
    atlas_width: u32,
    atlas_height: u32,
    atlas_image: Option<&RgbaImage>,
) -> AnalysisReport {
    let total_references = count_total_references(atlas);
    let logical_part_references = count_logical_part_references(atlas);
    let used_pixels = atlas
        .sprites
        .iter()
        .map(|s| u64::from(s.rect.w) * u64::from(s.rect.h))
        .sum();
    let atlas_pixels = u64::from(atlas_width) * u64::from(atlas_height);

    // Build sprite-id → rect lookup for area computation.
    let sprite_rects: HashMap<&str, (u32, u32)> = atlas
        .sprites
        .iter()
        .map(|s| (s.id.as_str(), (s.rect.w, s.rect.h)))
        .collect();

    // Collect per-part pose data.
    let mut part_sprites: HashMap<&str, Vec<&str>> = HashMap::new();
    for anim in &atlas.animations {
        for frame in &anim.frames {
            for pose in &frame.parts {
                part_sprites
                    .entry(pose.part_id.as_str())
                    .or_default()
                    .push(pose.sprite_id.as_str());
            }
        }
    }

    // Find bridge markers: non-visual parts whose tags suggest they're
    // sub-parts of a visual part (e.g., wing_l/wing_r for wings_visual).
    //
    // Matching strategy: find the best visual part match for each bridge
    // marker by counting shared *specific* tags (excluding generic tags
    // like "limb", "targetable", "group", "overlay", "visual_only").
    let generic_tags: HashSet<&str> = [
        "limb",
        "targetable",
        "group",
        "overlay",
        "visual_only",
        "core",
        "left",
        "right",
    ]
    .into_iter()
    .collect();

    let visual_parts: Vec<&str> = atlas
        .parts
        .iter()
        .filter(|p| p.source_layer.is_some() || p.source_region.is_some())
        .map(|p| p.id.as_str())
        .collect();

    let bridge_markers: Vec<(&str, &str)> = atlas
        .parts
        .iter()
        .filter(|p| p.source_layer.is_none())
        .filter_map(|marker| {
            let marker_def = atlas
                .part_definitions
                .iter()
                .find(|d| d.id == marker.definition_id)?;
            let marker_specific_tags: HashSet<&str> = marker_def
                .tags
                .iter()
                .map(String::as_str)
                .filter(|t| !generic_tags.contains(t))
                .collect();

            if marker_specific_tags.is_empty() {
                return None;
            }

            // Find the visual part with the most specific tag overlap.
            let mut best_match: Option<(&str, usize)> = None;
            for &vp_id in &visual_parts {
                let Some(vp) = atlas.parts.iter().find(|p| p.id.as_str() == vp_id) else {
                    continue;
                };
                let Some(vp_def) = atlas
                    .part_definitions
                    .iter()
                    .find(|d| d.id == vp.definition_id)
                else {
                    continue;
                };
                let vp_specific_tags: HashSet<&str> = vp_def
                    .tags
                    .iter()
                    .map(String::as_str)
                    .filter(|t| !generic_tags.contains(t))
                    .collect();

                let overlap = marker_specific_tags.intersection(&vp_specific_tags).count();
                if overlap > 0 && best_match.is_none_or(|(_, best)| overlap > best) {
                    best_match = Some((vp_id, overlap));
                }
            }

            best_match.map(|(vp_id, _)| (marker.id.as_str(), vp_id))
        })
        .collect();

    // Group bridge markers by their visual parent.
    let mut bridges_by_visual: HashMap<&str, Vec<&str>> = HashMap::new();
    for (marker_id, visual_id) in &bridge_markers {
        bridges_by_visual
            .entry(visual_id)
            .or_default()
            .push(marker_id);
    }

    let parts: Vec<PartAnalysis> = atlas
        .parts
        .iter()
        .map(|part| {
            let is_visual = part.source_layer.is_some() || part.source_region.is_some();
            let refs = part_sprites.get(part.id.as_str());
            let total_refs = refs.map_or(0, Vec::len);

            let unique: Vec<&str> = refs.map_or_else(Vec::new, |r| {
                let mut u: Vec<&str> = r.clone();
                u.sort_unstable();
                u.dedup();
                u
            });
            let unique_count = unique.len();
            let reuse_ratio = if unique_count > 0 {
                total_refs as f64 / unique_count as f64
            } else {
                0.0
            };

            let total_area_px: u64 = unique
                .iter()
                .filter_map(|sid| sprite_rects.get(sid))
                .map(|(w, h)| u64::from(*w) * u64::from(*h))
                .sum();

            let symmetry_score = if is_visual && !unique.is_empty() {
                atlas_image.map(|img| compute_symmetry_score(&unique, atlas, img))
            } else {
                None
            };

            let markers = bridges_by_visual
                .get(part.id.as_str())
                .map_or_else(Vec::new, |m| m.iter().map(|s| (*s).to_string()).collect());

            let split_candidate = is_visual
                && !markers.is_empty()
                && total_area_px >= MIN_AREA_FOR_SPLIT
                && symmetry_score.is_some_and(|s| s >= SYMMETRY_THRESHOLD);

            PartAnalysis {
                part_id: part.id.clone(),
                is_visual,
                unique_sprites: unique_count,
                total_references: total_refs,
                reuse_ratio,
                total_area_px,
                symmetry_score,
                bridge_markers: markers,
                split_candidate,
            }
        })
        .collect();

    AnalysisReport {
        entity: atlas.entity.clone(),
        depth: atlas.depth,
        atlas: AtlasStats {
            unique_sprites: atlas.sprites.len(),
            total_references,
            logical_part_references,
            unique_sprite_reference_ratio: if total_references > 0 {
                atlas.sprites.len() as f64 / total_references as f64
            } else {
                0.0
            },
            dedup_ratio: if total_references > 0 {
                1.0 - (atlas.sprites.len() as f64 / total_references as f64)
            } else {
                0.0
            },
            fragment_growth_ratio: if logical_part_references > 0 {
                (total_references as f64 - logical_part_references as f64)
                    / logical_part_references as f64
            } else {
                0.0
            },
            used_pixels,
            atlas_size: [atlas_width, atlas_height],
            atlas_pixels,
            packing_efficiency: if atlas_pixels > 0 {
                used_pixels as f64 / atlas_pixels as f64
            } else {
                0.0
            },
        },
        interner: InternerStatsReport {
            total_interns: interner_stats.total_interns,
            new_sprites: interner_stats.new_sprites,
            exact_hits: interner_stats.exact_hits,
            flip_x_hits: interner_stats.flip_x_hits,
            flip_y_hits: interner_stats.flip_y_hits,
            flip_xy_hits: interner_stats.flip_xy_hits,
            hit_rate: if interner_stats.total_interns > 0 {
                1.0 - (interner_stats.new_sprites as f64 / interner_stats.total_interns as f64)
            } else {
                0.0
            },
        },
        parts,
    }
}

/// Count the total sprite references across all animation frame poses.
fn count_total_references(atlas: &CompositionAtlas) -> usize {
    atlas
        .animations
        .iter()
        .flat_map(|a| &a.frames)
        .map(|f| f.parts.len())
        .sum()
}

/// Count the total logical-part references across all animation frames.
fn count_logical_part_references(atlas: &CompositionAtlas) -> usize {
    atlas
        .animations
        .iter()
        .flat_map(|animation| &animation.frames)
        .map(|frame| {
            frame
                .parts
                .iter()
                .map(|pose| pose.part_id.as_str())
                .collect::<HashSet<_>>()
                .len()
        })
        .sum()
}

/// Compute a horizontal symmetry score for a set of sprite IDs.
///
/// For each unique sprite, compares the left half to a horizontal mirror of
/// the right half, pixel by pixel. Returns 0.0 (fully asymmetric) to 1.0
/// (perfect horizontal mirror).
///
/// This is advisory only — used to identify split candidates, never to
/// drive automatic decisions.
fn compute_symmetry_score(
    unique_sprite_ids: &[&str],
    atlas: &CompositionAtlas,
    atlas_image: &RgbaImage,
) -> f64 {
    if unique_sprite_ids.is_empty() {
        return 0.0;
    }

    let sprite_rects: HashMap<&str, &crate::aseprite::Rect> = atlas
        .sprites
        .iter()
        .map(|s| (s.id.as_str(), &s.rect))
        .collect();

    let mut total_score = 0.0;
    let mut total_weight = 0.0;

    for &sprite_id in unique_sprite_ids {
        let Some(rect) = sprite_rects.get(sprite_id) else {
            continue;
        };
        let w = rect.w;
        let h = rect.h;
        if w < 2 || h == 0 {
            continue;
        }

        let half_w = w / 2;
        let mut matching = 0u64;
        let mut total = 0u64;

        for y in 0..h {
            for x in 0..half_w {
                let mirror_x = w - 1 - x;
                let left = atlas_image.get_pixel(rect.x + x, rect.y + y);
                let right = atlas_image.get_pixel(rect.x + mirror_x, rect.y + y);
                total += 1;
                if left == right {
                    matching += 1;
                }
            }
        }

        if total > 0 {
            let score = matching as f64 / total as f64;
            let weight = (w * h) as f64; // Weight by area so larger sprites matter more.
            total_score += score * weight;
            total_weight += weight;
        }
    }

    if total_weight > 0.0 {
        total_score / total_weight
    } else {
        0.0
    }
}

/// Print a human-readable analysis summary to stderr.
pub fn print_report(report: &AnalysisReport) {
    let flip_total =
        report.interner.flip_x_hits + report.interner.flip_y_hits + report.interner.flip_xy_hits;

    eprintln!(
        "\n=== Dedup Report: {} depth={} ===",
        report.entity, report.depth
    );
    eprintln!(
        "Atlas: {} sprites, {}x{} px ({} total, {} used, {:.0}% packed)",
        report.atlas.unique_sprites,
        report.atlas.atlas_size[0],
        report.atlas.atlas_size[1],
        report.atlas.atlas_pixels,
        report.atlas.used_pixels,
        report.atlas.packing_efficiency * 100.0,
    );
    eprintln!(
        "Interner: {} cel interns -> {} unique sprites ({:.0}% hit rate)",
        report.interner.total_interns,
        report.interner.new_sprites,
        report.interner.hit_rate * 100.0,
    );
    eprintln!(
        "References: {} fragment refs, {} logical refs, {:.0}% reference savings, {:+.0}% fragment growth",
        report.atlas.total_references,
        report.atlas.logical_part_references,
        report.atlas.dedup_ratio * 100.0,
        report.atlas.fragment_growth_ratio * 100.0,
    );
    if flip_total > 0 || report.interner.exact_hits > 0 {
        eprintln!(
            "Reuse breakdown: {} exact, {} total flipped ({} flip_x, {} flip_y, {} flip_xy)",
            report.interner.exact_hits,
            flip_total,
            report.interner.flip_x_hits,
            report.interner.flip_y_hits,
            report.interner.flip_xy_hits,
        );
    }

    eprintln!("\nPer-part breakdown:");
    for part in &report.parts {
        if !part.is_visual && part.bridge_markers.is_empty() {
            continue; // Skip non-visual parts with no bridge significance.
        }

        let sym_str = part
            .symmetry_score
            .map_or_else(|| "n/a".to_string(), |s| format!("{s:.2}"));

        let mut flags = Vec::new();
        if !part.is_visual {
            flags.push("bridge");
        }
        if part.split_candidate {
            flags.push("SPLIT CANDIDATE");
        }
        if !part.bridge_markers.is_empty() {
            flags.push("has bridges");
        }

        let flag_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(", "))
        };

        eprintln!(
            "  {:<20} {:>2} sprites, {:>5} px, reuse={:.1}x, sym={}{flag_str}",
            part.part_id, part.unique_sprites, part.total_area_px, part.reuse_ratio, sym_str,
        );

        if !part.bridge_markers.is_empty() {
            eprintln!("    bridges: {}", part.bridge_markers.join(", "));
        }
    }
    eprintln!();
}

/// Write the analysis report as a JSON sidecar file.
pub fn write_json_report(report: &AnalysisReport, output_dir: &Path) -> anyhow::Result<()> {
    let path = output_dir.join("analysis.json");
    let json = serde_json::to_string_pretty(report)?;
    fs::write(&path, format!("{json}\n"))?;
    Ok(())
}

/// Near-miss dedup diagnostic: compares all unique sprites within each semantic
/// part pair to find cases where flip dedup *almost* works but fails.
///
/// For each candidate pair (e.g., arm_l vs arm_r), compares every sprite A from
/// one part against flip(B) for every sprite B from the other. Reports:
/// - dimension mismatches
/// - pixel-level diffs for same-dimension pairs
///
/// This is a diagnostic tool, not a runtime feature. Call it after export to
/// understand why certain parts are not deduplicating.
pub fn print_dedup_diagnostic(atlas: &CompositionAtlas, atlas_image: &RgbaImage) {
    use image::imageops;

    let sprite_rects: HashMap<&str, &crate::aseprite::Rect> = atlas
        .sprites
        .iter()
        .map(|s| (s.id.as_str(), &s.rect))
        .collect();

    // Collect sprites per part.
    let mut part_sprites: HashMap<&str, HashSet<&str>> = HashMap::new();
    for anim in &atlas.animations {
        for frame in &anim.frames {
            for pose in &frame.parts {
                part_sprites
                    .entry(pose.part_id.as_str())
                    .or_default()
                    .insert(pose.sprite_id.as_str());
            }
        }
    }

    // Find semantic pairs: parts that share a definition_id but have different IDs.
    let mut pairs: Vec<(&str, &str)> = Vec::new();
    for (i, pa) in atlas.parts.iter().enumerate() {
        for pb in atlas.parts.iter().skip(i + 1) {
            if pa.definition_id == pb.definition_id
                && pa.id != pb.id
                && part_sprites.contains_key(pa.id.as_str())
                && part_sprites.contains_key(pb.id.as_str())
            {
                pairs.push((pa.id.as_str(), pb.id.as_str()));
            }
        }
    }

    if pairs.is_empty() {
        return;
    }

    eprintln!("\n=== Dedup Diagnostic: near-miss analysis ===");

    for (pa_id, pb_id) in &pairs {
        let sprites_a: Vec<&str> = part_sprites[pa_id].iter().copied().collect();
        let sprites_b: Vec<&str> = part_sprites[pb_id].iter().copied().collect();

        // Check if they already share sprites (already deduped).
        let shared: HashSet<&str> = sprites_a
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .intersection(&sprites_b.iter().copied().collect())
            .copied()
            .collect();

        let a_only: Vec<&str> = sprites_a
            .iter()
            .copied()
            .filter(|s| !shared.contains(s))
            .collect();
        let b_only: Vec<&str> = sprites_b
            .iter()
            .copied()
            .filter(|s| !shared.contains(s))
            .collect();

        eprintln!(
            "\n{pa_id} vs {pb_id}: {}/{} shared, {}/{} unique",
            shared.len(),
            sprites_a.len(),
            a_only.len(),
            b_only.len()
        );

        if a_only.is_empty() && b_only.is_empty() {
            eprintln!("  fully deduped");
            continue;
        }

        // Compare each unique-to-A sprite against each unique-to-B sprite (flipped).
        let mut dim_mismatches = 0u32;
        let mut exact_matches = 0u32;
        let mut near_matches = 0u32;
        let mut large_diffs = 0u32;

        for &sa in &a_only {
            let Some(ra) = sprite_rects.get(sa) else {
                continue;
            };
            let img_a = imageops::crop_imm(atlas_image, ra.x, ra.y, ra.w, ra.h).to_image();

            for &sb in &b_only {
                let Some(rb) = sprite_rects.get(sb) else {
                    continue;
                };

                if ra.w != rb.w || ra.h != rb.h {
                    if (ra.w as i32 - rb.w as i32).unsigned_abs() <= 1
                        && (ra.h as i32 - rb.h as i32).unsigned_abs() <= 1
                    {
                        eprintln!(
                            "  {sa} ({w1}x{h1}) vs {sb} ({w2}x{h2}): dimension near-miss",
                            w1 = ra.w,
                            h1 = ra.h,
                            w2 = rb.w,
                            h2 = rb.h
                        );
                    }
                    dim_mismatches += 1;
                    continue;
                }

                let img_b = imageops::crop_imm(atlas_image, rb.x, rb.y, rb.w, rb.h).to_image();
                let flipped_b = imageops::flip_horizontal(&img_b);

                let mut diff_count = 0u32;
                for y in 0..ra.h {
                    for x in 0..ra.w {
                        if img_a.get_pixel(x, y) != flipped_b.get_pixel(x, y) {
                            diff_count += 1;
                        }
                    }
                }

                let total = ra.w * ra.h;
                if diff_count == 0 {
                    eprintln!(
                        "  {sa} vs flip({sb}): EXACT MATCH ({w}x{h}) — dedup should have caught this!",
                        w = ra.w,
                        h = ra.h
                    );
                    exact_matches += 1;
                } else if diff_count <= 10 {
                    eprintln!(
                        "  {sa} vs flip({sb}): {diff_count}/{total} pixels differ ({w}x{h})",
                        w = ra.w,
                        h = ra.h
                    );
                    near_matches += 1;
                } else {
                    large_diffs += 1;
                }
            }
        }

        eprintln!(
            "  summary: {} dim mismatch, {} exact (BUG), {} near (<= 10px), {} large diff",
            dim_mismatches, exact_matches, near_matches, large_diffs
        );
    }
    eprintln!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aseprite::{
        AnimationFrame, AtlasSprite, CompositionAtlas, CompositionGameplay, InternerStats,
        PartDefinition, PartInstance, PartPose, Point, Rect, Size, SourceRegion, SplitHalf,
    };

    fn make_sprite(id: &str, w: u32, h: u32) -> AtlasSprite {
        AtlasSprite {
            id: id.to_string(),
            rect: Rect { x: 0, y: 0, w, h },
        }
    }

    fn make_part(id: &str, def_id: &str, source_layer: Option<&str>) -> PartInstance {
        PartInstance {
            id: id.to_string(),
            definition_id: def_id.to_string(),
            name: id.to_string(),
            parent_id: None,
            source_layer: source_layer.map(String::from),
            source_region: None,
            split: None,
            draw_order: 0,
            pivot: Point::default(),
            tags: vec![],
            visible_by_default: true,
            gameplay: Default::default(),
        }
    }

    fn make_pose(part_id: &str, sprite_id: &str) -> PartPose {
        PartPose {
            part_id: part_id.to_string(),
            sprite_id: sprite_id.to_string(),
            local_offset: Point::default(),
            flip_x: false,
            flip_y: false,
            visible: true,
            opacity: 255,
            fragment: 0,
        }
    }

    fn make_atlas(
        sprites: Vec<AtlasSprite>,
        parts: Vec<PartInstance>,
        part_defs: Vec<PartDefinition>,
        frames: Vec<Vec<PartPose>>,
    ) -> CompositionAtlas {
        let animations = vec![crate::aseprite::Animation {
            tag: "test".to_string(),
            direction: "forward".to_string(),
            repeats: None,
            frames: frames
                .into_iter()
                .map(|poses| AnimationFrame {
                    source_frame: 0,
                    duration_ms: 100,
                    events: vec![],
                    parts: poses,
                })
                .collect(),
            part_overrides: vec![],
        }];

        CompositionAtlas {
            schema_version: 3,
            entity: "test".to_string(),
            depth: 1,
            source: "test.aseprite".to_string(),
            canvas: Size { w: 64, h: 64 },
            origin: Point::default(),
            spawn_anchor: Default::default(),
            ground_anchor_y: None,
            air_anchor_y: None,
            atlas_image: "source.png".to_string(),
            part_definitions: part_defs,
            parts,
            sprites,
            animations,
            gameplay: CompositionGameplay::default(),
        }
    }

    #[test]
    fn report_counts_references_correctly() {
        let atlas = make_atlas(
            vec![make_sprite("s0", 10, 10), make_sprite("s1", 20, 20)],
            vec![make_part("body", "body", Some("body"))],
            vec![PartDefinition {
                id: "body".to_string(),
                tags: vec![],
                gameplay: Default::default(),
            }],
            vec![
                vec![make_pose("body", "s0")],
                vec![make_pose("body", "s0")],
                vec![make_pose("body", "s1")],
            ],
        );

        let stats = InternerStats {
            total_interns: 3,
            new_sprites: 2,
            exact_hits: 1,
            ..Default::default()
        };

        let report = build_report(&atlas, &stats, 32, 32, None);
        assert_eq!(report.atlas.unique_sprites, 2);
        assert_eq!(report.atlas.total_references, 3);
        assert_eq!(report.atlas.used_pixels, 100 + 400);
        assert_eq!(report.interner.exact_hits, 1);

        let body = report.parts.iter().find(|p| p.part_id == "body").unwrap();
        assert_eq!(body.unique_sprites, 2);
        assert_eq!(body.total_references, 3);
        assert!((body.reuse_ratio - 1.5).abs() < 0.01);
    }

    #[test]
    fn bridge_markers_detected_by_shared_tags() {
        let atlas = make_atlas(
            vec![make_sprite("s0", 40, 30)],
            vec![
                make_part("wings_visual", "wing", Some("wings")),
                {
                    let mut p = make_part("wing_l", "wing_marker", None);
                    p.tags = vec!["left".to_string()];
                    p
                },
                {
                    let mut p = make_part("wing_r", "wing_marker", None);
                    p.tags = vec!["right".to_string()];
                    p
                },
            ],
            vec![
                PartDefinition {
                    id: "wing".to_string(),
                    tags: vec!["wing".to_string()],
                    gameplay: Default::default(),
                },
                PartDefinition {
                    id: "wing_marker".to_string(),
                    tags: vec!["wing".to_string()],
                    gameplay: Default::default(),
                },
            ],
            vec![vec![make_pose("wings_visual", "s0")]],
        );

        let stats = InternerStats::default();
        let report = build_report(&atlas, &stats, 64, 64, None);

        let wings = report
            .parts
            .iter()
            .find(|p| p.part_id == "wings_visual")
            .unwrap();
        assert!(wings.bridge_markers.contains(&"wing_l".to_string()));
        assert!(wings.bridge_markers.contains(&"wing_r".to_string()));
    }

    #[test]
    fn non_visual_parts_have_no_symmetry_score() {
        let atlas = make_atlas(
            vec![],
            vec![make_part("wing_l", "wing_marker", None)],
            vec![PartDefinition {
                id: "wing_marker".to_string(),
                tags: vec![],
                gameplay: Default::default(),
            }],
            vec![],
        );

        let stats = InternerStats::default();
        let report = build_report(&atlas, &stats, 32, 32, None);

        let wing_l = report.parts.iter().find(|p| p.part_id == "wing_l").unwrap();
        assert!(wing_l.symmetry_score.is_none());
    }

    #[test]
    fn symmetry_score_detects_perfect_horizontal_mirror() {
        // Create a 4x2 image: left half = right half mirrored.
        //   [R G | G R]
        //   [B W | W B]
        let mut img = RgbaImage::new(4, 2);
        let red = image::Rgba([255, 0, 0, 255]);
        let green = image::Rgba([0, 255, 0, 255]);
        let blue = image::Rgba([0, 0, 255, 255]);
        let white = image::Rgba([255, 255, 255, 255]);
        img.put_pixel(0, 0, red);
        img.put_pixel(1, 0, green);
        img.put_pixel(2, 0, green);
        img.put_pixel(3, 0, red);
        img.put_pixel(0, 1, blue);
        img.put_pixel(1, 1, white);
        img.put_pixel(2, 1, white);
        img.put_pixel(3, 1, blue);

        let atlas = make_atlas(
            vec![make_sprite("s0", 4, 2)],
            vec![make_part("wings", "wing", Some("wings"))],
            vec![PartDefinition {
                id: "wing".to_string(),
                tags: vec![],
                gameplay: Default::default(),
            }],
            vec![vec![make_pose("wings", "s0")]],
        );

        let stats = InternerStats::default();
        let report = build_report(&atlas, &stats, 4, 2, Some(&img));

        let wings = report.parts.iter().find(|p| p.part_id == "wings").unwrap();
        assert!(
            (wings.symmetry_score.unwrap() - 1.0).abs() < 0.01,
            "Expected perfect symmetry, got {}",
            wings.symmetry_score.unwrap()
        );
    }

    #[test]
    fn symmetry_score_low_for_asymmetric_sprite() {
        // Create a 4x2 image with no symmetry.
        let mut img = RgbaImage::new(4, 2);
        let red = image::Rgba([255, 0, 0, 255]);
        let black = image::Rgba([0, 0, 0, 255]);
        img.put_pixel(0, 0, red);
        img.put_pixel(1, 0, red);
        img.put_pixel(2, 0, black);
        img.put_pixel(3, 0, black);
        img.put_pixel(0, 1, red);
        img.put_pixel(1, 1, red);
        img.put_pixel(2, 1, black);
        img.put_pixel(3, 1, black);

        let atlas = make_atlas(
            vec![make_sprite("s0", 4, 2)],
            vec![make_part("body", "body", Some("body"))],
            vec![PartDefinition {
                id: "body".to_string(),
                tags: vec![],
                gameplay: Default::default(),
            }],
            vec![vec![make_pose("body", "s0")]],
        );

        let stats = InternerStats::default();
        let report = build_report(&atlas, &stats, 4, 2, Some(&img));

        let body = report.parts.iter().find(|p| p.part_id == "body").unwrap();
        assert!(
            body.symmetry_score.unwrap() < 0.5,
            "Expected low symmetry, got {}",
            body.symmetry_score.unwrap()
        );
    }

    #[test]
    fn dedup_ratio_reflects_interner_efficiency() {
        let atlas = make_atlas(
            vec![make_sprite("s0", 10, 10)],
            vec![make_part("body", "body", Some("body"))],
            vec![PartDefinition {
                id: "body".to_string(),
                tags: vec![],
                gameplay: Default::default(),
            }],
            vec![
                vec![make_pose("body", "s0")],
                vec![make_pose("body", "s0")],
                vec![make_pose("body", "s0")],
                vec![make_pose("body", "s0")],
            ],
        );

        let stats = InternerStats {
            total_interns: 4,
            new_sprites: 1,
            exact_hits: 3,
            ..Default::default()
        };

        let report = build_report(&atlas, &stats, 16, 16, None);
        // 1 unique sprite, 4 references → 75% dedup ratio.
        assert!((report.atlas.dedup_ratio - 0.75).abs() < 0.01);
    }

    #[test]
    fn source_region_parts_are_treated_as_visual() {
        let mut region_part = make_part("wing_l", "wing", None);
        region_part.source_region = Some(SourceRegion {
            layer: "wings".to_string(),
            half: SplitHalf::Left,
        });
        let atlas = make_atlas(
            vec![make_sprite("s0", 8, 4)],
            vec![region_part],
            vec![PartDefinition {
                id: "wing".to_string(),
                tags: vec![],
                gameplay: Default::default(),
            }],
            vec![vec![make_pose("wing_l", "s0")]],
        );

        let report = build_report(&atlas, &InternerStats::default(), 16, 16, None);
        let wing = report
            .parts
            .iter()
            .find(|part| part.part_id == "wing_l")
            .unwrap();
        assert!(
            wing.is_visual,
            "source_region parts should remain visible to analysis"
        );
    }

    #[test]
    fn report_separates_hit_rate_and_fragment_growth() {
        let atlas = make_atlas(
            vec![make_sprite("s0", 6, 6)],
            vec![make_part("body", "body", Some("body"))],
            vec![PartDefinition {
                id: "body".to_string(),
                tags: vec![],
                gameplay: Default::default(),
            }],
            vec![vec![
                make_pose("body", "s0"),
                PartPose {
                    fragment: 1,
                    ..make_pose("body", "s0")
                },
            ]],
        );
        let stats = InternerStats {
            total_interns: 2,
            new_sprites: 1,
            exact_hits: 1,
            ..Default::default()
        };

        let report = build_report(&atlas, &stats, 16, 16, None);
        assert_eq!(report.atlas.total_references, 2);
        assert_eq!(report.atlas.logical_part_references, 1);
        assert!((report.interner.hit_rate - 0.5).abs() < 0.01);
        assert!((report.atlas.unique_sprite_reference_ratio - 0.5).abs() < 0.01);
        assert!((report.atlas.fragment_growth_ratio - 1.0).abs() < 0.01);
    }
}
