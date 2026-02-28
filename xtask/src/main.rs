use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use cargo_metadata::MetadataCommand;
use serde::Serialize;

const DEFAULT_PORT: u16 = 15702;
const BRP_POLL_INTERVAL: Duration = Duration::from_millis(100);
const BRP_POLL_TIMEOUT_DEFAULT: Duration = Duration::from_secs(300);
const SCREENSHOT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const SCREENSHOT_TIMEOUT: Duration = Duration::from_secs(10);
const EXCLUDED: &[&str] = &["brp_screenshot"];

#[derive(Serialize)]
struct JsonRpc<'a, P: Serialize> {
    jsonrpc: &'a str,
    id: u32,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<P>,
}

#[derive(Serialize)]
struct ScreenshotParams<'a> {
    path: &'a str,
}

struct Example {
    name: String,
    features: Vec<String>,
}

/// Reads example targets and their `required-features` from `Cargo.toml` via `cargo_metadata`.
fn discover_examples() -> Vec<Example> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata failed");

    let pkg = metadata
        .packages
        .iter()
        .find(|p| p.name == "carapace")
        .expect("carapace package not found");

    let mut examples: Vec<Example> = pkg
        .targets
        .iter()
        .filter(|t| t.is_example())
        .filter(|t| !EXCLUDED.contains(&t.name.as_str()))
        .map(|t| Example {
            features: {
                let mut features = t.required_features.clone();
                features.sort();
                features.dedup();
                features
            },
            name: t.name.clone(),
        })
        .collect();

    examples.sort_by(|a, b| a.name.cmp(&b.name));
    examples
}

fn sort_examples_by_feature_set(examples: &mut [&Example]) {
    examples.sort_by(|a, b| {
        a.features
            .cmp(&b.features)
            .then_with(|| a.name.cmp(&b.name))
    });
}

fn feature_set_label(example: &Example) -> String {
    if example.features.is_empty() {
        "default (+brp_extras)".to_string()
    } else {
        format!("{} (+brp_extras)", example.features.join(","))
    }
}

/// Returns the BRP poll timeout, overridable via `XTASK_BRP_TIMEOUT_SECS`.
fn brp_poll_timeout() -> Duration {
    match std::env::var("XTASK_BRP_TIMEOUT_SECS") {
        Ok(secs) => secs
            .parse::<u64>()
            .ok()
            .map(Duration::from_secs)
            .unwrap_or(BRP_POLL_TIMEOUT_DEFAULT),
        Err(_) => BRP_POLL_TIMEOUT_DEFAULT,
    }
}

/// Polls `rpc.discover` until the BRP endpoint responds or the child exits early.
fn wait_for_brp(child: &mut Child, port: u16) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/jsonrpc");
    let start = Instant::now();
    let timeout = brp_poll_timeout();

    let body = JsonRpc {
        jsonrpc: "2.0",
        id: 1,
        method: "rpc.discover",
        params: None::<()>,
    };

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("failed to check child status: {e}"))?
        {
            return Err(format!("example exited before BRP was ready: {status}"));
        }

        if start.elapsed() > timeout {
            return Err(format!(
                "BRP on port {port} not ready within {}s",
                timeout.as_secs()
            ));
        }

        if ureq::post(&url).send_json(&body).is_ok() {
            return Ok(());
        }

        std::thread::sleep(BRP_POLL_INTERVAL);
    }
}

/// Sends `brp_extras/screenshot` and atomically moves the result to `output_path`.
fn take_screenshot(port: u16, output_path: &str) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/jsonrpc");
    let pending_path = output_path.replace(".png", ".pending.png");

    if std::path::Path::new(&pending_path).exists() {
        std::fs::remove_file(&pending_path)
            .map_err(|e| format!("failed to clean pending screenshot '{pending_path}': {e}"))?;
    }

    let body = JsonRpc {
        jsonrpc: "2.0",
        id: 1,
        method: "brp_extras/screenshot",
        params: Some(ScreenshotParams {
            path: &pending_path,
        }),
    };

    ureq::post(&url)
        .send_json(&body)
        .map_err(|e| format!("screenshot request failed: {e}"))?;

    let start = Instant::now();
    loop {
        if start.elapsed() > SCREENSHOT_TIMEOUT {
            return Err(format!(
                "screenshot not created within {}s: {pending_path}",
                SCREENSHOT_TIMEOUT.as_secs()
            ));
        }
        if let Ok(meta) = std::fs::metadata(&pending_path) {
            if meta.len() > 0 {
                break;
            }
        }
        std::thread::sleep(SCREENSHOT_POLL_INTERVAL);
    }

    std::fs::rename(&pending_path, output_path).map_err(|e| {
        format!("failed to move screenshot '{pending_path}' to '{output_path}': {e}")
    })?;

    Ok(())
}

