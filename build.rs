//! Provide `libxdo.so` symlink when only `libxdo.so.*` exists (runtime .so without `-dev` symlink).

use std::path::Path;

fn main() {
    let candidates = [
        "/usr/lib/x86_64-linux-gnu/libxdo.so.3",
        "/usr/lib/libxdo.so.3",
    ];
    for p in &candidates {
        if Path::new(p).exists() {
            let out = std::env::var("OUT_DIR").expect("OUT_DIR");
            let link = format!("{}/libxdo.so", out);
            let _ = std::fs::remove_file(&link);
            if std::os::unix::fs::symlink(p, &link).is_ok() {
                println!("cargo:rustc-link-search=native={}", out);
            }
            break;
        }
    }
}
