use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let lib_src = manifest_dir.join("libembroidery");

    // emb-outline.c contains incomplete/non-C code and must be excluded
    let skip = ["emb-outline.c"];
    let c_files: Vec<_> = std::fs::read_dir(&lib_src)
        .expect("Failed to read libembroidery dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("c")
                && !skip.contains(&p.file_name().and_then(|s| s.to_str()).unwrap_or(""))
        })
        .collect();

    // The read-side shim lives outside libembroidery/, so add it explicitly.
    let shim = manifest_dir.join("csrc/shim.c");

    cc::Build::new()
        .files(c_files.clone())
        .file(&shim)
        .include(&lib_src)
        .warnings(false)
        // libembroidery declares globals (inputArray, currentIndex, mStatus, …)
        // in headers as tentative definitions. GCC 10+/clang default to
        // -fno-common, which promotes each to a strong symbol, so the same
        // global defined across translation units becomes a duplicate-symbol
        // link error (seen with lld on Linux). -fcommon restores the merge.
        // flag_if_supported => MSVC (which lacks the flag) silently ignores it.
        .flag_if_supported("-fcommon")
        .compile("embroidery");

    // Recompile whenever any vendored C source (or the shim) changes. Without
    // this, edits to libembroidery/*.c are silently ignored and the stale static
    // lib is relinked. `.file()` alone does not reliably emit these.
    for c in &c_files {
        println!("cargo:rerun-if-changed={}", c.display());
    }
    println!("cargo:rerun-if-changed={}", shim.display());
    println!("cargo:rerun-if-changed=build.rs");
}
