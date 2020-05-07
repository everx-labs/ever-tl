use std::{fs, path};
use std::io::Read;
use std::path::Path;

const OUTPUT_DIR: &str = "src/ton";
const TL_DIR: &str = "tl";

fn main() {
    println!("cargo:rerun-if-changed={}", OUTPUT_DIR);
    println!("cargo:rerun-if-changed={}", TL_DIR);
    println!("cargo:rerun-if-changed=../ton_tl_codegen");
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
        fs::File::open(&file)
            .expect(format!("Unable to open file for reading: {}", file.to_string_lossy()).as_str())
            .read_to_string(&mut input)
            .expect(format!("Unable to read file contents: {}", file.to_string_lossy()).as_str());
        println!("cargo:rerun-if-changed={}", file.to_string_lossy());
    }
    ton_tl_codegen::generate_code_for(&input, Path::new(OUTPUT_DIR));
}
