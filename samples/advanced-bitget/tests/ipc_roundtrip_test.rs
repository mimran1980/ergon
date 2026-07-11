//! Aeron IPC integration tests with Rusteron 0.2.1.
#![allow(unused)]

/// Rusteron 0.2.1 crates compile and link.
#[test]
fn rusteron_dependency_links() {
    // Prove Rusteron 0.2.1 is available and links successfully.
    // Use the CStr-based API that rusteron exposes.
    let aeron_dir = std::ffi::CString::new("/tmp/test_aeron_dir").unwrap();
    let _ctx = rusteron_client::AeronContext::new();
    // If we reach here, rusteron compiles and links.
}
