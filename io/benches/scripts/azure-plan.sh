# shellcheck shell=bash
# The fair mode's plan, time and cost estimate, and quota check on Azure
# (sourced by azure-flags.sh and azure-fair.sh after lib-cloud.sh and
# lib-matrix.sh; uses LOC, ARCH, RCV_SIZE, SND_SIZE, IMAGE, PRIORITY, RG,
# STAMP, MAX_S and EST_S).
#
# Prices come from Azure's public retail prices API
# (https://prices.azure.com/api/retail/prices): no sign-in, and the requests
# carry nothing but the filter (service, region, SKU). Pay-as-you-go list
# prices in USD; spot prices move, and an evicted run costs less.

# Set-up before the first point (two VMs, packages, the release build,
# reboot, gap map) and the end (analysis, copy, deleting the group), seconds:
# for the estimate.
FAIR_SETUP_S=${FAIR_SETUP_S:-2400}
FAIR_FINISH_S=${FAIR_FINISH_S:-1200}
# The OS disks (az vm create --os-disk-size-gb; Premium SSD, az's default for
# these sizes, billed by tier): the receiver's holds the build and every
# point's records and traces (128 GB, P10), the sender's only the system and
# its logs (30 GB, the image's own size: P4).
FAIR_RCV_DISK_GB=${FAIR_RCV_DISK_GB:-128}
FAIR_RCV_DISK_SKU=${FAIR_RCV_DISK_SKU:-P10 LRS}
FAIR_SND_DISK_GB=${FAIR_SND_DISK_GB:-30}
FAIR_SND_DISK_SKU=${FAIR_SND_DISK_SKU:-P4 LRS}

# vCPUs, cores and the `lscpu -p=CPU,CORE` topology of a size the fair mode
# knows (the CPU numbering Azure gives each family), or nothing.
size_vcpus() {
  case $1 in
    Standard_F2as_v6 | Standard_D2ps_v6 | Standard_D2ds_v6) echo 2 ;;
    Standard_F4as_v6 | Standard_D4ps_v6 | Standard_D4ds_v6) echo 4 ;;
    Standard_D8ds_v6) echo 8 ;;
  esac
}
size_topo() {
  case $1 in
    # AMD Genoa F-series v6 and Cobalt 100: one vCPU per core, no SMT.
    Standard_F2as_v6 | Standard_D2ps_v6) printf '0,0\n1,1\n' ;;
    Standard_F4as_v6 | Standard_D4ps_v6) printf '0,0\n1,1\n2,2\n3,3\n' ;;
    # Intel D-series v6: two hyperthreads per core, numbered side by side
    # (flags-azure-20261015T045348Z/env-rcv.txt).
    Standard_D2ds_v6) printf '0,0\n1,0\n' ;;
    Standard_D4ds_v6) printf '0,0\n1,0\n2,1\n3,1\n' ;;
    Standard_D8ds_v6) printf '0,0\n1,0\n2,1\n3,1\n4,2\n5,2\n6,3\n7,3\n' ;;
  esac
}

# One price query: the Items of the API's answer for filter $1, one line
# each: retailPrice <tab> skuName <tab> productName <tab> unitOfMeasure <tab>
# meterName. Items carry nested savingsPlan objects after their own fields,
# so each item's own fields are the fragment before its first brace. Readers
# take the whole answer (an early exit would fail the pipeline).
azure_prices() {
  # shellcheck disable=SC2020 # both braces become line breaks
  { curl -sS --max-time 30 -G "https://prices.azure.com/api/retail/prices" --data-urlencode "\$filter=$1" 2> /dev/null || true; } \
    | tr '{}' '\n\n' | { grep '"skuName"' || true; } | while IFS= read -r l; do
      printf '%s\t%s\t%s\t%s\t%s\n' \
        "$(sed -n 's/.*"retailPrice":\([0-9.eE+-]*\).*/\1/p' <<< "$l")" \
        "$(sed -n 's/.*"skuName":"\([^"]*\)".*/\1/p' <<< "$l")" \
        "$(sed -n 's/.*"productName":"\([^"]*\)".*/\1/p' <<< "$l")" \
        "$(sed -n 's/.*"unitOfMeasure":"\([^"]*\)".*/\1/p' <<< "$l")" \
        "$(sed -n 's/.*"meterName":"\([^"]*\)".*/\1/p' <<< "$l")"
    done
}

# USD per hour of Linux VM size $1 at priority $2 (Spot|Regular) in $LOC;
# empty when the API has no such price.
azure_vm_price() {
  azure_prices "serviceName eq 'Virtual Machines' and armRegionName eq '$LOC' and armSkuName eq '$1' and priceType eq 'Consumption'" \
    | awk -F'\t' -v spot="$2" '$3 !~ /Windows/ && $4 == "1 Hour" && !done {
        s = ($2 ~ / Spot$/); lp = ($2 ~ /Low Priority/)
        if ((spot == "Spot" && s) || (spot == "Regular" && !s && !lp)) { print $1; done = 1 } }'
}

