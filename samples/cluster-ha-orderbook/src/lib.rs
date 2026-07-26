//! HA cluster orderbook sample: try_claim publish + never-stale book.
//!
//! Recipes: `just samples-cluster-ha`, `just samples-cluster-ha-kill-leader`.

// Generated AppMessage / L2Book codecs (`generate_to_out_dir` → OUT_DIR).
ergo_sbe::sbe_mod!(pub normalized_app);

pub mod follower;
pub mod ha_book;
pub mod market;
pub mod publish;
