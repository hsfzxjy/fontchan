use std::process::Command;
use wasmprinter;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../fontchan-decoder-wasm/*");
    Command::new("cargo")
        .args(&[
            "build",
            "-p",
            "fontchan-decoder-wasm",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--target-dir",
            "../target/wasm/",
        ])
        .current_dir("../fontchan-decoder-wasm/")
        .status()
        .unwrap();
    let binary =
        std::fs::read("../target/wasm/wasm32-unknown-unknown/release/fontchan_decoder_wasm.wasm")
            .unwrap();
    let wat = wasmprinter::print_bytes(&binary).unwrap();
    let mut out_path = std::env::var("OUT_DIR").unwrap();
    out_path.push_str("/decoder.wat");
    std::fs::write(out_path, &wat).unwrap();
}
