use assert_assets_path::assert_assets_path;
use carcinisation::cutscene::data::CutsceneData;
use ron::de::from_str;
use std::{fs, path::Path};

fn main() {
    let path = Path::new("../../assets/")
        .join(assert_assets_path!("cinematics/intro/scene.ron").to_string());

    println!("{:?}", path);

    let data = fs::read_to_string(path).expect("Unable to read file");

    let cutscene_data: CutsceneData = from_str(&data).expect("RON was not well-formatted");

    // Print the deserialized data
    println!("{:?}", cutscene_data);
}
