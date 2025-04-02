fn main() {
    let libpath = std::env::var("LIBPATH").unwrap_or(String::new());
    println!("cargo:rustc-link-search={}", libpath);
    println!("cargo:rustc-link-lib=zlib")
}
