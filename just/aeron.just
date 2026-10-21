# ── Aeron / Java / cluster harness recipes ────────────────────────────────

# Check the cluster crate (lib: fmt + clippy + tests, no Java required).
check-aeron-cluster:
    cd cluster && cargo fmt --check
    cd cluster && cargo clippy --all-targets -- -D warnings
    cd cluster && cargo test --lib

# Cluster integration tests (requires a JDK 17 or newer, 25 recommended, and
# built Aeron jars — run `just build-aeron-jars` once first; slow). The
# harness runs $JAVA_HOME/bin/java, else java on PATH.
# Cluster integration tests (Java 25 and built Aeron jars; slow)
test-aeron-cluster-harness:
    cargo test -p ergo-aeron-cluster-test-harness -- --test-threads=1

# Run once before the test-harness tests. Aeron's Gradle 9.7.1 runs on JDK 17
# to 26 ($JAVA_HOME, else java on PATH): 27 fails with "Unsupported class
# file major version 71".
# Build the Aeron cluster test jars (JDK 17 to 26; 25 recommended)
build-aeron-jars:
    cd aeron && ./gradlew :aeron-cluster:jar :aeron-archive:jar :aeron-all:jar