/// Launches `cargo run --example` with `brp_extras` and any extra required features.
fn spawn_example(example: &Example, port: u16) -> Child {
    let mut features = vec!["brp_extras".to_string()];
    features.extend(example.features.iter().cloned());

    Command::new("cargo")
        .args([
            "run",
            "--example",
            &example.name,
            "--features",
            &features.join(","),
        ])
        .env("BRP_EXTRAS_PORT", port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn '{}': {e}", example.name))
}

fn kill(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Spawns one example, waits for BRP, takes a screenshot, and cleans up.
fn capture_one(example: &Example, port: u16) -> Result<(), String> {
    let output_path = format!("screenshots/examples/{}.png", example.name);
    std::fs::create_dir_all("screenshots/examples")
        .map_err(|e| format!("create output dir: {e}"))?;

    eprintln!("  launching {} (port {port})", example.name);
    let mut child = spawn_example(example, port);

    let result = wait_for_brp(&mut child, port).and_then(|()| take_screenshot(port, &output_path));

    kill(&mut child);

    match &result {
        Ok(()) => eprintln!("  captured: {output_path}"),
        Err(e) => eprintln!("  FAILED: {e}"),
    }

    result
}

/// Captures screenshots for all examples, or a single one if named.
fn cmd_screenshots(args: &[String]) {
    let examples = discover_examples();

    let mut targets: Vec<&Example> = match args.first() {
        Some(name) => match examples.iter().find(|e| e.name == *name) {
            Some(ex) => vec![ex],
            None => {
                eprintln!("unknown example: {name}");
                eprintln!("available:");
                for ex in &examples {
                    eprintln!("  {}", ex.name);
                }
                std::process::exit(1);
            }
        },
        None => examples.iter().collect(),
    };

    if args.is_empty() {
        sort_examples_by_feature_set(&mut targets);
    }

    eprintln!("capturing {} screenshot(s)", targets.len());

    let mut failures = Vec::new();
    let mut active_feature_set: Option<String> = None;
    for (i, example) in targets.iter().enumerate() {
        let port = DEFAULT_PORT + i as u16;
        let feature_set = feature_set_label(example);
        if active_feature_set.as_deref() != Some(feature_set.as_str()) {
            eprintln!();
            eprintln!("feature set: {feature_set}");
            active_feature_set = Some(feature_set);
        }
        if let Err(e) = capture_one(example, port) {
            failures.push((example.name.clone(), e));
        }
    }

    eprintln!();
    if failures.is_empty() {
        eprintln!("all {} screenshots captured", targets.len());
    } else {
        eprintln!("{} of {} failed:", failures.len(), targets.len());
        for (name, err) in &failures {
            eprintln!("  {name}: {err}");
        }
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("screenshots") => cmd_screenshots(&args[1..]),
        Some("-h" | "--help") | None => {
            eprintln!("Usage:");
            eprintln!("  cargo xtask screenshots          capture all examples");
            eprintln!("  cargo xtask screenshots <name>   capture a single example");
        }
        Some(other) => {
            eprintln!("unknown subcommand: {other}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Example, sort_examples_by_feature_set};

    #[test]
    fn sort_examples_groups_by_required_features_then_name() {
        let examples = vec![
            Example {
                name: "zeta".to_string(),
                features: vec!["line".to_string()],
            },
            Example {
                name: "alpha".to_string(),
                features: vec![],
            },
            Example {
                name: "beta".to_string(),
                features: vec!["gpu_palette".to_string()],
            },
            Example {
                name: "gamma".to_string(),
                features: vec![],
            },
            Example {
                name: "delta".to_string(),
                features: vec!["gpu_palette".to_string()],
            },
        ];
        let mut targets: Vec<&Example> = examples.iter().collect();
        sort_examples_by_feature_set(&mut targets);

        let ordered_names: Vec<&str> = targets.iter().map(|ex| ex.name.as_str()).collect();
        assert_eq!(
            ordered_names,
            vec!["alpha", "gamma", "beta", "delta", "zeta"]
        );
    }
}
