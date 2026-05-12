use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use carcinisation_admin::{AdminRequest, AdminResponse, socket_path_for};
use clap::Parser;

#[derive(Parser)]
#[command(about = "Admin CLI for the carcinisation multiplayer server")]
struct Cli {
    /// Instance name (e.g. "deathmatch", "coop", "sandbox").
    instance: String,

    /// Admin command to run.
    command: String,

    /// Additional arguments (used by commands like `say`).
    args: Vec<String>,

    /// Override the admin socket path instead of deriving from instance name.
    #[arg(long)]
    socket: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let socket_path = cli.socket.unwrap_or_else(|| socket_path_for(&cli.instance));

    let request = match cli.command.as_str() {
        "help" => AdminRequest::Help,
        "status" => AdminRequest::Status,
        "players" => AdminRequest::Players,
        "say" => {
            let message = cli.args.join(" ");
            if message.is_empty() {
                eprintln!("usage: carcinisationctl <instance> say <message>");
                process::exit(2);
            }
            AdminRequest::Say { message }
        }
        "restart" => AdminRequest::Restart,
        "reset-map" => AdminRequest::ResetMap,
        "shutdown" => AdminRequest::Shutdown,
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("run `carcinisationctl <instance> help` to list commands");
            process::exit(2);
        }
    };

    let mut stream = match UnixStream::connect(&socket_path) {
        Ok(s) => s,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    eprintln!(
                        "socket not found: {}\nIs the '{}' instance running?",
                        socket_path.display(),
                        cli.instance
                    );
                }
                std::io::ErrorKind::PermissionDenied => {
                    eprintln!(
                        "permission denied: {}\nTry: sudo -u carcinisation carcinisationctl {} {}",
                        socket_path.display(),
                        cli.instance,
                        cli.command
                    );
                }
                _ => {
                    eprintln!("failed to connect to {}: {e}", socket_path.display());
                }
            }
            process::exit(1);
        }
    };

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .expect("set write timeout");

    let payload = serde_json::to_string(&request).expect("serialize request");
    if let Err(e) = writeln!(stream, "{payload}") {
        eprintln!("failed to send command: {e}");
        process::exit(1);
    }

    // Signal we're done writing so the server sees EOF if it reads again.
    let _ = stream.shutdown(std::net::Shutdown::Write);

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    if let Err(e) = reader.read_line(&mut line) {
        eprintln!("failed to read response: {e}");
        process::exit(1);
    }

    let response: AdminResponse = match serde_json::from_str(line.trim()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("invalid response from server: {e}");
            eprintln!("raw: {line}");
            process::exit(1);
        }
    };

    if response.ok {
        if let Some(msg) = &response.message {
            println!("{msg}");
        }
        if let Some(data) = &response.data {
            println!("{}", serde_json::to_string_pretty(data).unwrap());
        }
    } else {
        if let Some(err) = &response.error {
            eprintln!("error: {err}");
        }
        process::exit(1);
    }
}
