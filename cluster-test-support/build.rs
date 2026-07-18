use std::path::Path;

fn main() {
    let aeron_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("aeron");
    let libs_dir = aeron_dir.join("aeron-all").join("build").join("libs");

    if !libs_dir.exists() {
        let gradle = if cfg!(target_os = "windows") {
            aeron_dir.join("gradlew.bat")
        } else {
            aeron_dir.join("gradlew")
        };
        eprintln!("Building aeron jars via Gradle in {}", aeron_dir.display());
        let status = std::process::Command::new(&gradle)
            .current_dir(&aeron_dir)
            .args([
                ":aeron-cluster:jar",
                ":aeron-archive:jar",
                ":aeron-all:jar",
                ":aeron-samples:jar",
            ])
            .status()
            .expect("failed to run Gradle — is Java 17+ installed?");
        if !status.success() {
            panic!("Gradle jar build failed with exit code: {status}");
        }
    }

    // Compile ClusterLauncher into the aeron-samples build dir so it
    // sits alongside the other compiled classes on the classpath.
    let java_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("java")
        .join("ClusterLauncher.java");
    let jar_dir = aeron_dir.join("aeron-all").join("build").join("libs");
    let cluster_jar = aeron_dir.join("aeron-cluster").join("build").join("libs");
    let samples_classes = aeron_dir
        .join("aeron-samples")
        .join("build")
        .join("classes")
        .join("java")
        .join("main");

    let cp = format!("{}/*:{}/*", jar_dir.display(), cluster_jar.display(),);

    eprintln!(
        "Compiling ClusterLauncher into {}",
        samples_classes.display()
    );
    let _ = std::fs::create_dir_all(&samples_classes);
    let status = std::process::Command::new("javac")
        .args(["-cp", &cp, "-d"])
        .arg(&samples_classes)
        .arg(&java_src)
        .status()
        .expect("failed to run javac — is Java 17+ installed?");
    if !status.success() {
        panic!("javac failed to compile ClusterLauncher");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/java/ClusterLauncher.java");
    println!("cargo:rerun-if-changed=../aeron/aeron-all/build/libs");
}
