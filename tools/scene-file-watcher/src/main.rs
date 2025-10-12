use carcinisation::cutscene::data::CutsceneData;
use colored::*;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use ron::de::from_str;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEBOUNCE_DURATION: Duration = Duration::from_secs(1);

fn find_assets_root() -> PathBuf {
    let mut dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().expect("unable to determine current dir"));

    loop {
        let candidate = dir.join("assets");
        if candidate.exists() {
            return candidate;
        }

        if !dir.pop() {
            panic!(
                "Unable to locate an `assets` directory relative to `{}`",
                env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| dir.display().to_string())
            );
        }
    }
}

fn main() -> notify::Result<()> {
    let (tx, rx) = channel();

    // Create a watcher object, delivering events.
    let mut watcher: RecommendedWatcher = Watcher::new(
        tx,
        Config::default()
            .with_compare_contents(true)
            .with_poll_interval(Duration::from_secs(1)),
    )?;

    // Define the path to watch.
    let assets_root = find_assets_root();
    let path_to_watch: &Path = assets_root.as_ref();
    watcher.watch(path_to_watch, RecursiveMode::Recursive)?;

    println!("Watching {:?} for changes...", path_to_watch);

    let last_processed: Arc<Mutex<HashMap<PathBuf, Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        match rx.recv() {
            Ok(event) => {
                let last_processed = Arc::clone(&last_processed);
                handle_result(event, last_processed)
            }
            Err(e) => println!("{} {}", "Watch error:".red(), format!("{:?}", e)),
        }
    }
}

// Function to handle the result of the event
fn handle_result(
    event: Result<Event, notify::Error>,
    last_processed: Arc<Mutex<HashMap<PathBuf, Instant>>>,
) {
    match event {
        Ok(event) => handle_event(event, last_processed),
        Err(e) => println!("{} {}", "Watch error:".red(), format!("{:?}", e)),
    }
}

// Function to handle changes in RON files
fn handle_event(event: Event, last_processed: Arc<Mutex<HashMap<PathBuf, Instant>>>) {
    let mut last_processed = last_processed.lock().unwrap();

    for path in event.paths {
        if path.extension().map_or(false, |ext| ext == "ron") {
            let now = Instant::now();
            if let Some(&last_time) = last_processed.get(&path) {
                if now.duration_since(last_time) < DEBOUNCE_DURATION {
                    // Skip this event as it's within the debounce interval
                    continue;
                }
            }
            last_processed.insert(path.clone(), now);

            println!("Change detected in: {}", path.display().to_string().cyan());
            let data = fs::read_to_string(&path);
            match data {
                Ok(content) => match from_str::<CutsceneData>(&content) {
                    Ok(_) => println!("{} {}", "SUCCESSFULLY".green().bold(), "parsed RON file."),
                    Err(err) => {
                        println!("{} {}", "FAILED".red().bold(), "to parse RON:");
                        println!("{}", format!("{:?}", err));
                    }
                },
                Err(err) => {
                    println!(
                        "{} {}",
                        "FAILED".red().bold(),
                        format!("to read file: {:?}", err)
                    )
                }
            }
        }
    }
}
