#!/usr/bin/env bash
# `azure.sh down` against a fake az: it deletes nothing unconfirmed, removes
# NetworkWatcherRG only while it holds nothing but watchers, and fails while
# anything is left to bill.
set -euo pipefail
sample_root=$(cd "$(dirname "$0")/.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/groups"
export AZ_TEST_GROUPS="$fixture/groups" PATH="$fixture/bin:$PATH" LAB_AZ_GROUP=ergon-lab

# Each group is a file of "type name location" lines.
cat > "$fixture/bin/az" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
g=$AZ_TEST_GROUPS
arg() { local want=$1; shift; while (($#)); do [[ $1 == "$want" ]] && { echo "$2"; return; }; shift; done; }
case "$1 $2" in
    "group exists") [[ -f $g/$(arg -n "$@") ]] && echo true || echo false ;;
    "group delete") echo "$(arg -n "$@")" >> "$g/../deleted"; rm "$g/$(arg -n "$@")" ;;
    "group list") for f in "$g"/*; do [[ -f $f ]] && printf '%s\tjapaneast\n' "$(basename "$f")"; done; true ;;
    "resource list")
        group=$(arg -g "$@")
        query=$(arg --query "$@")
        files=("$g"/*)
        [[ -n $group ]] && files=("$g/$group")
        lines=$(cat "${files[@]}" 2>/dev/null || true)
        case "$query" in
            "length(@)") grep -c . <<<"$lines" || true ;;
            *networkWatchers*) grep -vc '^Microsoft.Network/networkWatchers ' <<<"$lines" || true ;;
            *) printf '%s\n' "$lines" ;;
        esac ;;
    *) echo "fake az: unexpected $*" >&2; exit 2 ;;
esac
SH
chmod +x "$fixture/bin/az"

reset() {
    rm -f "$fixture/groups"/* "$fixture/deleted"
    printf 'Microsoft.Compute/virtualMachines lab-an1 japaneast\n' > "$fixture/groups/ergon-lab"
    printf 'Microsoft.Network/networkWatchers NetworkWatcher_japaneast japaneast\n' > "$fixture/groups/NetworkWatcherRG"
}
down() { "$sample_root/scripts/azure.sh" down <<<"$1" >/dev/null 2>&1; }

reset
if down no; then echo 'down must fail without "destroy"' >&2; exit 1; fi
[[ -f $fixture/groups/ergon-lab && ! -f $fixture/deleted ]] || { echo 'down deleted without confirmation' >&2; exit 1; }

reset
down destroy || { echo 'down failed on a group it could delete' >&2; exit 1; }
[[ $(cat "$fixture/deleted") == $'ergon-lab\nNetworkWatcherRG' ]] || { echo 'down left a group behind' >&2; exit 1; }

reset
printf 'Microsoft.Storage/storageAccounts other japaneast\n' >> "$fixture/groups/NetworkWatcherRG"
if down destroy; then echo 'down must fail while resources remain' >&2; exit 1; fi
[[ -f $fixture/groups/NetworkWatcherRG ]] || { echo 'down deleted a NetworkWatcherRG holding more than watchers' >&2; exit 1; }
echo 'azure down: PASS (unconfirmed, full teardown, leftover resources)'
