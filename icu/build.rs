fn main() {
    cc::Build::new().file("src/bindings.c").compile("bindings");
    println!("cargo:rustc-link-lib=dylib=icuuc");
    println!("cargo:rustc-link-lib=dylib=icui18n");
}
