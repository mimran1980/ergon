//! Run the ergo-sbe feature tour demos.
//!
//! ```sh
//! cargo run --manifest-path samples/sbe-feature-tour/Cargo.toml
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    sbe_feature_tour::run_all()
}
