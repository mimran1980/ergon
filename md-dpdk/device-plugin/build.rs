//! Compiles the vendored kubelet deviceplugin v1beta1 API
//! (proto/deviceplugin/v1beta1/api.proto) with protox — a pure-Rust protoc —
//! and feeds the resulting FileDescriptorSet to tonic-prost-build. No protoc
//! binary is needed in Docker/CI (plan §10.3). Output lands in OUT_DIR as
//! `v1beta1.rs` (package `v1beta1`).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const PROTO: &str = "deviceplugin/v1beta1/api.proto";
    println!("cargo:rerun-if-changed=proto/{PROTO}");

    let mut compiler = protox::Compiler::new(["proto"])?;
    compiler.open_file(PROTO)?;
    tonic_prost_build::configure().compile_fds(compiler.file_descriptor_set())?;
    Ok(())
}
