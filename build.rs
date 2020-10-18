use std::env;

fn main() {
    if env::var("HOST").unwrap() == "x86_64-apple-darwin" {
        // PowerDNA
        println!("cargo:rustc-link-lib=dylib=powerdna");
        println!("cargo:rustc-link-lib=dylib=UeiPal");
        println!("cargo:rustc-link-search=native=syslib/uei");

        // ZMQ
        println!("cargo:rustc-env=LIBZMQ_PREFIX=syslib/zmq");
        println!("cargo:rustc-link-search=native=syslib/zmq/lib");
    }
}
