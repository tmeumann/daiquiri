fn main() {
    println!("cargo:rustc-link-lib=dylib=powerdna");
    println!("cargo:rustc-link-lib=dylib=UeiPal");
}
