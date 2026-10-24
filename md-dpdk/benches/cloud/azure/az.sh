# Sourced by azure/lab.sh: what it asks of Azure before and while the run's
# resources exist, as functions test/test-lab.sh checks against a fake az.

# azure_arch <size>: arm64 for Azure's Arm sizes (Cobalt 100: a p after the
# vCPUs, Standard_D8ps_v6, Standard_D8pls_v6, Standard_E8ps_v6), else x86_64
azure_arch() {
    if [[ $1 =~ ^Standard_[A-Z]+[0-9]+[a-z]*p[a-z]*_ ]]; then echo arm64; else echo x86_64; fi
}

# azure_image <arm64|x86_64>: Debian 13's Gen2 image for it
azure_image() {
    case $1 in
    arm64) echo Debian:debian-13:13-arm64:latest ;;
    *) echo Debian:debian-13:13-gen2:latest ;;
    esac
}

# az_words: an `-o tsv` answer on one line, its values space-separated.
# azure-cli 2.91 prints a top-level list one value per line (earlier ones
# tab-separated), so `read a b < <(az ... -o tsv)` would read the first alone.
az_words() { tr -s '\t\r\n' '   ' | sed 's/^ //; s/ $//'; }

# azure_quota_use <region> <quota> <vCPUs>: the quota's use and limit
# ("<used> <limit>"), or 2 saying why on stderr: unread, or too full for
# <vCPUs> more (quota: a family's name, or cores: the region's vCPUs)
azure_quota_use() {
    local region=$1 q=$2 v=$3 ans used limit what
    ans=$(az vm list-usage -l "$region" --query "[?name.value=='$q'] | [0].[currentValue, limit]" -o tsv 2>&1) ||
        { echo "quota: cannot read the $q quota in $region: $ans" >&2; return 2; }
    read -r used limit <<<"$(az_words <<<"$ans")"
    [[ $used =~ ^[0-9]+$ && $limit =~ ^[0-9]+$ ]] ||
        { echo "quota: the $q quota in $region reads [$ans], not its use and limit" >&2; return 2; }
    what=$q
    [[ $q == cores ]] && what="the region's vCPUs (cores)"
    if ((used + v > limit)); then
        echo "quota: $what in $region: $used of $limit vCPUs in use, and this run needs $v more: delete what runs there or raise the quota" >&2
        return 2
    fi
    echo "$used $limit"
}

# azure_quota_check <region> <size>...: 0, printing each quota family's and
# the region's vCPUs in use and this run's, or 2 saying why on stderr: a
# size the region does not offer, a quota unread, or one the sizes do not fit
azure_quota_check() {
    local region=$1 s fam v total=0 pairs= use line=
    shift
    for s in "$@"; do
        [[ $s =~ ^Standard_[A-Z]+([0-9]+) ]] || { echo "quota: $s: no vCPU count in its name" >&2; return 2; }
        v=${BASH_REMATCH[1]}
        total=$((total + v))
        fam=$(az vm list-skus -l "$region" --size "$s" --resource-type virtualMachines \
            --query "[?name=='$s'].family | [0]" -o tsv 2>&1) || { echo "quota: cannot read $s's quota family: $fam" >&2; return 2; }
        fam=$(az_words <<<"$fam")
        [[ $fam =~ ^[A-Za-z0-9]+$ ]] || { echo "quota: $s is not offered in $region" >&2; return 2; }
        pairs+="$fam $v"$'\n'
    done
    while read -r fam v; do
        use=$(azure_quota_use "$region" "$fam" "$v") || return 2
        line+="${line:+, }$fam ${use% *} of ${use#* } vCPUs in use, this run $v"
    done < <(awk 'NF == 2 {n[$1] += $2} END {for (f in n) print f, n[f]}' <<<"$pairs" | sort)
    use=$(azure_quota_use "$region" cores "$total") || return 2
    echo "quota: $line; the region ${use% *} of ${use#* }, this run $total"
}

# azure_reaper <group> <stamp> <seconds> <log>: the backstop, should this
# machine's teardown never run (killed): a process of its own (nohup, so it
# outlives this shell) deleting the whole group <seconds> from now, only
# while the group still carries this run's tag (run=<stamp>), so it never
# deletes a later run's group of the same name, then checking it gone. Its
# deadline is wall-clock time, polled every AZ_REAPER_POLL seconds (60): a
# Mac asleep stops sleep's clock, so one sleep of the whole period would
# delete as many hours late as the Mac slept. Prints its pid; TERM stops it
# and its sleep.
azure_reaper() {
    # shellcheck disable=SC2016
    nohup bash -c 'trap '\''kill "$s" 2>/dev/null; exit 0'\'' TERM
        deadline=$(($(date +%s) + $1)) poll=$4
        while now=$(date +%s) && ((now < deadline)); do
            sleep $((deadline - now < poll ? deadline - now : poll)) & s=$!
            wait "$s"
        done
        run=$(az group show -n "$2" --query tags.run -o tsv 2>/dev/null)
        if [[ $run != "$3" ]]; then
            echo "$(date +%T) backstop: resource group $2 is gone or not run=$3 ([$run]): nothing deleted"
            exit 0
        fi
        for try in 1 2 3; do
            echo "$(date +%T) backstop: deleting resource group $2 (run=$3), try $try"
            az group delete -n "$2" --yes -o none
            if [[ $(az group exists -n "$2" 2>/dev/null) == false ]]; then
                echo "$(date +%T) backstop: resource group $2 verified gone"
                exit 0
            fi
            sleep 60 & s=$!
            wait "$s"
        done
        echo "$(date +%T) backstop: ERROR: resource group $2 still exists: az group delete -n $2 --yes"
        exit 1' ergon-lab-reaper "$3" "$1" "$2" "${AZ_REAPER_POLL:-60}" >"$4" 2>&1 </dev/null &
    echo $!
}

# azure_watch <group> <seconds>: every <seconds>, a log line listing the
# group's resources (type/name), so a run's log shows what is billing
azure_watch() {
    local group=$1 every=$2 list
    while sleep "$every"; do
        list=$(az resource list -g "$group" --query '[].[type, name]' -o tsv 2>&1) || {
            echo "[$(date +%T)] resources in $group: unread: $(az_words <<<"$list")" >&2
            continue
        }
        echo "[$(date +%T)] resources in $group: $(grep -c . <<<"$list"): $(awk -F'\t' 'NF >= 2 {printf "%s%s/%s", sep, $1, $2; sep = " "}' <<<"$list")" >&2
    done
}
