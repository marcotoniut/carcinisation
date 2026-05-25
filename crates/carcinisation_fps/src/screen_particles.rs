//! Screen-space framebuffer particles for first-person presentation effects.

use bevy::prelude::*;
use carapace::image::CxImage;
use carcinisation_fps_core::ScreenParticleConfig;

use crate::render::BAYER_4X4;

#[derive(Clone, Copy, Debug, PartialEq)]
struct FpsScreenParticle {
    x: f32,
    y: f32,
    vy: f32,
    age: f32,
    lifetime: f32,
    max_r: u8,
    speed_scale: f32,
    is_highlight: bool,
    highlight_window: f32,
    flicker_phase: f32,
}

/// Local screen-space particles drawn directly into the FPS framebuffer.
#[derive(Resource, Debug)]
pub struct FpsScreenParticles {
    particles: Vec<FpsScreenParticle>,
    rng: ScreenParticleRng,
}

impl Default for FpsScreenParticles {
    fn default() -> Self {
        Self {
            particles: Vec::with_capacity(128),
            rng: ScreenParticleRng::new(0xC4C1_5A71),
        }
    }
}

impl FpsScreenParticles {
    #[must_use]
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Spawn one SUBTLE health-restored burst scaled to the framebuffer.
    ///
    /// Particle count and behaviour are driven by `config`. When the internal
    /// buffer is full, the oldest particles are discarded to make room — FIFO
    /// eviction keeps recent particles visible.
    pub fn spawn_health_pickup_burst(
        &mut self,
        framebuffer_width: u32,
        framebuffer_height: u32,
        config: &ScreenParticleConfig,
    ) {
        let w = framebuffer_width.max(1) as f32;
        let h = framebuffer_height.max(1) as f32;
        let scale = (h / config.prototype_reference_height.get()).max(0.5);
        let anchor_x = w * config.spawn_anchor_x;
        let anchor_y = h * config.spawn_anchor_y;
        let min_dist_sq = (config.min_spawn_distance * scale).powi(2);
        let mut burst_positions: Vec<Vec2> = Vec::with_capacity(config.particle_count.get());

        for _ in 0..config.particle_count.get() {
            let tier = choose_health_tier(&mut self.rng, config);
            let radius = scaled_radius(tier, scale);
            let position = choose_spawn_position(
                &mut self.rng,
                &burst_positions,
                anchor_x,
                anchor_y,
                w,
                h,
                min_dist_sq,
                config,
            );
            burst_positions.push(position);

            let pop_mag = config.pop_impulse.get() * scale * (0.5 + self.rng.f32() * 0.5);
            let random_highlight = self.rng.f32() < config.highlight_chance;
            let is_highlight = tier.always_highlight || random_highlight;
            let highlight_window = tier.highlight_window;
            let lifetime = self
                .rng
                .range_f32(config.lifetime_min.get(), config.lifetime_max.get())
                * tier.life_scale;
            let flicker_phase = self.rng.range_f32(0.0, std::f32::consts::TAU);
            self.push_particle(
                FpsScreenParticle {
                    x: position.x,
                    y: position.y,
                    vy: -pop_mag * tier.speed_scale,
                    age: 0.0,
                    lifetime,
                    max_r: radius,
                    speed_scale: tier.speed_scale,
                    is_highlight,
                    highlight_window,
                    flicker_phase,
                },
                config,
            );
        }
    }

    pub fn update(&mut self, dt: f32, config: &ScreenParticleConfig) {
        let dt = dt.clamp(0.0, config.max_particle_dt.get());
        if dt == 0.0 {
            return;
        }
        let drag_mul = config.drag.powf(dt * 60.0);
        for particle in &mut self.particles {
            particle.vy *= drag_mul;
            particle.vy += config.upward_accel.get() * particle.speed_scale * dt;
            particle.y += particle.vy * dt;
            particle.age += dt;
        }
        self.particles.retain(|p| p.age < p.lifetime);
    }

    fn push_particle(&mut self, particle: FpsScreenParticle, config: &ScreenParticleConfig) {
        if self.particles.len() >= config.max_particles.get() {
            let overflow = self.particles.len() + 1 - config.max_particles.get();
            self.particles.drain(..overflow);
        }
        self.particles.push(particle);
    }
}

