# The lab on Azure: one VM per region, all in one resource group
# (scripts/azure.sh), with k3s across them (scripts/k3s.sh). The first node
# builds the images and runs the lab's recipes; `on` runs one there.
#
#   just azure up           create the VMs and k3s; prints the .env lines
#   just azure sync         copy this checkout to the VMs (after every edit)
#   just azure on RECIPE    run a lab recipe on the first node: `on up`, `on md`,
#                           `on verify`, `on aeron`, `on test`
#   just azure pause        stop k3s and every pod, keeping VMs and data
#   just azure resume       start them again
#   just azure stop         deallocate the VMs: no compute charge, cluster kept
#   just azure start        start them again
#   just azure status       every resource group and resource in the subscription
#   just azure down         delete everything; fails unless the subscription is empty
#
# End every session with `just azure down`: a stopped VM still pays for its
# disk and address; only a deleted group costs nothing.

set dotenv-load

# A bare `just azure` lists these: it must never create anything.
[private]
default:
    @just --list azure

# Create the VMs and k3s; prints the .env lines
up:
    scripts/azure.sh up

# Delete everything; fails unless the subscription is empty
down:
    scripts/azure.sh down

# Deallocate the VMs: no compute charge, cluster kept
stop:
    scripts/azure.sh stop

# Start deallocated VMs again
start:
    scripts/azure.sh start

# Every resource group and resource in the subscription
status:
    scripts/azure.sh status

# Stop k3s and every pod, keeping VMs and data
pause:
    scripts/k3s.sh stop

# Start k3s again, the server first
resume:
    scripts/k3s.sh start

# The first node holds the build and the checkout its build container mounts.
# Run lab recipes on the first node: `just azure on md`, `on verify`
on +recipes:
    #!/usr/bin/env bash
    set -euo pipefail
    read -r first _ <<<"$LAB_VM_IPS"
    ssh -t -o BatchMode=yes "$first" "cd /srv/ergon/samples/clickhouse && LAB_CONTEXT=lab-vms just {{recipes}}"

# /lab points into /srv/ergon. Never deletes; the Mac's copy wins, so bring a
# notebook edited on a VM back first.
# Copy this checkout to every VM (after each edit)
sync:
    #!/usr/bin/env bash
    set -euo pipefail
    files=$(mktemp)
    trap 'rm -f "$files"' EXIT
    # What git sees (tracked, and new files it does not ignore): not the
    # gigabytes of ignored build output beside them. A tracked file deleted
    # here is skipped (macOS rsync has no --ignore-missing-args).
    (cd ../.. && { echo samples/clickhouse/.env; git ls-files -co --exclude-standard; git ls-files --recurse-submodules; } \
        | sort -u | while IFS= read -r f; do if [[ -e $f ]]; then printf '%s\n' "$f"; fi; done) > "$files"
    first=1
    for ip in $LAB_VM_IPS; do
        ssh -o BatchMode=yes "$ip" 'ls -la /srv/ergon | head -5'
        # .git only to the first node, which builds and benchmarks; the
        # others need the checkout's files alone. By content, not times: a
        # copied file gets the time it lands, so cargo rebuilds what changed
        # (kept times older than its last build left a stale binary).
        { ((first)) && echo .git; cat "$files"; } | rsync -rlpz --checksum --files-from=- ../../ "$ip:/srv/ergon/"
        first=0
    done
