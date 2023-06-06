use rustc_version::{version_meta, Channel};

fn main() {
    let version = version_meta().unwrap();

    match version.channel {
        Channel::Nightly => {
            println!("cargo:rustc-cfg=rustc_nightly");
        }
        _ => {}
    }
}
