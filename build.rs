// Ensure web/dist/ exists so that rust-embed's `#[derive(Embed)]` on
// `web/dist/` compiles on a fresh clone, before the frontend has ever been
// built. The folder may be empty in that case — rust-embed embeds an empty
// asset set and the frontend fallback returns a 404 until `task web-build`
// populates it. Idempotent and cheap; no rerun-if-changed (rust-embed emits
// its own for the folder contents).
fn main() {
    let dist = std::path::Path::new("web/dist");
    if !dist.is_dir() {
        let _ = std::fs::create_dir_all(dist);
    }
}
