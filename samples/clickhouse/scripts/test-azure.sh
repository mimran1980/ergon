#!/usr/bin/env bash
# `azure.sh down` against a fake az: it deletes nothing unconfirmed, removes
# NetworkWatcherRG only while it holds nothing but watchers, and fails while
# anything is left to bill. And az's import deadlock is retried, nothing else.
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
# AZ_TEST_FAIL: "<count> <stderr line>": fail that many calls with that line.
if [[ -s ${AZ_TEST_FAIL:-} ]]; then
    read -r n line < "$AZ_TEST_FAIL"
    if ((n > 0)); then
        echo "$((n - 1)) $line" > "$AZ_TEST_FAIL"
        echo "$line" >&2
        exit 1
    fi
fi
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

status() { AZ_TEST_FAIL=$fixture/fail "$sample_root/scripts/azure.sh" status >/dev/null 2>&1; }

reset
echo "2 _frozen_importlib._DeadlockError: deadlock detected by _ModuleLock('requests.structures')" > "$fixture/fail"
status || { echo 'an az import deadlock was not retried' >&2; exit 1; }

reset
echo '1 ERROR: (QuotaExceeded) Operation could not be completed' > "$fixture/fail"
if status; then echo 'a real az failure was retried away' >&2; exit 1; fi
[[ $(cut -d' ' -f1 "$fixture/fail") == 0 ]] || { echo 'a real az failure was not reported' >&2; exit 1; }

# As `up` creates nodes: a function run in the background (`node ... &`) calls
# the wrapper in an `if`, and az fails because the VM does not exist yet. The
# wrapper must return that failure. With `command az`, macOS's bash 3.2 execed
# az in place of the subshell, which ended every node silently.
eval "$(sed -n '/^az_bin=/p; /^az() {/,/^}/p' "$sample_root/scripts/azure.sh")"
probe() {
    if az vm show -g ergon-lab -n lab-x -o none 2>/dev/null; then echo exists; fi
    echo after
}
probe > "$fixture/probe-1" & first=$!
probe > "$fixture/probe-2" & second=$!
wait "$first" && wait "$second" && [[ $(cat "$fixture/probe-1" "$fixture/probe-2") == $'after\nafter' ]] \
    || { echo 'a failing az ended a background subshell instead of returning' >&2; exit 1; }
echo 'azure down: PASS (unconfirmed, full teardown, leftover resources); az retries: PASS (deadlock only); az in a background condition: PASS'
