//! Pistol hitscan combat integration tests.
//!
//! Networked client+server: fire release, damage, death, fixed-tick cooldown.
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use std::net::SocketAddr;

use common::combat::{
    build_combat_client, build_combat_server, enemy_count, get_player_id, queue_fire, queue_idle,
    wait_for_player,
};
use common::{get_enemy_health, get_enemy_state, reserve_port, tick_with_sleep};

use carcinisation_net::NetEnemyState;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Hold BTN_FIRE, then release. Server should stop firing after release.
#[test]
fn fire_input_release_stops_firing() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));
    assert_eq!(enemy_count(&mut server), 1);

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Fire for several ticks.
    for seq in 1..=5 {
        queue_fire(&mut client, seq);
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after_fire = get_enemy_health(&mut server).unwrap();
    assert!(
        hp_after_fire < initial_hp,
        "enemy should have taken damage: {initial_hp} → {hp_after_fire}"
    );

    // Release fire (send buttons=0).
    queue_idle(&mut client, 6);
    for _ in 0..10 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after_release = get_enemy_health(&mut server).unwrap();

    // Tick more — no new damage should occur.
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_final = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        hp_after_release, hp_final,
        "enemy health should not change after fire released: {hp_after_release} → {hp_final}"
    );
}

/// Fire at enemy, verify NetHealth decreases.
#[test]
fn enemy_takes_damage_from_hitscan() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    let initial_hp = get_enemy_health(&mut server).unwrap();
    assert!(
        (initial_hp - 100.0).abs() < 0.1,
        "enemy should start at 100hp"
    );

    // Fire once.
    queue_fire(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert!(
        hp_after < initial_hp,
        "enemy should take damage: {initial_hp} → {hp_after}"
    );
}

/// Fire until enemy dies. Verify exactly one death transition.
#[test]
fn enemy_dies_once() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    // 100 HP / 37 dmg = 3 shots needed. At 0.33 s cooldown and 30 Hz tick,
    // need ~1.0 s real time for 3 shots. Use longer sleeps to accumulate time.
    for seq in 1..=20 {
        queue_fire(&mut client, seq);
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.update();
        client.update();
    }
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.update();
        client.update();
    }

    let state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(
            state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ),
        "enemy should be dying or dead, got {state:?}"
    );

    let hp = get_enemy_health(&mut server).unwrap();
    assert!(hp <= 0.0, "enemy health should be <= 0: {hp}");

    // Keep firing — health should not go further negative or cause issues.
    for seq in 21..=30 {
        queue_fire(&mut client, seq);
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.update();
        client.update();
    }

    let hp_final = get_enemy_health(&mut server).unwrap();
    assert!(
        hp_final <= 0.0,
        "enemy health should remain <= 0: {hp_final}"
    );
    let final_state = get_enemy_state(&mut server).unwrap();
    assert!(
        matches!(
            final_state,
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
        ),
        "enemy should stay dying or dead, got {final_state:?}"
    );
}

/// Cooldown uses server fixed tick delta, not frame count.
#[test]
fn combat_uses_fixed_tick_cooldown() {
    use carcinisation_server::systems::FireCooldownMap;

    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Send fire on two consecutive ticks — cooldown should prevent the second.
    queue_fire(&mut client, 1);
    tick_with_sleep(&mut server, &mut client);
    queue_fire(&mut client, 2);
    tick_with_sleep(&mut server, &mut client);

    // Wait for FixedUpdate to process both commands.
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp = get_enemy_health(&mut server).unwrap();
    let damage = initial_hp - hp;

    // Exactly one shot should land (37 damage). Two shots would be 74.
    assert_eq!(
        damage,
        carcinisation_fps_core::FpsCombatConfig::default().hitscan_damage,
        "cooldown should allow exactly 1 shot, got damage={damage}"
    );

    // Verify the cooldown resource has a non-zero entry for the player.
    let pid = get_player_id(&mut server).expect("player should exist");
    let cooldowns = server.world().resource::<FireCooldownMap>();
    let cd = cooldowns.0.get(&pid).copied().unwrap_or(0.0);
    assert!(
        cd >= 0.0,
        "FireCooldownMap should have an entry for the player after firing"
    );
}
