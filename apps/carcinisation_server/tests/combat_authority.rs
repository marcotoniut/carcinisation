//! Authority and regression tests for combat edge cases.
//!
//! Networked client+server: dead-player fire, cooldown-on-death,
//! multi-player same-tick fresh-state hitscan.
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use std::net::SocketAddr;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use carcinisation_fps_core::map::{EntitySpawnData, EntitySpawnKind, test_map};
use carcinisation_net::{
    ClientIntent, InputSequence, NetAttackId, NetHealth, NetPlayer, PlayerActions, PlayerId,
    PlayerNetState,
};
use carcinisation_server::ServerPlugin;
use common::combat::{
    build_combat_client, build_combat_server, get_player_id, queue_fire, wait_for_player,
};
use common::{build_server_app, get_enemy_health, reserve_port, tick_with_sleep};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Dead player's fire commands should not deal damage (network-level test).
#[test]
fn dead_player_fire_does_not_damage_enemy() {
    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    let initial_hp = get_enemy_health(&mut server).unwrap();

    // Kill the player by setting health to 0, then tick to trigger death.
    {
        let mut q = server.world_mut().query::<(&NetPlayer, &mut NetHealth)>();
        for (_, mut h) in q.iter_mut(server.world_mut()) {
            h.current = 0.0;
        }
    }
    // 30 ticks at 2 ms ≈ 2 FixedUpdate cycles at 30 Hz — trigger death.
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    let state = server
        .world_mut()
        .query::<&NetPlayer>()
        .iter(server.world())
        .next()
        .map(|p| p.state.clone());
    assert!(
        matches!(state, Some(PlayerNetState::Dead)),
        "player should be dead, got {state:?}"
    );

    // Send fire commands while dead.
    let mut seq = 100;
    for _ in 0..10 {
        queue_fire(&mut client, seq);
        seq += 1;
        tick_with_sleep(&mut server, &mut client);
    }
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let hp_after = get_enemy_health(&mut server).unwrap();
    assert_eq!(
        initial_hp, hp_after,
        "dead player fire should not damage enemy: hp before={initial_hp}, after={hp_after}"
    );
}

/// Fire cooldown is cleared when a player dies, so respawned players start fresh.
#[test]
fn fire_cooldown_cleared_on_death() {
    use carcinisation_server::systems::FireCooldownMap;

    let port = reserve_port();
    let mut server = build_combat_server(port);
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_combat_client(addr);
    client.update();

    assert!(wait_for_player(&mut server, &mut client));

    // Fire once to set a cooldown.
    queue_fire(&mut client, 1);
    for _ in 0..20 {
        tick_with_sleep(&mut server, &mut client);
    }

    let pid = get_player_id(&mut server).expect("player should exist");

    let has_cd = server
        .world()
        .resource::<FireCooldownMap>()
        .0
        .contains_key(&pid);
    assert!(has_cd, "cooldown should exist after firing");

    // Kill the player.
    {
        let mut q = server.world_mut().query::<(&NetPlayer, &mut NetHealth)>();
        for (_, mut h) in q.iter_mut(server.world_mut()) {
            h.current = 0.0;
        }
    }
    // 30 ticks — trigger death.
    for _ in 0..30 {
        tick_with_sleep(&mut server, &mut client);
    }

    let cleared = !server
        .world()
        .resource::<FireCooldownMap>()
        .0
        .contains_key(&pid);
    assert!(
        cleared,
        "FireCooldownMap should be cleared after player death"
    );
}

/// P2 regression: two players fire during the same tick at enemies in a line.
/// Player 1 kills the front enemy. Player 2's hitscan must see fresh state.
#[test]
fn two_players_same_tick_second_sees_fresh_enemy_state() {
    use carcinisation_server::systems::PlayerIntentBuffer;

    let port = reserve_port();

    let entities = vec![
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 1,
                speed: 0.0,
            },
            x: 3.5,
            y: 1.5,
        },
        EntitySpawnData {
            kind: EntitySpawnKind::Mosquiton {
                health: 100,
                speed: 0.0,
            },
            x: 5.5,
            y: 1.5,
        },
    ];
    let mut server = build_server_app(ServerPlugin {
        port,
        map: test_map(),
        entities,
        player_starts: vec![],
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    });
    server.update();

    // Both players at the same position facing east.
    for pid in [1u32, 2] {
        server.world_mut().spawn((
            NetPlayer {
                player_id: PlayerId(pid),
                position: Vec2::new(1.5, 1.5),
                angle: 0.0,
                current_attack: NetAttackId::None,
                state: PlayerNetState::Alive,
                flame_active: false,
                avatar_palette_variant: None,
            },
            NetHealth {
                current: 100.0,
                max: 100.0,
            },
            Replicated,
        ));
    }

    // Inject fire_held for both players.
    {
        let mut buf = server.world_mut().resource_mut::<PlayerIntentBuffer>();
        for pid in [1u32, 2] {
            buf.set(
                PlayerId(pid),
                &ClientIntent {
                    sequence: InputSequence(1),
                    movement: Vec2::ZERO,
                    turn: 0.0,
                    fire_held: true,
                    actions: PlayerActions::default(),
                },
            );
        }
    }

    // 50 ticks at 2 ms ≈ 3 FixedUpdate cycles at 30 Hz — enough for hitscan.
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        server.update();
    }

    // Collect enemy health values.
    let mut healths: Vec<(f32, f32)> = server
        .world_mut()
        .query::<(&carcinisation_net::components::NetEnemy, &NetHealth)>()
        .iter(server.world())
        .map(|(e, h)| (e.position.x, h.current))
        .collect();
    healths.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    assert!(
        healths.len() >= 2,
        "both enemies should still exist, got {healths:?}"
    );

    let front_hp = healths[0].1;
    let back_hp = healths[1].1;

    assert!(front_hp <= 0.0, "front enemy should be dead: hp={front_hp}");
    assert!(
        back_hp < 100.0,
        "back enemy should be hit by player 2 (fresh snapshot): hp={back_hp}. \
         If 100, the enemy list was stale and player 2 wasted a shot on the dead front enemy."
    );
}
