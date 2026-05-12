//! Local-only Unix domain socket admin interface.
//!
//! Each server instance binds a socket at a configurable path (default
//! `/run/carcinisation/<instance>.admin.sock`). The `carcinisationctl` CLI
//! connects, sends a single JSON-line request, reads the JSON-line response,
//! and disconnects. The socket is non-blocking and polled once per
//! `FixedUpdate` tick (~30 Hz).

use std::io::{BufRead, BufReader, Write as _};
use std::os::unix::net::UnixListener;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use carcinisation_admin::{AdminRequest, AdminResponse};
use carcinisation_net::{NetHealth, NetPlayer, TickCounter};

use super::NetEnemy;
use super::reset::MapResetRequested;

/// Server-side admin socket state.
#[derive(Resource)]
pub struct AdminSocketState {
    listener: UnixListener,
    started_at: Instant,
    instance_name: String,
    map_path: String,
}

/// Bind the admin socket. Removes a stale socket file if present.
///
/// # Panics
///
/// Panics if the socket cannot be bound (permissions, missing parent dir, etc.).
pub fn setup_admin_socket(
    socket_path: &str,
    instance_name: String,
    map_path: String,
) -> AdminSocketState {
    let path = std::path::Path::new(socket_path);

    // Remove stale socket from a previous run.
    if path.exists() {
        std::fs::remove_file(path).expect("failed to remove stale admin socket");
    }

    let listener = UnixListener::bind(path).unwrap_or_else(|e| {
        panic!("failed to bind admin socket at {socket_path}: {e}");
    });
    listener
        .set_nonblocking(true)
        .expect("failed to set admin socket non-blocking");

    // Best-effort: restrict socket to owner (carcinisation user).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660));
    }

    info!("Admin socket listening on {socket_path}");

    AdminSocketState {
        listener,
        started_at: Instant::now(),
        instance_name,
        map_path,
    }
}

/// Poll the admin socket for incoming commands. Runs in `FixedUpdate`.
#[allow(clippy::needless_pass_by_value)]
pub fn poll_admin_socket(
    admin: ResMut<AdminSocketState>,
    players: Query<(&NetPlayer, &NetHealth)>,
    enemies: Query<&NetEnemy>,
    tick_counter: Res<TickCounter>,
    server_port: Res<crate::ServerPort>,
    mut exit: MessageWriter<AppExit>,
    mut map_reset: ResMut<MapResetRequested>,
) {
    // Accept at most a few connections per tick to avoid stalling the game loop.
    for _ in 0..4 {
        let stream = match admin.listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => {
                warn!("admin socket accept error: {e}");
                break;
            }
        };

        // Per-connection: blocking with a short timeout.
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            let resp = AdminResponse::error("failed to read request");
            let _ = writeln!(
                &stream,
                "{}",
                serde_json::to_string(&resp).unwrap_or_default()
            );
            continue;
        }

        let exit_code;
        let response = match serde_json::from_str::<AdminRequest>(line.trim()) {
            Ok(req) => {
                info!("admin command: {req:?}");
                exit_code = match &req {
                    AdminRequest::Shutdown => Some(AppExit::Success),
                    AdminRequest::Restart => {
                        Some(AppExit::Error(std::num::NonZero::new(1).unwrap()))
                    }
                    AdminRequest::ResetMap => {
                        map_reset.0 = true;
                        None
                    }
                    _ => None,
                };
                handle_request(&admin, &players, &enemies, &tick_counter, &server_port, req)
            }
            Err(e) => {
                exit_code = None;
                AdminResponse::error(format!("invalid request: {e}"))
            }
        };

        let _ = writeln!(
            &stream,
            "{}",
            serde_json::to_string(&response).unwrap_or_default()
        );

        if let Some(code) = exit_code {
            let label = if matches!(code, AppExit::Success) {
                "shutdown"
            } else {
                "restart"
            };
            info!("admin {label} requested — exiting");
            exit.write(code);
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn handle_request(
    admin: &AdminSocketState,
    players: &Query<(&NetPlayer, &NetHealth)>,
    enemies: &Query<&NetEnemy>,
    tick_counter: &TickCounter,
    server_port: &crate::ServerPort,
    request: AdminRequest,
) -> AdminResponse {
    match request {
        AdminRequest::Help => AdminResponse::success(
            "Available commands: help, status, players, say <message>, restart, reset-map, shutdown",
        ),

        AdminRequest::Status => {
            let uptime_secs = admin.started_at.elapsed().as_secs();
            let hours = uptime_secs / 3600;
            let minutes = (uptime_secs % 3600) / 60;
            let secs = uptime_secs % 60;

            let player_count = players.iter().count();
            let enemy_count = enemies.iter().count();

            let data = serde_json::json!({
                "instance": admin.instance_name,
                "port": server_port.0,
                "map": admin.map_path,
                "uptime_seconds": uptime_secs,
                "uptime": format!("{hours}h {minutes}m {secs}s"),
                "tick": tick_counter.0.0,
                "players": player_count,
                "enemies": enemy_count,
            });

            AdminResponse::success_with_data(
                format!(
                    "{} | port {} | {} players | up {hours}h{minutes}m{secs}s",
                    admin.instance_name, server_port.0, player_count
                ),
                data,
            )
        }

        AdminRequest::Players => {
            let mut list = Vec::new();
            for (np, health) in players.iter() {
                list.push(serde_json::json!({
                    "player_id": np.player_id.0,
                    "state": format!("{:?}", np.state),
                    "health": format!("{}/{}", health.current, health.max),
                    "position": format!("({:.1}, {:.1})", np.position.x, np.position.y),
                }));
            }

            if list.is_empty() {
                AdminResponse::success("No players connected.")
            } else {
                AdminResponse::success_with_data(
                    format!("{} player(s) connected", list.len()),
                    serde_json::Value::Array(list),
                )
            }
        }

        AdminRequest::Say { .. } => {
            AdminResponse::error("say: not implemented — no in-game chat system yet")
        }

        AdminRequest::Restart => {
            AdminResponse::success("Restart acknowledged. Server exiting (systemd will restart).")
        }

        AdminRequest::ResetMap => AdminResponse::success(
            "Map reset requested. Enemies respawned, players reset to spawn points.",
        ),

        AdminRequest::Shutdown => AdminResponse::success("Shutdown acknowledged. Server exiting."),
    }
}

impl Drop for AdminSocketState {
    fn drop(&mut self) {
        // Best-effort: remove the socket file to avoid stale-socket errors
        // on fast restarts. The OS also cleans up on process exit.
        if let Ok(addr) = self.listener.local_addr()
            && let Some(path) = addr.as_pathname()
        {
            let _ = std::fs::remove_file(path);
        }
    }
}