# USD per hour of one managed OS disk of SKU $1 (e.g. "P10 LRS", billed
# monthly over 730 h) and of one standard static public IPv4 address, in
# $LOC. The disk's own meter ("P10 LRS Disk"): the SKU also has a "Disk
# Mount" meter (shared disks) at a few percent of the price, in the same
# unit, which the API may list first.
azure_disk_price() {
  azure_prices "serviceName eq 'Storage' and armRegionName eq '$LOC' and skuName eq '$1' and meterName eq '$1 Disk' and priceType eq 'Consumption'" \
    | awk -F'\t' -v sku="$1" '$2 == sku && $5 == sku " Disk" && $3 ~ /Managed Disks$/ && $4 == "1/Month" && !done { printf "%.6f\n", $1 / 730; done = 1 }'
}
azure_ip_price() {
  azure_prices "serviceName eq 'Virtual Network' and armRegionName eq '$LOC' and meterName eq 'Standard IPv4 Static Public IP' and priceType eq 'Consumption'" \
    | awk -F'\t' '$4 == "1 Hour" && !done { print $1; done = 1 }'
}

# The receiver's roles for a size, from its known topology (fair_roles).
fair_layout_of() {
  # Set by fair_roles; local so the run's own roles are untouched.
  # shellcheck disable=SC2034
  local TOPO HK ISO CPU_IRQ CPU_A CPU_SQ CPU_SPARE CPU_SIB SMT LAYOUT
  TOPO=$(size_topo "$1")
  if [[ -z $TOPO ]]; then
    echo "  $1: topology not known before the run; fair_roles reads it from the host (four cores, or two with SMT, or the run stops)"
    return 0
  fi
  if ! fair_roles 2> /dev/null; then
    echo "  $1: $(fair_roles 2>&1 > /dev/null)"
    return 0
  fi
  local rarch=x86_64
  [[ $ARCH == arm ]] && rarch=aarch64
  echo "  $1: $(size_vcpus "$1") vCPUs, SMT $SMT, layout $LAYOUT"
  echo "  housekeeping CPU $HK (ssh, kernel work, every other interrupt); NIC interrupts CPU $CPU_IRQ; loop CPU $CPU_A; SQPOLL CPU $CPU_SQ; spare ${CPU_SPARE:-none} (no spare-core probe without one)"
  if [[ $LAYOUT == smt-shared ]]; then
    echo "  two cores with SMT: the NIC interrupts share housekeeping's core (CPU $HK's sibling) and the SQPOLL thread shares the loop's (CPU $CPU_A's sibling), as uring-sqpoll-sib measured it in 122127Z; idle CPUs halt (idle=poll would spin a sibling)"
  fi
  echo "  receiver boot profile $(fair_boot "$SMT" "$rarch") (isolated: $ISO)"
}

# vCPUs of size $1: the known table, else as Azure lists it in $LOC
# (read-only); fails when neither gives a number, so no quota check passes
# on a guess.
azure_vcpus() {
  local n
  n=$(size_vcpus "$1")
  [[ -n $n ]] || n=$(az vm list-skus -l "$LOC" --size "$1" --resource-type virtualMachines \
    --query "[?name=='$1'] | [0].capabilities[?name=='vCPUs'] | [0].value" -o tsv)
  [[ $n =~ ^[0-9]+$ ]] || { echo "vCPUs of $1 not known" >&2; return 1; }
  echo "$n"
}

# Azure's quota for this platform: vCPUs $1 of the regional spot quota
# (Spot) or of the regional and the size family's (Regular). Read-only.
azure_quota_check() {
  local need=$1 used limit fam
  if [[ $PRIORITY == Spot ]]; then
    read -r used limit < <(az vm list-usage -l "$LOC" --query "[?name.value=='lowPriorityCores'] | [0].[currentValue, limit]" -o tsv)
    log "quota: spot vCPUs in $LOC: $used of $limit used, $need needed"
    (( limit - used >= need )) || { log "!!! not enough spot vCPU quota in $LOC: nothing created"; return 1; }
    return 0
  fi
  read -r used limit < <(az vm list-usage -l "$LOC" --query "[?name.value=='cores'] | [0].[currentValue, limit]" -o tsv)
  log "quota: regional vCPUs in $LOC: $used of $limit used, $need needed"
  (( limit - used >= need )) || { log "!!! not enough regional vCPU quota in $LOC: nothing created"; return 1; }
  for fam in $(az vm list-skus -l "$LOC" --resource-type virtualMachines --query "[?name=='$RCV_SIZE' || name=='$SND_SIZE'].family" -o tsv | sort -u); do
    read -r used limit < <(az vm list-usage -l "$LOC" --query "[?name.value=='$fam'] | [0].[currentValue, limit]" -o tsv)
    log "quota: $fam: $used of $limit used"
  done
  return 0
}