/// Tick local FPS screen particles.
pub fn update_fps_screen_particles(
    mut particles: ResMut<FpsScreenParticles>,
    time: Res<Time>,
    config: Res<ScreenParticleConfig>,
) {
    particles.update(time.delta_secs(), &config);
}

/// Draw active FPS screen particles directly into the palette-index framebuffer.
pub fn draw_fps_screen_particles(
    image: &mut CxImage,
    particles: &FpsScreenParticles,
    config: &ScreenParticleConfig,
) {
    let width = image.width() as i32;
    let height = image.height() as i32;
    let data = image.data_mut();

    for particle in &particles.particles {
        draw_particle(data, width, height, particle, config);
    }
}

fn draw_particle(
    data: &mut [u8],
    img_w: i32,
    img_h: i32,
    particle: &FpsScreenParticle,
    config: &ScreenParticleConfig,
) {
    let age_ratio = (particle.age / particle.lifetime).clamp(0.0, 1.0);
    let r_factor = if age_ratio < 0.25 {
        1.0
    } else {
        1.0 - (age_ratio - 0.25) * 1.15
    };
    let radius = (f32::from(particle.max_r) * r_factor.max(0.0)).round() as i32;
    if radius <= 0 {
        return;
    }

    let mut fade = if age_ratio > config.dither_fade_start {
        (age_ratio - config.dither_fade_start) / (1.0 - config.dither_fade_start)
    } else {
        0.0
    };
    if age_ratio > 0.7 {
        fade += 0.10 * (particle.age * 38.0 + particle.flicker_phase).sin();
    }
    let fade_threshold =
        (fade.clamp(0.0, 1.0) * 16.0 * config.dither_fade_strength.get()).clamp(0.0, 16.0) as u8;

    let cx = particle.x.floor() as i32;
    let cy = particle.y.floor() as i32;
    let r_y = radius;
    let r_x = ((radius as f32) * config.diamond_aspect.get())
        .round()
        .max(1.0) as i32;
    let (core, edge) = if particle.is_highlight && age_ratio < particle.highlight_window {
        (config.palette_highlight, config.palette_light)
    } else {
        (config.palette_light, config.palette_light)
    };

    let y_min = (cy - r_y).max(0);
    let y_max = (cy + r_y).min(img_h - 1);
    let x_min = (cx - r_x).max(0);
    let x_max = (cx + r_x).min(img_w - 1);

    for py in y_min..=y_max {
        for px in x_min..=x_max {
            if BAYER_4X4[(py & 3) as usize][(px & 3) as usize] < fade_threshold {
                continue;
            }

            let dx = px - cx;
            let dy = py - cy;
            let n = (dx.abs() as f32 / r_x as f32) + (dy.abs() as f32 / r_y as f32);
            if n > 1.0 {
                continue;
            }
            let color = if n > 0.65 && radius >= 3 { edge } else { core };
            data[(py * img_w + px) as usize] = color;
        }
    }
}

/// Best-candidate anti-clustered spawn position near the vertical midline.
///
/// Sampling strategy:
/// 1. Draw a candidate with [`peripheral_offset`] — horizontal offset biased
///    toward the screen edges so particles cluster away from the centre, plus
///    vertical jitter from `spawn_area_h` so the distance computation separates
///    in both axes.
/// 2. If the candidate is farther than `min_dist_sq` from every existing particle,
///    accept immediately (early-out).
/// 3. Otherwise, track the best (farthest) candidate seen and retry up to
///    `spawn_rejection_attempts` times.
/// 4. After exhausting retries, fall back to the best candidate found.
fn choose_spawn_position(
    rng: &mut ScreenParticleRng,
    existing: &[Vec2],
    anchor_x: f32,
    anchor_y: f32,
    width: f32,
    height: f32,
    min_dist_sq: f32,
    config: &ScreenParticleConfig,
) -> Vec2 {
    let mut best = Vec2::new(anchor_x, anchor_y);
    let mut best_nearest = f32::NEG_INFINITY;
    let y_half_range = config.spawn_area_h * height;

    for _ in 0..config.spawn_rejection_attempts.get() {
        let y_jitter = rng.range_f32(-y_half_range, y_half_range);
        let candidate = Vec2::new(
            anchor_x + peripheral_offset(rng, width, config),
            anchor_y + y_jitter,
        );
        let nearest = nearest_distance_squared(candidate, existing);
        if nearest >= min_dist_sq {
            return candidate;
        }
        if nearest > best_nearest {
            best = candidate;
            best_nearest = nearest;
        }
    }

    best
}

