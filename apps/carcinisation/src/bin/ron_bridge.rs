//! RON ⇄ JSON conversion bridge for editor-v2
//! Reads JSON from stdin with format: { "mode": "ron-to-json" | "json-to-ron", "payload": "..." }
//! Outputs converted data to stdout

use anyhow::Result;
use carcinisation::cutscene::data::CutsceneData;
use carcinisation::stage::data::StageData;
use serde::{Deserialize, Serialize};
use std::io::{self, Read};

#[derive(Deserialize)]
struct Input {
    mode: String,
    payload: String,
    #[serde(default)]
    file_type: Option<String>,
}

fn main() -> Result<()> {
    // Read all input from stdin
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let input: Input = serde_json::from_str(&buffer)?;

    match input.mode.as_str() {
        "ron-to-json" => {
            // Determine file type from extension or payload structure
            let file_type = input.file_type.as_deref().unwrap_or("stage");

            match file_type {
                "stage" | "sg" => {
                    let stage: StageData = ron::from_str(&input.payload)?;
                    let json = serde_json::to_string(&stage)?;
                    println!("{}", json);
                }
                "cutscene" | "cs" => {
                    let cutscene: CutsceneData = ron::from_str(&input.payload)?;
                    let json = serde_json::to_string(&cutscene)?;
                    println!("{}", json);
                }
                _ => {
                    eprintln!("Unknown file type: {}", file_type);
                    std::process::exit(1);
                }
            }
        }
        "json-to-ron" => {
            let file_type = input.file_type.as_deref().unwrap_or("stage");

            match file_type {
                "stage" | "sg" => {
                    let stage: StageData = serde_json::from_str(&input.payload)?;
                    let ron = ron::ser::to_string_pretty(
                        &stage,
                        ron::ser::PrettyConfig::default()
                            .struct_names(false)
                            .enumerate_arrays(false),
                    )?;
                    println!("{}", ron);
                }
                "cutscene" | "cs" => {
                    let cutscene: CutsceneData = serde_json::from_str(&input.payload)?;
                    let ron = ron::ser::to_string_pretty(
                        &cutscene,
                        ron::ser::PrettyConfig::default()
                            .struct_names(false)
                            .enumerate_arrays(false),
                    )?;
                    println!("{}", ron);
                }
                _ => {
                    eprintln!("Unknown file type: {}", file_type);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!(
                "Unknown mode: {}. Use 'ron-to-json' or 'json-to-ron'",
                input.mode
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
