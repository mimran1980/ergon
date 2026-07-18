fn main() {
    let codecs_dir = std::path::Path::new("src/codecs/generated");
    if codecs_dir.exists() {
        println!("cargo::rerun-if-changed=src/codecs/generated");
    }
    // Codecs are committed; build.rs does NOT regenerate.
    // Use `just generate-cluster-codecs` for regeneration.
}
