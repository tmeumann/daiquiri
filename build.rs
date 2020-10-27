use std::env;

fn main() {
    println!("cargo:rustc-link-lib=dylib=powerdna");
    println!("cargo:rustc-link-lib=dylib=UeiPal");
    if env::var("HOST").unwrap() == "x86_64-apple-darwin" {
        println!("cargo:rustc-link-search=native=syslib/uei");
    }
}
