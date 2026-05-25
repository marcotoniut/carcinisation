#![allow(dead_code, clippy::cast_possible_truncation)]

use std::net::SocketAddr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy_renet2::netcode::{NativeSocket, NetcodeClientTransport};
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::RenetChannelsExt;
use carcinisation_fps_core::map::test_map;
use carcinisation_server::ServerPlugin;

/// Create a `ServerPlugin` for tests using the hardcoded `test_map` (no entities).
pub fn test_server_plugin(port: u16) -> ServerPlugin {
    ServerPlugin {
        port,
        map: test_map(),
        entities: Vec::new(),
        player_starts: Vec::new(),
        admin_socket: None,
        instance_name: "test".to_string(),
        map_path: "test_map".to_string(),
    }
}

/// Build a server app for input/movement tests.
pub fn build_fixed_tick_server(port: u16) -> App {
    build_server_app(test_server_plugin(port))
}

/// Build a client app for input/movement tests.
pub fn build_fixed_tick_client(addr: SocketAddr) -> App {
    build_client_app(
        carcinisation_net::NetProtocolPlugin,
        carcinisation_net::register_net_all,
        addr,
    )
}

/// Tick server + client. A 2 ms sleep lets real time accumulate so that:
/// 1. Bevy's `FixedUpdate` fires once per ~16–17 ticks (30 Hz ≈ 33 ms period).
/// 2. UDP packets have wall-clock time to arrive before the server's next
///    `FixedUpdate` processes them.
///
/// NOTE: Do NOT replace this with `TimeUpdateStrategy::ManualDuration` on the
/// server. That makes `FixedUpdate` fire every `server.update()`, but the client
/// sends input in its `Update` phase (after `server.update()`), so packets arrive
/// 1–2 ticks too late and inputs are silently dropped.
pub fn tick_with_sleep(server: &mut App, client: &mut App) {
    std::thread::sleep(Duration::from_millis(2));
    server.update();
    client.update();
}

/// Tick server + two clients. See `tick_with_sleep` for why the sleep is needed.
pub fn tick3_with_sleep(server: &mut App, c1: &mut App, c2: &mut App) {
    std::thread::sleep(Duration::from_millis(2));
    server.update();
    c1.update();
    c2.update();
}
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient, RenetServer};

/// Per-process port base offset to avoid collisions under parallel test execution
/// (cargo-nextest). Each process gets a unique 20k-port range based on its PID.
static PORT_BASE: OnceLock<u16> = OnceLock::new();

/// Counter offset from `PORT_BASE` for sequential port allocation.
static NEXT_TEST_PORT: AtomicU16 = AtomicU16::new(0);

/// Reserve the next available test port, verifying the port is bindable.
/// Retries up to 64 times on bind failure (port in `TIME_WAIT` or already in use).
/// Port range per process: [25000 + (PID % 20000), 65535).
#[allow(clippy::single_match)]
pub fn reserve_port() -> u16 {
    let base =
        *PORT_BASE.get_or_init(|| 25000u16.wrapping_add((std::process::id() as u16) % 20000));
    for _ in 0..64 {
        let port = base.wrapping_add(NEXT_TEST_PORT.fetch_add(1, Ordering::SeqCst));
        let addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), port);
        if let Ok(_socket) = std::net::UdpSocket::bind(addr) {
            return port; // Socket drops, port is free for the test server.
        }
    }
    panic!("Could not find a free UDP port after 64 attempts");
}

/// Build a minimal headless server App.
pub fn build_server_app(server_plugin: impl Plugin + 'static) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(server_plugin);
    app.finish();
    app
}

/// Build a minimal headless client App.
pub fn build_client_app(
    net_protocol: impl Plugin + 'static,
    register_net: fn(&mut App),
    server_addr: SocketAddr,
) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));

    app.add_plugins(net_protocol)
        .add_plugins(RepliconSharedPlugin {
            auth_method: AuthMethod::None,
        })
        .add_plugins(bevy_replicon::prelude::ClientPlugin)
        .add_plugins(bevy_replicon::prelude::ClientMessagePlugin);

    register_net(&mut app);

    app.add_plugins(bevy_replicon_renet2::RepliconRenetPlugins)
        .add_systems(
            Startup,
            move |mut commands: Commands, channels: Res<RepliconChannels>| {
                let client_id = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time went backwards")
                    .as_millis() as u64;

                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time went backwards");

                let server_configs = channels.server_configs();
                let client_configs = channels.client_configs();
                let connection_config =
                    ConnectionConfig::from_channels(server_configs, client_configs);

                let local_addr =
                    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0);
                let socket =
                    NativeSocket::new(std::net::UdpSocket::bind(local_addr).expect("bind"))
                        .expect("create socket");

                let authentication = bevy_renet2::netcode::ClientAuthentication::Unsecure {
                    client_id,
                    protocol_id: carcinisation_net::PROTOCOL_ID,
                    socket_id: 0,
                    server_addr,
                    user_data: None,
                };

                let transport = NetcodeClientTransport::new(current_time, authentication, socket)
                    .expect("create client transport");

                let client = RenetClient::new(connection_config, transport.is_reliable());

                commands.insert_resource(client);
                commands.insert_resource(transport);
            },
        )
        .add_systems(
            PreUpdate,
            kickstart_client_transport
                .run_if(resource_added::<RenetClient>)
                .before(bevy_renet2::prelude::RenetReceive),
        );

    app.finish();
    app
}

#[allow(clippy::needless_pass_by_value)]
fn kickstart_client_transport(
    mut client: ResMut<RenetClient>,
    mut transport: ResMut<NetcodeClientTransport>,
    time: Res<Time<Real>>,
) {
    if let Err(e) = transport.update(time.delta(), &mut client) {
        trace!("CLIENT kickstart transport error: {:?}", e);
    }
}

/// Run one update cycle on both server and client.
pub fn update_both(server: &mut App, client: &mut App) {
    server.update();
    client.update();
}

/// Update server once, then update multiple clients.
#[allow(dead_code)]
pub fn update_server_and_clients(server: &mut App, clients: &mut [&mut App]) {
    server.update();
    for client in clients {
        client.update();
    }
}

/// Wait up to `max_frames` for a condition to become true.
pub fn wait_for<F>(max_frames: u32, server: &mut App, client: &mut App, mut condition: F) -> bool
where
    F: FnMut(&mut App, &mut App) -> bool,
{
    for _ in 0..max_frames {
        update_both(server, client);
        if condition(server, client) {
            return true;
        }
    }
    false
}

/// Check that `RenetServer` and `NetcodeServerTransport` exist.
#[allow(dead_code)]
pub fn assert_server_resources(app: &App) {
    assert!(
        app.world().get_resource::<RenetServer>().is_some(),
        "RenetServer resource should exist"
    );
    assert!(
        app.world()
            .get_resource::<bevy_renet2::netcode::NetcodeServerTransport>()
            .is_some(),
        "NetcodeServerTransport should exist"
    );
}

/// Check that client is connected or connecting.
#[allow(dead_code)]
pub fn assert_client_connected(app: &App, frame_label: &str) {
    let client_res = app
        .world()
        .get_resource::<RenetClient>()
        .expect("client should exist");
    assert!(
        client_res.is_connected() || client_res.is_connecting(),
        "{}: client unexpectedly disconnected (connected={}, connecting={}, disconnected={})",
        frame_label,
        client_res.is_connected(),
        client_res.is_connecting(),
        client_res.is_disconnected()
    );
}
