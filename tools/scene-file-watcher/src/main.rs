use carcinisation::cutscene::data::CutsceneData;
use carcinisation::stage::data::StageData;
use colored::Colorize;
use notify::Watcher;
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
    let mut dir = env::var("CARGO_MANIFEST_DIR").map_or_else(
        |_| env::current_dir().expect("unable to determine current dir"),
        PathBuf::from,
    );

    loop {
        let candidate = dir.join("assets");
        if candidate.exists() {
            return candidate;
        }

        assert!(
            dir.pop(),
            "Unable to locate an `assets` directory relative to `{}`",
            env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| dir.display().to_string())
        );
    }
}

fn main() -> notify::Result<()> {
    let (tx, rx) = channel();

    // Create a watcher object, delivering events.
    let mut watcher: notify::RecommendedWatcher = notify::Watcher::new(
        tx,
        notify::Config::default()
            .with_compare_contents(true)
            .with_poll_interval(Duration::from_secs(1)),
    )?;

    // Define the path to watch.
    let assets_root = find_assets_root();
    let path_to_watch: &Path = assets_root.as_ref();
    watcher.watch(path_to_watch, notify::RecursiveMode::Recursive)?;

    println!("Watching {} for changes...", path_to_watch.display());

    let last_processed: Arc<Mutex<HashMap<PathBuf, Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        match rx.recv() {
            Ok(event) => {
                handle_result(event, &last_processed);
            }
            Err(e) => println!("{} {:?}", "Watch error:".red(), e),
        }
    }
}

// Function to handle the result of the event
fn handle_result(
    event: Result<notify::Event, notify::Error>,
    last_processed: &Arc<Mutex<HashMap<PathBuf, Instant>>>,
) {
    match event {
        Ok(event) => handle_event(event, last_processed),
        Err(e) => println!("{} {:?}", "Watch error:".red(), e),
    }
}

/// Determines the RON file kind from its path extension pattern.
enum RonKind {
    Cutscene,
    Stage,
}

fn classify_ron(path: &Path) -> Option<RonKind> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".cs.ron") {
        Some(RonKind::Cutscene)
    } else if name.ends_with(".sg.ron") {
        Some(RonKind::Stage)
    } else {
        None
    }
}

// Function to handle changes in RON files
fn handle_event(event: notify::Event, last_processed: &Arc<Mutex<HashMap<PathBuf, Instant>>>) {
    let mut last_processed = last_processed.lock().unwrap();

    for path in event.paths {
        if path.extension().is_some_and(|ext| ext == "ron") {
            let now = Instant::now();
            if let Some(&last_time) = last_processed.get(&path)
                && now.duration_since(last_time) < DEBOUNCE_DURATION
            {
                // Skip this event as it's within the debounce interval
                continue;
            }
            last_processed.insert(path.clone(), now);

            let Some(kind) = classify_ron(&path) else {
                continue;
            };

            println!("Change detected in: {}", path.display().to_string().cyan());
            let data = fs::read_to_string(&path);
            match data {
                Ok(content) => {
                    let result = match kind {
                        RonKind::Cutscene => from_str::<CutsceneData>(&content).map(|_| ()),
                        RonKind::Stage => from_str::<StageData>(&content).map(|_| ()),
                    };
                    match result {
                        Ok(()) => println!("{} parsed RON file.", "SUCCESSFULLY".green().bold()),
                        Err(err) => {
                            println!("{} to parse RON:", "FAILED".red().bold());
                            println!("{err:?}");
                        }
                    }
                }
                Err(err) => {
                    println!("{} to read file: {:?}", "FAILED".red().bold(), err);
                }
            }
        }
    }
}