fn peripheral_offset(
    rng: &mut ScreenParticleRng,
    width: f32,
    config: &ScreenParticleConfig,
) -> f32 {
    let r = rng.f32() - 0.5;
    let s = if r < 0.0 { -1.0 } else { 1.0 };
    let a = r.abs() * 2.0;
    s * a.powf(config.spawn_periphery_bias.get()) * width * 0.5
}

/// Minimum squared distance from `candidate` to any point in `existing`.
///
/// Returns [`f32::INFINITY`] when `existing` is empty (no proximity constraint).
fn nearest_distance_squared(candidate: Vec2, existing: &[Vec2]) -> f32 {
    let mut nearest = f32::INFINITY;
    for position in existing {
        let d = candidate.distance_squared(*position);
        if d < nearest {
            nearest = d;
        }
    }
    nearest
}

/// Pick a tier index by weighted random selection, returning the chosen tier.
fn choose_health_tier<'a>(
    rng: &mut ScreenParticleRng,
    config: &'a ScreenParticleConfig,
) -> &'a carcinisation_fps_core::SizeTierConfig {
    let total: f32 = config.size_tiers.iter().map(|t| t.weight).sum();
    let roll = rng.f32() * total;
    let mut cumulative = 0.0;
    for tier in &config.size_tiers {
        cumulative += tier.weight;
        if roll < cumulative {
            return tier;
        }
    }
    config.size_tiers.last()
}

/// Compute the scaled pixel radius for a selected size tier.
fn scaled_radius(tier: &carcinisation_fps_core::SizeTierConfig, scale: f32) -> u8 {
    ((tier.radius_px * scale).round() as u8).max(1)
}

#[derive(Clone, Copy, Debug)]
struct ScreenParticleRng {
    state: u32,
}

impl ScreenParticleRng {
    const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::UVec2;

    fn default_config() -> ScreenParticleConfig {
        ScreenParticleConfig::default()
    }

    fn count_particle_pixels(image: &CxImage, config: &ScreenParticleConfig) -> usize {
        image
            .data()
            .iter()
            .filter(|&&pixel| pixel == config.palette_light || pixel == config.palette_highlight)
            .count()
    }

    #[test]
    fn health_burst_spawns_exact_particle_count() {
        let config = default_config();
        let mut particles = FpsScreenParticles::default();
        particles.spawn_health_pickup_burst(160, 144, &config);
        assert_eq!(particles.len(), config.particle_count.get());
    }

    #[test]
    fn health_tier_selection_roughly_matches_weights() {
        let config = default_config();
        let mut rng = ScreenParticleRng::new(0x1234_5678);
        let mut counts = vec![0usize; config.size_tiers.len()];
        for _ in 0..10_000 {
            let tier = choose_health_tier(&mut rng, &config);
            let idx = config
                .size_tiers
                .iter()
                .position(|t| (t.radius_px - tier.radius_px).abs() < f32::EPSILON)
                .expect("tier exists");
            counts[idx] += 1;
        }

        // Small tier has weight 0.70 → ~7000
        assert!(
            (6800..7200).contains(&counts[0]),
            "small count={}",
            counts[0]
        );
        // Medium tier has weight 0.22 → ~2200
        assert!(
            (2000..2400).contains(&counts[1]),
            "medium count={}",
            counts[1]
        );
        // Large tier has weight 0.08 → ~800
        assert!(
            (600..1000).contains(&counts[2]),
            "large count={}",
            counts[2]
        );
    }