# The fair run's plan and estimate for this platform, with no cloud call.
azure_fair_plan() {
  local pts_s hours rcv_s rcv_r snd_s snd_r rdisk sdisk ip need
  pts_s=$(fair_points_s)
  hours=$(awk -v s="$EST_S" 'BEGIN { printf "%.2f", s / 3600 }')
  need=$(( $(size_vcpus "$RCV_SIZE" || echo 0) + $(size_vcpus "$SND_SIZE" || echo 0) ))
  echo "# Fair run plan, Azure $ARCH (run $STAMP)"
  echo "== platform"
  echo "  region $LOC; resource group $RG (deleted on every exit, verified gone); tags owner=ergon-lab run=$STAMP platform=$ARCH"
  echo "  receiver $RCV_SIZE, sender $SND_SIZE, one proximity placement group, accelerated networking (MANA), image $IMAGE"
  echo "  priority $PRIORITY$([[ $PRIORITY == Spot ]] && echo ", eviction policy Delete: an eviction ends this platform and its teardown deletes the group")"
  echo "  quota needed: $need vCPUs ($([[ $PRIORITY == Spot ]] && echo "regional spot quota, lowPriorityCores" || echo "regional and family quota")), checked before anything is created"
  echo "  MANA interrupt moderation: kernel 6.17.0-1022-azure, which every earlier Azure run used, has no setting (ethtool -c unsupported, env-rcv.txt of 045348Z and 144146Z): moderation did not slow epoll there; mainline Linux added MANA interrupt moderation, adaptive RX on by default where the hardware has it (2026-07), and RX CQE coalescing, off by default (2026-03): this run turns both off if its kernel has them and records what it found"
  echo "== receiver CPU layout"
  fair_layout_of "$RCV_SIZE"
  echo "  sender $SND_SIZE: housekeeping on its first CPU (its NIC interrupts too), the server loop on the second"
  fair_plan
  echo "== estimate"
  echo "  measuring $(( $(fair_measure_s) / 60 )) min; with about ${FAIR_POINT_OVERHEAD_S} s around each point $(( pts_s / 60 )) min; set-up $(( FAIR_SETUP_S / 60 )) min; analysis, copy and deleting the group $(( FAIR_FINISH_S / 60 )) min"
  echo "  total about $(( EST_S / 60 )) min ($hours h); deadline (watchdog) $(( MAX_S / 60 )) min; the run detaches (nohup) when its deadline is over 2 h"
  rcv_s=$(azure_vm_price "$RCV_SIZE" Spot); rcv_r=$(azure_vm_price "$RCV_SIZE" Regular)
  snd_s=$(azure_vm_price "$SND_SIZE" Spot); snd_r=$(azure_vm_price "$SND_SIZE" Regular)
  rdisk=$(azure_disk_price "$FAIR_RCV_DISK_SKU"); sdisk=$(azure_disk_price "$FAIR_SND_DISK_SKU"); ip=$(azure_ip_price)
  if [[ -z $rcv_s || -z $rcv_r || -z $snd_s || -z $snd_r || -z $rdisk || -z $sdisk || -z $ip ]]; then
    echo "  cost: prices not available (retail prices API: receiver ${rcv_s:-?}/${rcv_r:-?}, sender ${snd_s:-?}/${snd_r:-?}, disks ${rdisk:-?}/${sdisk:-?}, IP ${ip:-?} USD/h)"
    return 0
  fi
  awk -v h="$hours" -v mh="$(awk -v s="$MAX_S" 'BEGIN { printf "%.2f", s / 3600 }')" -v rs="$rcv_s" -v rr="$rcv_r" -v ss="$snd_s" -v sr="$snd_r" \
    -v rd="$rdisk" -v sd="$sdisk" -v ip="$ip" -v r="$RCV_SIZE" -v s="$SND_SIZE" -v p="$PRIORITY" \
    -v rdk="$FAIR_RCV_DISK_SKU, $FAIR_RCV_DISK_GB GB" -v sdk="$FAIR_SND_DISK_SKU, $FAIR_SND_DISK_GB GB" 'BEGIN {
    fixed = rd + sd + 2 * ip
    printf "  prices (USD/h, Linux, retail): %s spot %.4f, regular %.4f; %s spot %.4f, regular %.4f; OS disks: receiver (%s) %.4f, sender (%s) %.4f; 2 public IPs %.4f\n", r, rs, rr, s, ss, sr, rdk, rd, sdk, sd, 2 * ip
    printf "  cost for %.2f h: spot %.2f USD (%.4f/h), regular %.2f USD (%.4f/h); this run: %s\n", h, h * (rs + ss + fixed), rs + ss + fixed, h * (rr + sr + fixed), rr + sr + fixed, p
    printf "  at most (stopped by the deadline, %.2f h): spot %.2f USD, regular %.2f USD\n", mh, mh * (rs + ss + fixed), mh * (rr + sr + fixed)
  }'
}
