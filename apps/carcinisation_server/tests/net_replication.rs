mod common;

use std::collections::HashSet;
use std::net::SocketAddr;

use bevy::prelude::*;
use carcinisation_net::{NetPlayer, NetProtocolPlugin, PlayerNetState, register_net_all};
use common::{build_client_app, build_server_app, reserve_port, test_server_plugin};

#[test]
fn server_spawns_netplayer_on_connect() {
    let port = reserve_port();
    let mut server = build_server_app(test_server_plugin(port));
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_client_app(NetProtocolPlugin, register_net_all, addr);
    client.update();

    let got_player = wait_for_netplayer(200, &mut server, &mut client);
    assert!(
        got_player,
        "client should have received a replicated NetPlayer"
    );

    let net_player = get_first_netplayer(&mut client);
    assert!(matches!(net_player.state, PlayerNetState::Alive));
}

#[test]
fn server_netplayer_has_correct_position() {
    let port = reserve_port();
    let mut server = build_server_app(test_server_plugin(port));
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_client_app(NetProtocolPlugin, register_net_all, addr);
    client.update();

    let got_player = wait_for_netplayer(200, &mut server, &mut client);
    assert!(
        got_player,
        "client should have received a replicated NetPlayer"
    );

    let net_player = get_first_netplayer(&mut client);
    let expected = [
        Vec2::new(1.5, 1.5),
        Vec2::new(6.5, 1.5),
        Vec2::new(1.5, 6.5),
        Vec2::new(6.5, 6.5),
    ];
    assert!(
        expected.contains(&net_player.position),
        "position {:?} should be one of {:?}",
        net_player.position,
        expected
    );
}

#[test]
fn multiple_clients_get_different_player_ids() {
    let port = reserve_port();
    let mut server = build_server_app(test_server_plugin(port));
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client1 = build_client_app(NetProtocolPlugin, register_net_all, addr);
    client1.update();
    let mut client2 = build_client_app(NetProtocolPlugin, register_net_all, addr);
    client2.update();

    for _ in 0..200 {
        server.update();
        client1.update();
        client2.update();

        let p1_count = count_netplayers(&mut client1);
        let p2_count = count_netplayers(&mut client2);

        if p1_count >= 2 && p2_count >= 2 {
            let ids1: HashSet<_> = collect_player_ids(&mut client1);
            let ids2: HashSet<_> = collect_player_ids(&mut client2);

            assert_eq!(
                ids1, ids2,
                "both clients should see the same set of PlayerIds"
            );
            assert_eq!(ids1.len(), 2, "should have exactly 2 unique PlayerIds");
            return;
        }
    }

    panic!("both clients should have received 2 replicated NetPlayers");
}

fn wait_for_netplayer(max_frames: u32, server: &mut App, client: &mut App) -> bool {
    common::wait_for(max_frames, server, client, |_server, client| {
        count_netplayers(client) > 0
    })
}

fn get_first_netplayer(app: &mut App) -> NetPlayer {
    let mut query = app.world_mut().query::<&NetPlayer>();
    let net_player = query
        .iter(app.world())
        .next()
        .expect("NetPlayer should exist");
    net_player.clone()
}

fn count_netplayers(app: &mut App) -> usize {
    app.world_mut()
        .query::<&NetPlayer>()
        .iter(app.world())
        .count()
}

fn collect_player_ids(app: &mut App) -> HashSet<u32> {
    app.world_mut()
        .query::<&NetPlayer>()
        .iter(app.world())
        .map(|p| p.player_id.0)
        .collect()
}