    #[test]
    fn health_burst_uses_one_tier_for_all_tier_dependent_values() {
        let mut config = default_config();
        config.particle_count = std::num::NonZeroUsize::new(64).unwrap();
        config.highlight_chance = 0.0;
        config.lifetime_min = carapace::constrained::PositiveFiniteF32::new(1.0).unwrap();
        config.lifetime_max = carapace::constrained::PositiveFiniteF32::new(1.0).unwrap();
        config.prototype_reference_height =
            carapace::constrained::PositiveFiniteF32::new(180.0).unwrap();
        config.size_tiers = vec1::vec1![
            carcinisation_fps_core::SizeTierConfig {
                radius_px: 2.0,
                speed_scale: 1.25,
                life_scale: 1.50,
                weight: 0.5,
                always_highlight: false,
                highlight_window: 0.25,
            },
            carcinisation_fps_core::SizeTierConfig {
                radius_px: 7.0,
                speed_scale: 0.40,
                life_scale: 0.60,
                weight: 0.5,
                always_highlight: true,
                highlight_window: 0.80,
            },
        ];

        let mut particles = FpsScreenParticles::default();
        particles.spawn_health_pickup_burst(320, 180, &config);

        assert!(
            particles.particles.iter().any(|p| p.max_r == 2),
            "test seed should select the small tier at least once"
        );
        assert!(
            particles.particles.iter().any(|p| p.max_r == 7),
            "test seed should select the large tier at least once"
        );

        for particle in &particles.particles {
            match particle.max_r {
                2 => {
                    assert!((particle.speed_scale - 1.25).abs() < f32::EPSILON);
                    assert!((particle.lifetime - 1.50).abs() < f32::EPSILON);
                    assert!(!particle.is_highlight);
                    assert!((particle.highlight_window - 0.25).abs() < f32::EPSILON);
                }
                7 => {
                    assert!((particle.speed_scale - 0.40).abs() < f32::EPSILON);
                    assert!((particle.lifetime - 0.60).abs() < f32::EPSILON);
                    assert!(particle.is_highlight);
                    assert!((particle.highlight_window - 0.80).abs() < f32::EPSILON);
                }
                other => panic!("unexpected radius {other}"),
            }
        }
    }

    #[test]
    fn health_burst_uses_scaled_anti_clustering() {
        let config = default_config();
        let mut particles = FpsScreenParticles::default();
        particles.spawn_health_pickup_burst(320, 180, &config);
        let min_dist_sq = config.min_spawn_distance.powi(2);
        let mut close_pairs = 0;
        let mut overlapping_pairs = 0;
        for i in 0..particles.particles.len() {
            for j in (i + 1)..particles.particles.len() {
                let a = Vec2::new(particles.particles[i].x, particles.particles[i].y);
                let b = Vec2::new(particles.particles[j].x, particles.particles[j].y);
                let distance_sq = a.distance_squared(b);
                if distance_sq < min_dist_sq {
                    close_pairs += 1;
                }
                if distance_sq < 1.0 {
                    overlapping_pairs += 1;
                }
            }
        }
        assert_eq!(overlapping_pairs, 0, "particles should not overlap exactly");
        assert!(
            close_pairs <= 2,
            "best-candidate rejection should keep close pairs rare; got {close_pairs}"
        );
    }

    #[test]
    fn particles_expire_after_lifetime() {
        let config = default_config();
        let mut particles = FpsScreenParticles::default();
        particles.spawn_health_pickup_burst(160, 144, &config);
        for _ in 0..40 {
            particles.update(0.05, &config);
        }
        assert!(particles.is_empty());
    }

    #[test]
    fn particle_cap_discards_oldest_particles() {
        let config = default_config();
        let mut particles = FpsScreenParticles::default();
        for _ in 0..10 {
            particles.spawn_health_pickup_burst(160, 144, &config);
        }
        assert_eq!(particles.len(), config.max_particles.get());
    }

    #[test]
    fn rasterisation_clips_to_framebuffer_edges() {
        let config = default_config();
        let mut image = CxImage::empty(UVec2::new(16, 16));
        let mut particles = FpsScreenParticles::default();
        particles.particles.push(FpsScreenParticle {
            x: -2.25,
            y: 2.0,
            vy: 0.0,
            age: 0.0,
            lifetime: 1.0,
            max_r: 9,
            speed_scale: 1.0,
            is_highlight: true,
            highlight_window: 0.65,
            flicker_phase: 0.0,
        });

        draw_fps_screen_particles(&mut image, &particles, &config);
        assert!(count_particle_pixels(&image, &config) > 0);
    }

