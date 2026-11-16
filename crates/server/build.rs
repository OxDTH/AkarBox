// A build script runs before the crate is compiled.
// `rust-embed` refuses to compile if the folder it embeds is missing, so make sure
// `frontend/dist` exists even before the frontend has been built.
fn main() {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/dist");
    std::fs::create_dir_all(&dist).expect("create frontend/dist");
    // Rebuild (re-embed) whenever the built frontend changes.
    println!("cargo:rerun-if-changed=../../frontend/dist");
}
