use std::env;

fn main() {
    if env::var("HOST").unwrap() == "x86_64-apple-darwin" {
        println!("cargo:rustc-link-search=native=syslib/uei");
    }
}
