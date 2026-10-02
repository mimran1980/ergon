# kind on this machine: four nodes, three regions (deploy/kind.yaml), the
# checkout mounted at /lab on each. The lab's other recipes are in justfile.
#
#   just mac up        create the cluster if needed, then build, deploy and wait
#   just mac stop      pause every node, keeping data
#   just mac start     resume them
#   just mac destroy   delete the cluster, including all recorded data

set dotenv-load

cluster := "clickhouse-lab"

# A bare `just mac` lists these: it must never create anything.
[private]
default:
    @just --list mac

# Create the kind cluster (if needed), then build, deploy and wait
up:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! kind get clusters | grep -qx {{cluster}}; then
        sed "s|LAB_DIR|$PWD|" deploy/kind.yaml | kind create cluster --config -
    fi
    LAB_CONTEXT=kind-{{cluster}} just up

# Pause every kind node, keeping data
stop:
    #!/usr/bin/env bash
    set -euo pipefail
    nodes=$(kind get nodes --name {{cluster}})
    test -n "$nodes"
    docker stop $nodes

# Resume the kind nodes
start:
    #!/usr/bin/env bash
    set -euo pipefail
    nodes=$(kind get nodes --name {{cluster}})
    test -n "$nodes"
    docker start $nodes

# Delete the cluster, including all recorded data
destroy:
    kind delete cluster --name {{cluster}}
