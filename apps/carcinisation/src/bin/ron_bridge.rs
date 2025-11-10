use carcinisation::stage::data::StageData;
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::Deserialize;
use serde_json::{from_str as json_from_str, to_string as json_to_string};
use std::io::{self, Read};
use std::process::exit;

#[derive(Deserialize)]
struct BridgeRequest {
    mode: String,
    payload: String,
}

fn main() {
    let mut input = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("failed to read stdin: {err}");
        exit(1);
    }

    let request: BridgeRequest = match json_from_str(&input) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("invalid request body: {err}");
            exit(1);
        }
    };

    let output = match request.mode.as_str() {
        "ron-to-json" => match ron_to_json(&request.payload) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("failed to convert RON to JSON: {err}");
                exit(1);
            }
        },
        "json-to-ron" => match json_to_ron(&request.payload) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("failed to convert JSON to RON: {err}");
                exit(1);
            }
        },
        other => {
            eprintln!("unsupported mode: {other}");
            exit(1);
        }
    };

    print!("{output}");
}

fn ron_to_json(ron_text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let stage_data: StageData = ron::from_str(ron_text)?;
    Ok(json_to_string(&stage_data)?)
}

fn json_to_ron(json_text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let stage_data: StageData = json_from_str(json_text)?;

    let config = PrettyConfig::new()
        .new_line("\n".to_string())
        .indentor("    ".to_string())
        .struct_names(true)
        .separate_tuple_members(true)
        .enumerate_arrays(false)
        .compact_arrays(false);

    let ron_body = to_string_pretty(&stage_data, config)?;

    // Add RON feature flags at the top
    let ron_with_flags = format!(
        "#![enable(implicit_some)]\n#![enable(unwrap_newtypes)]\n#![enable(unwrap_variant_newtypes)]\n{}",
        ron_body
    );

    Ok(ron_with_flags)
}
