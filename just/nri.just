# k8s/nri-shared-cpus: the containerd NRI plugin and kubelet device plugin
# that give a pod its exclusive CPUs and the node's shared housekeeping CPUs.
# Go, not Rust. Nothing is published: RELEASE.md in that tree is the gate.
# The images stay local (`:dev`) unless you run the image workflow's gated
# publish job.

# gofmt, go vet, go test -race and a static build for amd64 and arm64, then the cgroup kernel lane (needs Docker)
check-nri:
    bash k8s/nri-shared-cpus/scripts/check.sh

# The unit lane on the host, with the Go toolchain on PATH (CI)
check-nri-unit:
    bash k8s/nri-shared-cpus/scripts/check.sh --local

# The cgroup kernel lane on its own: a real cgroup v2 hierarchy in a privileged container
check-nri-cgroup:
    bash k8s/nri-shared-cpus/scripts/check-cgroup.sh

# The 18-step kind e2e. Needs Docker and >= 6 CPUs. Deletes everything it creates
test-nri-e2e:
    bash k8s/nri-shared-cpus/test/e2e.sh

# Lint and render the Helm chart without a cluster
check-nri-chart:
    helm lint k8s/nri-shared-cpus/charts/ergon-shared-cpus
    helm template ergon-shared-cpus k8s/nri-shared-cpus/charts/ergon-shared-cpus \
        --set config.sharedCpus=2 --set 'config.namespaces={trading}' >/dev/null

# Build the image locally as ergon-shared-cpus:dev. Not pushed anywhere
build-nri-image:
    docker build -t ergon-shared-cpus:dev k8s/nri-shared-cpus

# Multi-arch buildx, loaded locally. Not pushed anywhere
build-nri-image-multiarch:
    docker buildx build --platform linux/amd64,linux/arm64 \
        -t ergon-shared-cpus:dev k8s/nri-shared-cpus

# The Azure latency lab without a cloud: the script against fake az/ssh, the
# lab's functions on fixtures, and the gap probe's unit tests. Needs ruby, jq
test-nri-lab:
    bash k8s/nri-shared-cpus/lab/test/test-lab.sh
    cargo test --manifest-path k8s/nri-shared-cpus/lab/gapprobe/Cargo.toml

# The latency lab's plan, size and price: no cloud call
plan-nri-lab:
    DRY_RUN=1 bash k8s/nri-shared-cpus/lab/azure-vm.sh

# The full local gate before anything is published: units, cgroup lane, chart, image
check-nri-all: check-nri check-nri-chart build-nri-image
