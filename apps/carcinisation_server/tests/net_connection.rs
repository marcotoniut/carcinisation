mod common;

use std::net::SocketAddr;

use bevy_replicon::prelude::*;
use carcinisation_net::{NetProtocolPlugin, PlayerId, register_net_all};
use carcinisation_server::systems::{FlameActiveTracker, FlameCharCooldowns};
use common::{
    assert_server_resources, build_client_app, build_server_app, reserve_port, test_server_plugin,
    update_both, wait_for,
};

/// Server boots, client connects and stays connected, server receives
/// `ConnectedClient` entity. (Consolidates the former boot / connect /
/// multi-frame / event tests into one end-to-end flow.)
#[test]
fn server_boots_client_connects_and_event_fires() {
    let port = reserve_port();
    let mut server = build_server_app(test_server_plugin(port));
    server.update();
    assert_server_resources(&server);

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_client_app(NetProtocolPlugin, register_net_all, addr);
    client.update();

    // Server should receive a ConnectedClient entity within 120 frames.
    let connected = wait_for(120, &mut server, &mut client, |server, _client| {
        server
            .world_mut()
            .query::<&ConnectedClient>()
            .iter(server.world())
            .count()
            > 0
    });

    assert!(
        connected,
        "server should have a ConnectedClient entity within 120 frames"
    );

    // Client should still be connected after additional frames.
    for _ in 0..30 {
        update_both(&mut server, &mut client);
    }
    common::assert_client_connected(&client, "post-connect stability");
}

#[test]
fn client_graceful_disconnect() {
    let port = reserve_port();
    let mut server = build_server_app(test_server_plugin(port));
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_client_app(NetProtocolPlugin, register_net_all, addr);
    client.update();

    // Get client connected
    let connected = wait_for(120, &mut server, &mut client, |server, _client| {
        server
            .world_mut()
            .query::<&ConnectedClient>()
            .iter(server.world())
            .count()
            > 0
    });
    assert!(connected, "client should connect first");

    // Explicitly disconnect the client transport
    client
        .world_mut()
        .resource_mut::<bevy_renet2::netcode::NetcodeClientTransport>()
        .disconnect();

    // Run both sides to propagate disconnect
    let disconnected = wait_for(60, &mut server, &mut client, |server, _| {
        server
            .world_mut()
            .query::<&ConnectedClient>()
            .iter(server.world())
            .count()
            == 0
    });

    assert!(
        disconnected,
        "server should clean up ConnectedClient after client disconnects"
    );
}

#[test]
fn disconnect_cleans_up_flame_state_resources() {
    let port = reserve_port();
    let mut server = build_server_app(test_server_plugin(port));
    server.update();

    let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let mut client = build_client_app(NetProtocolPlugin, register_net_all, addr);
    client.update();

    let connected = wait_for(120, &mut server, &mut client, |server, _| {
        server
            .world_mut()
            .query::<&ConnectedClient>()
            .iter(server.world())
            .count()
            > 0
    });
    assert!(connected, "client should connect");

    // Seed per-player flame resources as if the player was actively flaming.
    let pid = PlayerId(1);
    server
        .world_mut()
        .resource_mut::<FlameCharCooldowns>()
        .0
        .insert(pid, 0.05);
    server
        .world_mut()
        .resource_mut::<FlameActiveTracker>()
        .0
        .insert(pid, true);

    // Disconnect the client.
    client
        .world_mut()
        .resource_mut::<bevy_renet2::netcode::NetcodeClientTransport>()
        .disconnect();

    let cleaned = wait_for(60, &mut server, &mut client, |server, _| {
        let char_cd = server.world().resource::<FlameCharCooldowns>();
        let flame = server.world().resource::<FlameActiveTracker>();
        char_cd.0.is_empty() && flame.0.is_empty()
    });
    assert!(
        cleaned,
        "FlameCharCooldowns and FlameActiveTracker should be empty after disconnect"
    );
}
