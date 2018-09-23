extern crate tl_codegen;

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let types_dest_path = Path::new(&out_dir).join("td_api_types.rs");
    let methods_dest_path = Path::new(&out_dir).join("td_api_methods.rs");

    let src_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_path = Path::new(&src_dir).join("td_api.tl");

    println!("cargo:rerun-if-changed={}",src_path.display());

    let src = fs::read_to_string(src_path).expect("no td_api.tl file");
    let (t, m) = tl_codegen::generate(&src);
    fs::write(types_dest_path, t).expect("cannot write output file");
    fs::write(methods_dest_path, m).expect("cannot write output file");
}
