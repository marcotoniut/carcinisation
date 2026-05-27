//! Deterministic composed-animation cue contract tests.
//!
//! These cover the Mosquiton ranged-attack timing contract without depending on
//! full app startup, async asset loading, or transient message readers.

mod common;

use carcinisation_base::direction::SpriteDirection;

use std::{fs, path::PathBuf, time::Duration};

use asset_pipeline::aseprite::{AnimationEventKind, CompositionAtlas};
use carcinisation::stage::enemy::{
    composed::ComposedAnimationState,
    data::mosquiton::{ACTION_IDLE_FLY, ACTION_SHOOT_FLY},
    mosquito::{
        entity::{EnemyMosquitoAttack, EnemyMosquitoAttacking},
        systems::ENEMY_MOSQUITO_RANGED_PRESENTATION,
    },
    mosquiton::entity::EnemyMosquitonAnimation,
};
use common::{advance_stage, build_attack_timing_app, spawn_composed_mosquiton};

fn mosquiton_atlas_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/sprites/enemies/mosquiton_3/atlas.json")
}

fn load_mosquiton_atlas() -> CompositionAtlas {
    serde_json::from_str(
        &fs::read_to_string(mosquiton_atlas_path()).expect("mosquiton atlas should be readable"),
    )
    .expect("mosquiton atlas should deserialize")
}

fn shoot_fly_cue_elapsed(atlas: &CompositionAtlas) -> Duration {
    let shoot = atlas
        .animations
        .iter()
        .find(|animation| animation.tag == SpriteDirection::Front.tag_name(ACTION_SHOOT_FLY))
        .expect("shoot_fly animation should exist");

    let mut elapsed_ms = 0u64;
    for frame in &shoot.frames {
        if frame.events.iter().any(|event| {
            event.kind == AnimationEventKind::ProjectileSpawn && event.id == "blood_shot"
        }) {
            return Duration::from_millis(elapsed_ms);
        }

        elapsed_ms += u64::from(frame.duration_ms);
    }

    panic!("shoot_fly must author a blood_shot projectile cue");
}

#[test]
fn mosquiton_blood_shot_cue_is_authored_within_presentation_window() {
    let atlas = load_mosquiton_atlas();
    let cue_elapsed = shoot_fly_cue_elapsed(&atlas);

    assert!(
        cue_elapsed < ENEMY_MOSQUITO_RANGED_PRESENTATION,
        "blood_shot cue is authored at {cue_elapsed:?}, outside presentation window {ENEMY_MOSQUITO_RANGED_PRESENTATION:?}"
    );
}

#[test]
fn attack_state_persists_through_authored_cue_then_clears() {
    let atlas = load_mosquiton_atlas();
    let cue_elapsed = shoot_fly_cue_elapsed(&atlas);

    let mut app = build_attack_timing_app();
    let entity = spawn_composed_mosquiton(&mut app);

    app.update();

    advance_stage(&mut app, cue_elapsed);

    let attacking = app
        .world()
        .entity(entity)
        .get::<EnemyMosquitoAttacking>()
        .expect("attack component should exist");
    assert!(
        matches!(attacking.attack, Some(EnemyMosquitoAttack::Ranged)),
        "ranged attack should still be active when the authored cue fires"
    );

    advance_stage(
        &mut app,
        ENEMY_MOSQUITO_RANGED_PRESENTATION
            .checked_sub(cue_elapsed)
            .expect("cue must be within presentation")
            + Duration::from_millis(1),
    );

    let attacking_after = app
        .world()
        .entity(entity)
        .get::<EnemyMosquitoAttacking>()
        .expect("attack component should exist");
    assert!(
        attacking_after.attack.is_none(),
        "ranged attack should clear after the presentation window closes"
    );
}

#[test]
fn mosquiton_animation_cycles_through_attack_presentation() {
    let mut app = build_attack_timing_app();
    let entity = spawn_composed_mosquiton(&mut app);

    app.update();

    {
        let animation = app
            .world()
            .entity(entity)
            .get::<EnemyMosquitonAnimation>()
            .expect("animation should be assigned");
        let state = app
            .world()
            .entity(entity)
            .get::<ComposedAnimationState>()
            .expect("animation state should exist");

        assert_eq!(*animation, EnemyMosquitonAnimation::ShootFly);
        assert_eq!(
            state.requested_tag,
            SpriteDirection::Front.tag_name(ACTION_SHOOT_FLY)
        );
    }

    advance_stage(
        &mut app,
        ENEMY_MOSQUITO_RANGED_PRESENTATION + Duration::from_millis(1),
    );

    {
        let animation = app
            .world()
            .entity(entity)
            .get::<EnemyMosquitonAnimation>()
            .expect("animation should be assigned");
        let state = app
            .world()
            .entity(entity)
            .get::<ComposedAnimationState>()
            .expect("animation state should exist");

        assert_eq!(*animation, EnemyMosquitonAnimation::IdleFly);
        assert_eq!(
            state.requested_tag,
            SpriteDirection::Front.tag_name(ACTION_IDLE_FLY)
        );
    }
}
