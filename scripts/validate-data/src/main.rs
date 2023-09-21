mod data;
mod paths;

extern crate serde;
extern crate serde_yaml;

use data::stage::StageData;
use paths::ASSETS_STAGES_PATH;
use std::fs::File;
use std::path::Path;

fn main() {
    let path = Path::new(ASSETS_STAGES_PATH);

    for entry in path.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            if let Ok(file) = File::open(entry.path()) {
                let result: Result<StageData, serde_yaml::Error> = serde_yaml::from_reader(file);
                match result {
                    Ok(stage) => println!("{:#?}", stage),
                    Err(e) => eprintln!("Error parsing file {:?}: {}", entry.path(), e),
                }
            }
        }
    }
}