    #[test]
    fn large_particle_near_corner_writes_only_valid_pixels() {
        let config = default_config();
        let mut image = CxImage::empty(UVec2::new(16, 16));
        let mut particles = FpsScreenParticles::default();
        // Large particle centred at (-4, -4) — most of the diamond is off-screen.
        particles.particles.push(FpsScreenParticle {
            x: -4.0,
            y: -4.0,
            vy: 0.0,
            age: 0.0,
            lifetime: 1.0,
            max_r: 9,
            speed_scale: 1.0,
            is_highlight: true,
            highlight_window: 0.65,
            flicker_phase: 0.0,
        });
        // Large particle centred at (20, 20) — past bottom-right corner.
        particles.particles.push(FpsScreenParticle {
            x: 20.0,
            y: 20.0,
            vy: 0.0,
            age: 0.0,
            lifetime: 1.0,
            max_r: 9,
            speed_scale: 1.0,
            is_highlight: true,
            highlight_window: 0.65,
            flicker_phase: 0.0,
        });
        // Large particle centred at (-4, 20) — past bottom-left corner.
        particles.particles.push(FpsScreenParticle {
            x: -4.0,
            y: 20.0,
            vy: 0.0,
            age: 0.0,
            lifetime: 1.0,
            max_r: 9,
            speed_scale: 1.0,
            is_highlight: true,
            highlight_window: 0.65,
            flicker_phase: 0.0,
        });

        // Must not panic, and only valid indices should be written.
        draw_fps_screen_particles(&mut image, &particles, &config);
        let data = image.data();
        assert!(
            data.iter().all(|&p| p <= config.palette_highlight),
            "no out-of-range palette indices"
        );
    }

    #[test]
    fn dither_and_shrink_reduce_pixel_coverage() {
        let config = default_config();
        let mut early = CxImage::empty(UVec2::new(32, 32));
        let mut late = CxImage::empty(UVec2::new(32, 32));
        let particle = FpsScreenParticle {
            x: 16.0,
            y: 16.0,
            vy: 0.0,
            age: 0.20,
            lifetime: 1.0,
            max_r: 9,
            speed_scale: 1.0,
            is_highlight: true,
            highlight_window: 0.65,
            flicker_phase: 0.0,
        };
        let mut particles = FpsScreenParticles::default();
        particles.particles.push(particle);
        draw_fps_screen_particles(&mut early, &particles, &config);

        particles.particles[0].age = 0.90;
        draw_fps_screen_particles(&mut late, &particles, &config);

        assert!(count_particle_pixels(&early, &config) > count_particle_pixels(&late, &config));
    }

    #[test]
    fn zero_radius_particles_are_not_drawn() {
        let config = default_config();
        let mut image = CxImage::empty(UVec2::new(16, 16));
        let mut particles = FpsScreenParticles::default();
        particles.particles.push(FpsScreenParticle {
            x: 8.0,
            y: 8.0,
            vy: 0.0,
            age: 0.99,
            lifetime: 1.0,
            max_r: 1,
            speed_scale: 1.0,
            is_highlight: false,
            highlight_window: 0.55,
            flicker_phase: 0.0,
        });
        draw_fps_screen_particles(&mut image, &particles, &config);
        assert_eq!(count_particle_pixels(&image, &config), 0);
    }

    #[test]
    fn vertical_jitter_spreads_particles() {
        let config = default_config();
        let mut particles = FpsScreenParticles::default();
        particles.spawn_health_pickup_burst(160, 144, &config);
        let y_positions: Vec<f32> = particles.particles.iter().map(|p| p.y).collect();
        let min_y = y_positions.iter().copied().fold(f32::INFINITY, f32::min);
        let max_y = y_positions
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let spread = max_y - min_y;
        assert!(
            spread > 0.5,
            "vertical jitter should spread particles; spread={spread}"
        );
    }
}
