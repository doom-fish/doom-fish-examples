//! Adds the Swift Concurrency runtime rpath to binaries produced by this
//! crate. The doom-fish bridge crates (apple-cf, foundation-models, vision,
//! ...) all set this same rpath via their own `build.rs`, but those args
//! only apply to *their* binaries — when downstream crates link those
//! libraries into their own binaries, they have to set the rpath themselves.
//!
//! Without this, `cargo run --bin ocr_to_summary` fails at startup with:
//!   `dyld[…]: Library not loaded: @rpath/libswift_Concurrency.dylib`

use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=DEVELOPER_DIR");

    if env::var("DOCS_RS").is_ok() {
        return;
    }

    // Bake the system Swift runtime path in (works on most modern macOS systems).
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    // Plus the active Xcode toolchain's Swift runtime — required for
    // `libswift_Concurrency.dylib`, which isn't always at /usr/lib/swift on
    // every macOS / Xcode combination.
    if let Ok(output) = Command::new("xcode-select").arg("-p").output() {
        if output.status.success() {
            let xcode_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Newer-style path
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{xcode_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
            );
            // Older Swift 5.5 layout — kept for safety on stale toolchains
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{xcode_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx"
            );
        }
    }
}
