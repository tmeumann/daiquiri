use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/dataframe.fbs");

    flatc_rust::run(flatc_rust::Args {
        inputs: &[Path::new("src/dataframe.fbs")],
        out_dir: Path::new("target/flatbuffers/"),
        ..Default::default()
    })
    .expect("flatc failure");

    if env::var("HOST").unwrap() == "x86_64-apple-darwin" {
        println!("cargo:rustc-link-search=native=syslib/uei");
    }
}
