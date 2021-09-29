use std::{fs, path};
use std::io::Read;
use std::path::Path;

const OUTPUT_DIR: &str = "ton_api/src/ton";
const TL_DIR: &str = "ton_api/tl";

fn main() {
    let mut files = fs::read_dir(TL_DIR)
        .expect(format!("Unable to read directory contents: {}", TL_DIR).as_str())
        .filter_map(Result::ok)
        .map(|d| d.path())
        .filter(|path| path.to_str().unwrap().ends_with(".tl"))
        .collect::<Vec<path::PathBuf>>();

    assert!(files.len() > 0);
    files.sort();

    let mut input = String::new();
    for file in files {
        if input.len() > 0 {
            input += "---types---\n";
        }
        fs::File::open(&file).unwrap()
            .read_to_string(&mut input).unwrap();
    }

    ton_tl_codegen::generate_code_for(None, &input, Path::new(OUTPUT_DIR));
}

