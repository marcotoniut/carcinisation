use heck::ShoutySnakeCase;
use ignore::WalkBuilder;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn main() {
    let out_dir = "src/assets.rs";
    let in_dir = "assets";

    let mut file = File::create(&Path::new(&out_dir)).unwrap();

    for result in WalkBuilder::new(in_dir).build() {
        let entry = result.unwrap();
        writeln!(
            file,
            "use assert_assets_path::assert_assets_path;",
        );
        writeln!(
            file,
            "",
        );

        if entry.file_type().unwrap().is_file() {
            let path = entry.path().strip_prefix(in_dir).unwrap();
            let filename = path.to_str().unwrap();
            let var_name: String = filename
                .replace("/", ".")
                .to_shouty_snake_case()
                .replace(".", "__");

            writeln!(
                file,
                "pub const {}: &str = assert_assets_path!(\"{}\");",
                var_name,
                path.display()
            )
            .unwrap();
        }
    }
}
