# ergon-md-dpdk: the DPDK ENA kernel-bypass transport for the rusteron media
# driver. The `dpdk` feature builds only on Linux x86_64 or aarch64 with
# libdpdk >= 23.11 (Ubuntu 24.04: `libdpdk-dev`), so these recipes are not part
# of `just test` on other hosts; CI runs them in the `dpdk` job. The portable
# half of the crate (config, environment, binary selection) is in the regular
# recipes.

# Clippy and every test of the DPDK transport against the DPDK-free fakes, and the lab clients' tests (Linux x86_64 or aarch64 + libdpdk)
check-dpdk:
    cargo clippy -p ergon-md-dpdk -p ergon-md-dpdk-harness --all-targets --features ergon-md-dpdk/dpdk,ergon-md-dpdk-harness/dpdk -- -D warnings
    cargo test -p ergon-md-dpdk --features dpdk -- --test-threads=1
    cargo test -p ergon-md-dpdk --features static,dpdk -- --test-threads=1
    cargo test -p ergon-md-dpdk-harness -- --test-threads=1

# Interop over DPDK tap devices on a Linux bridge (root, /dev/net/tun, hugepages)
test-dpdk-vdev *args:
    sudo env "PATH=$PATH" scripts/test-dpdk-vdev.sh {{args}}

# The vdev scenarios without root: user namespaces, net_tap, EAL --no-huge (Linux; scripts/test-dpdk-vdev-rootless.sh says what the host needs)
test-dpdk-vdev-rootless *args:
    scripts/test-dpdk-vdev-rootless.sh {{args}}

# Node bootstrap and preflight against fake sysfs and IMDS trees
test-dpdk-bootstrap:
    bash md-dpdk/deploy/aws-eks/node/test/test-bootstrap.sh

# The cloud lab's analysis (analyse.py, spikes.py, fairness.py) against its fixtures, its scripts' syntax, the transport suite's runner, and the EKS runners against fakes; no cloud
test-dpdk-lab:
    bash md-dpdk/benches/cloud/test/test-lab.sh
    bash md-dpdk/benches/cloud/test/test-transport.sh
    bash md-dpdk/benches/cloud/test/test-transport-runner.sh
    bash md-dpdk/benches/cloud/aws/test/test-eks-nri.sh
    bash md-dpdk/benches/cloud/aws/test/test-latency-bench-podrun.sh

# The transport suite end to end on one Linux box in Docker (--privileged: netem, sysctls; not CI): every non-DPDK arm, 1 rep, then fairness.py and analyse.py --transport into target/transport-smoke
transport-smoke-docker *args:
    bash md-dpdk/benches/cloud/test/transport-smoke-docker.sh {{args}}

# The DPDK tests in Docker, amd64 emulation or `--arm64` (Graviton's architecture, native on Apple silicon); `--build` the image, `--all` for check-dpdk, `--rootless` for the vdev scenarios as a non-root user, else a test filter
verify-dpdk-docker *args:
    bash md-dpdk/native/dpdk/test/verify-dpdk.sh {{args}}
