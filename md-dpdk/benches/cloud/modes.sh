# Sourced by aws/lab.sh and azure/lab.sh (each mode checked before any cloud
# call), lib.sh and vm.sh: a mode's settings, read alike everywhere. A mode is
# <transport>@KEY=value@..., optionally ending +idle (lib.sh: an idle
# localhost subscription held throughout). Functions only, so test/test-lab.sh
# calls them directly.
#
# The raw floor's transports, kernel-raw and dpdk-raw (-mono too), run no
# Aeron at all: the harness's floor client (md-dpdk/harness/src/bin/floor.rs)
# does rtt's round trip over kernel UDP sockets or the DPDK transport's
# bindings alone, on kernel-busy's or dpdk's host set-up (raw_base), so a
# state's idle round trip splits into what is under Aeron and Aeron's own.
# Cross-host only (or, @HAIRPIN=1, on host a between its own two NICs), the
# round trip alone (lib.sh), with host settings and RUSTERON_* ones; anything
# only Aeron reads is refused (mode_check).
#
# The settings here (the rest are vm.sh host_setting's, or the driver's
# environment):
#   AERON_THREADING_MODE=DEDICATED|SHARED_NETWORK|SHARED|INVOKER  the driver's
#        agents (host.sh threading_env). INVOKER: no media driver process
#        (on Graviton under DPDK one, briefly, for its LLQ check: host.sh
#        invoker_llq_probe); each benchmark client embeds the driver
#        (HARNESS_DRIVER=invoker) and runs its own work, the client
#        conductor's and the driver's on its one pinned thread. Cross-host
#        only: two clients on one host would each embed a driver on one
#        AERON_DIR (localhost, +idle)
#   JUMBO=1   jumbo frames (AWS): the data NICs at 9001 bytes (8500 where the
#        path crosses inter-Region VPC peering: AWS's EC2 user guide, "Network
#        maximum transmission unit (MTU) for your EC2 instance": "MTU limit
#        8500 bytes: Traffic over an inter-region VPC peering connection";
#        a larger packet is dropped when Don't Fragment is set, as Aeron's
#        are), Aeron's MTU 8960 (8448), the DPDK transport's likewise
#        (RUSTERON_DPDK_MAX_AERON_MTU; its L3 MTU is that plus 28)
#   SMT=off   (x86) the SMT siblings offline for the mode (vm.sh)
#   PUB_PARAMS=<key>=<value>+...  Aeron URI parameters on every publication
#        channel the clients create (PUB_CHANNEL_PARAMS), and
#   SUB_PARAMS=<key>=<value>+...  on every subscription channel
#        (SUB_CHANNEL_PARAMS). + stands for Aeron's |: a | would start a pipe
#        in the ssh command lines that carry a mode, and a comma would split
#        its results.csv rows; = and + are safe in both and in file names.
#   IRQ=hk|receiver|spare  (kernel modes, AWS) the data NICs' queue IRQs (ENA's
#        <interface>-Tx-Rx-<n>) on one CPU for the mode: hk the housekeeping
#        CPU (every IRQ's otherwise: the baseline), receiver the Aeron
#        receiver agent's (AWS's ENA guidance: interrupts on the
#        application's vCPU; host.sh setting_cpu by threading mode), spare the
#        layout's fifth core (none on a four-core host; not with both ends on
#        one host, whose second end takes it); pinned there again after every
#        change to the NICs in the mode, back on housekeeping after it; the
#        check on the run (isolated-nic-irqs) counts theirs off that CPU
#   NAPI_DEFER=<n>/<ns>  (kernel modes) the data NICs' napi_defer_hard_irqs
#        and gro_flush_timeout (ns) for the mode (kernel-busy's are 2 and
#        200000 otherwise, kernel's 0 and 0), put back after it
#   NAPI_BUSY=spare  (kernel modes, Linux 6.17 and up) threaded NAPI busy
#        polling on every NAPI of the data NICs, their kthreads on the
#        layout's fifth core, which they spin at 100% (vm.sh; the kernel's
#        ynl, vm.sh build-ynl); threaded disabled again after the mode
#   HAIRPIN=1 (the raw floor's modes, AWS) the round trip on host a alone,
#        between its own two data NICs (ENIs on one subnet): the floor
#        client's xhairpin, ping and pong in one process, the ping on the app
#        core and its sender NIC, the pong on the receiver core (which the
#        raw floor leaves free) and its receiver NIC, so a packet leaves by
#        one NIC and comes back by the other through the host's own Nitro
#        card, never between hosts: host b idles. Against the cross-host
#        floor of the same mode it tells the local card's path from the
#        fabric. Host a needs four cores (hairpin_types_check). On the kernel
#        wire vm.sh routes each end's sends out of its own NIC (the kernel
#        would deliver its own address over loopback), checked fail closed
#   RTT_GAP_US=<us>  (1 to 1000000) one ping every that many microseconds,
#        send to send, the warm-up's too, the clients spinning in between
#        (rtt's and floor's Pace): a sparse flow, the network quiet between
#        round trips, as an idle Aeron channel is between its 100 ms
#        heartbeats. A rep then runs 20 to 60 s, about 10,000 round trips
#        where a minute holds them (rtt_plan): 20 s and 20,000 at 1 ms, 60 s
#        and 1,200 at 50 ms, 300 at 200 ms, 60 at 1 s. Cross-host only
#   CONDUCTOR=own  the driver's conductor on the layout's fifth core, alone,
#        spinning (AERON_CONDUCTOR_CPU_AFFINITY, AERON_CONDUCTOR_IDLE_STRATEGY
#        =spin), the DPDK transport's housekeeping and local-I/O threads kept
#        on the housekeeping CPU; checked at the driver's start (vm.sh, fail
#        closed). Cross-host only, under DEDICATED or SHARED_NETWORK (SHARED's
#        conductor is the shared agent; INVOKER has no conductor thread), never
#        with IRQ=spare or NAPI_BUSY=spare, which would share its core
# Any other upper-case @KEY must be an environment variable of the driver's,
# AERON_* or RUSTERON_*: anything else would be exported to a driver that never
# reads it, the mode measuring its baseline under another name (mode_check).
#
# The Aeron 1.53.3 URI parameters that matter over a long round trip (from
# its aeron_uri.h and the driver's aeron_driver_uri.c, which reads each into
# a publication's or a subscription's parameters; the driver-wide defaults
# are the AERON_* environment variables named, settable in a mode as
# @AERON_...=value):
#   publication side
#     term-length  each of the publication's three term buffers (power of 2,
#                  64 KiB to 1 GiB; AERON_TERM_BUFFER_LENGTH, 16 MiB). The
#                  receiver window is capped at half a term
#                  (aeron_receiver_window_length), so a bandwidth-delay
#                  product window needs a term of at least twice it
#     mtu          the largest frame (32-aligned, <= so-sndbuf;
#                  AERON_MTU_LENGTH, 1408 here). Over a long path a larger
#                  frame halves the frames per byte, but must fit the path's
#                  MTU (JUMBO=1 sets both)
#     pub-wnd      how far the publisher may run ahead of the sender
#                  (AERON_PUBLICATION_TERM_WINDOW_LENGTH; at most half a term)
#     linger       how long a closed publication keeps its log for
#                  retransmits (AERON_PUBLICATION_LINGER_TIMEOUT)
#     max-resend   retransmits in flight at once (1 to 16)
#     so-sndbuf    the send socket's buffer (AERON_SOCKET_SO_SNDBUF, 2 MiB
#                  here), capped by net.core.wmem_max (16 MiB, vm.sh
#                  bootstrap); kernel UDP only, the DPDK transport has none
#     so-rcvbuf    the send endpoint's receive buffer (status messages, NAKs)
#     sparse, eos, ssc, fc, untethered-*  log sparseness, end-of-stream,
#                  spies simulating a connection, flow control, untethered
#                  subscriptions' timeouts
#   subscription side
#     rcv-wnd      the receiver window: how much the receiver lets the sender
#                  have in flight (AERON_RCV_INITIAL_WINDOW_LENGTH, 2 MiB
#                  here). One window a round trip: a long path needs one of
#                  its bandwidth-delay product, at most half the term
#                  (above), and at most so-rcvbuf (Aeron refuses the
#                  subscription otherwise). At low RTT the lab sizes it to the
#                  NIC's receive ring (ring x smallest datagram); a
#                  bandwidth-delay product needs a deeper ring as well
#                  (@RX_RING=<n> kernel, @RUSTERON_DPDK_RX_DESCRIPTORS=<n>
#                  DPDK), which a mode may set beside it
#     so-rcvbuf    the receive socket's buffer (AERON_SOCKET_SO_RCVBUF),
#                  capped by net.core.rmem_max (16 MiB); kernel UDP only
#     cc           the receiver's congestion control: static (the default:
#                  the window as set) or cubic, which measures the round trip
#                  and grows the window from it
#                  (AERON_CONGESTIONCONTROL_SUPPLIER,
#                  AERON_CUBICCONGESTIONCONTROL_* for cubic's RTT measurement)
#     nak-delay    how long the receiver waits on a gap before NAKing it
#                  (AERON_NAK_UNICAST_DELAY: 1 us in Aeron 1.53.3's C driver,
#                  its minimum, aeron_driver_context.c; each retry
#                  AERON_NAK_UNICAST_RETRY_DELAY_RATIO times it, 100: every
#                  100 us). The sender's side of a NAK is
#                  AERON_RETRANSMIT_UNICAST_DELAY (0) and
#                  AERON_RETRANSMIT_UNICAST_LINGER (10 ms), driver-wide only
#     reliable     false: no NAKs, gaps filled (vm.sh RELIABLE=false sets it
#                  with group and tether)
#     media-rcv-ts-offset  where Aeron's image writes each data frame's media
#                  receive time (8 bytes, CLOCK_REALTIME ns: the kernel's
#                  SO_TIMESTAMPNS on its UDP transport; the DPDK transport's
#                  read once a receive burst, or the NIC's with
#                  RUSTERON_DPDK_RX_HW_TIMESTAMP=1: md-dpdk/README.md), as
#                  the frame goes into the image (aeron_publication_image.c,
#                  aeron_timestamps.c). The lab's media-receive-timestamp
#                  mode is dpdk@SUB_PARAMS=media-rcv-ts-offset=reserved: the
#                  data frame header's reserved value, which no client reads
#                  (rtt and load read bytes 0 to 15 of a message, the send
#                  stamp and the sequence, and echo the rest; tput reads
#                  bytes 0 to 7, its sequence; none reads a header's reserved
#                  value, and an echo's publication writes its own), and
#                  which every frame has whatever its size. A payload offset
#                  is taken from 16 to 24 only: lower, Aeron would overwrite
#                  the stamp or the sequence a round trip is measured by (the
#                  harness's invoker test shows it would); higher, rtt's
#                  32-byte messages would not fit it and Aeron writes none
#                  (aeron_timestamps_set_timestamp). channel_params refuses
#                  anything else
#     tether, group, rejoin, sparse, gtag, untethered-*
#   Aeron ignores a parameter it does not know, so a misspelt one would run
#   the mode untuned and unsaid: channel_params refuses it, and one on the
#   wrong side, where it does nothing.
#
# Reads nothing from the host: lab.sh runs it on the operator's machine.

# mode_pairs <mode>: a mode's overrides, one KEY=value a line (no +idle suffix)
mode_pairs() {
    local m=${1%+idle} p
    local IFS=@
    set -f
    # shellcheck disable=SC2086
    set -- $m
    set +f
    shift
    for p in "$@"; do printf '%s\n' "$p"; done
}

# mode_raw <mode>: 0 for the raw floor's modes (kernel-raw, dpdk-raw, -mono)
mode_raw() {
    local t=${1%+idle}
    t=${t%%@*}
    t=${t%-mono}
    [[ $t == kernel-raw || $t == dpdk-raw ]]
}

# mode_hairpin <mode>: 0 for a same-host hairpin (@HAIRPIN=1)
mode_hairpin() {
    [[ $(mode_get "${1%+idle}" HAIRPIN) == 1 ]]
}

# rtt_plan <mode> <messages> <warmup>: "<seconds> <messages> <warmup>" for
# its round trip (RTT_SECONDS, then the ping's round trips and warm-up): 20 s
# and as given, unless the mode paces its pings (@RTT_GAP_US=<us>): then as
# long as 10,000 round trips take (p99.99 needs them), from 20 s to 60 s; as
# many round trips as fit in that (at most the messages given), and about
# 2 s of warm-up pings (at least one, at most the warm-up given)
rtt_plan() {
    local gap seconds messages warmup
    gap=$(mode_get "${1%+idle}" RTT_GAP_US) || { echo "20 $2 $3"; return 0; }
    seconds=$(((10000 * gap + 999999) / 1000000))
    ((seconds >= 20)) || seconds=20
    ((seconds <= 60)) || seconds=60
    messages=$((seconds * 1000000 / gap))
    ((messages <= $2)) || messages=$2
    warmup=$(((2000000 + gap - 1) / gap))
    ((warmup <= $3)) || warmup=$3
    echo "$seconds $messages $warmup"
}

# hairpin_types_check <mode> <aws|azure> <type a>: 0, or 2 saying why, for a
# hairpin (@HAIRPIN=1) on a type a with fewer than four physical cores (the
# layout's housekeeping, app (ping), sender and receiver (pong) cores); 0
# for any other mode
hairpin_types_check() {
    local cores
    mode_hairpin "$1" || return 0
    cores=$(type_cores "$2" "$3" 2>&1) || { echo "mode $1: $cores" >&2; return 2; }
    ((cores >= 4)) ||
        { echo "mode $1: $3 has $cores physical cores: a hairpin's ping and pong need the layout's four cores on host a (housekeeping, app, sender, receiver: c9g.xlarge and up)" >&2; return 2; }
}

# mode_client <mode>: the round trip's client: floor for a raw floor mode,
# else rtt
mode_client() {
    if mode_raw "$1"; then echo floor; else echo rtt; fi
}

# raw_base <mode>: the transport whose host set-up a raw floor mode runs on
# (vm.sh driver): kernel-busy's (busy_read and busy_poll, NAPI deferral) for
# kernel-raw, dpdk's (igb_uio, the ports' environment) for dpdk-raw; nothing
# for any other mode
raw_base() {
    mode_raw "$1" || return 0
    local t=${1%%@*}
    if [[ ${t%-mono} == kernel-raw ]]; then echo kernel-busy; else echo dpdk; fi
}

# mode_get <mode> <KEY>: the value of KEY's last override; 1 when it has none
mode_get() {
    local p v= found=1
    while IFS= read -r p; do
        if [[ $p == "$2="* ]]; then v=${p#*=}; found=0; fi
    done < <(mode_pairs "$1")
    printf '%s' "$v"
    return $found
}

# The URI parameters the lab knows, by side (see above)
PUB_PARAM_KEYS=" term-length mtu pub-wnd linger max-resend so-sndbuf so-rcvbuf sparse eos ssc fc untethered-window-limit-timeout untethered-linger-timeout untethered-resting-timeout "
SUB_PARAM_KEYS=" rcv-wnd so-rcvbuf so-sndbuf cc nak-delay reliable tether group rejoin sparse gtag untethered-window-limit-timeout untethered-linger-timeout untethered-resting-timeout media-rcv-ts-offset "

# channel_params <pub|sub> <key>=<value>+...: the Aeron parameter string
# (key=value|key=value) for the clients' publications or subscriptions; 2,
# saying why, for a malformed one, a key the lab does not know, one on the
# other side, or a key given twice
channel_params() {
    local side=$1 spec=$2 own other p key out= seen=" "
    case $side in
    pub) own=$PUB_PARAM_KEYS other=$SUB_PARAM_KEYS ;;
    sub) own=$SUB_PARAM_KEYS other=$PUB_PARAM_KEYS ;;
    *) echo "channel_params: pub or sub, not $side" >&2; return 2 ;;
    esac
    local name=PUB_PARAMS
    [[ $side == sub ]] && name=SUB_PARAMS
    if [[ ! $spec =~ ^[a-z-]+=[A-Za-z0-9._-]+(\+[a-z-]+=[A-Za-z0-9._-]+)*$ ]]; then
        echo "$name=$spec: <key>=<value>+<key>=<value>... (Aeron URI parameters, + for Aeron's |; values of letters, digits, . _ -)" >&2
        return 2
    fi
    local IFS=+
    for p in $spec; do
        key=${p%%=*}
        if [[ $own != *" $key "* ]]; then
            if [[ $other == *" $key "* ]]; then
                echo "$name: $key is a $([[ $side == pub ]] && echo "subscription's" || echo "publication's") parameter (Aeron reads it there only)" >&2
            else
                echo "$name: $key is not an Aeron URI parameter the lab knows (Aeron ignores one it does not know):$own" >&2
            fi
            return 2
        fi
        [[ $seen != *" $key "* ]] || { echo "$name: $key given twice" >&2; return 2; }
        seen+="$key "
        # where Aeron writes each message's receive time (see above): never
        # over what the clients read, and within rtt's message
        if [[ $key == media-rcv-ts-offset && ! ${p#*=} =~ ^(reserved|1[6-9]|2[0-4])$ ]]; then
            echo "$name: $p: reserved (the frame header's reserved value, which no client reads) or a payload offset from 16 to 24: the clients read bytes 0 to 15 of every message (the send stamp and the sequence), which an 8-byte receive time at a lower offset would overwrite, and at a higher one rtt's 32-byte messages would go unstamped (Aeron writes none that does not fit)" >&2
            return 2
        fi
        out+=${out:+|}$p
    done
    printf '%s' "$out"
}

# aeron_size <value>: a size as Aeron's URI parameters and AERON_* variables
# take it (aeron_parse_size64: a number, k, m or g for powers of 1024), in
# bytes; 2 for anything else
aeron_size() {
    [[ $1 =~ ^([0-9]+)([kKmMgG]?)$ ]] || { echo "size $1: a number of bytes, optionally k, m or g" >&2; return 2; }
    local n=${BASH_REMATCH[1]} unit=${BASH_REMATCH[2]}
    case $unit in
    k | K) n=$((n * 1024)) ;;
    m | M) n=$((n * 1048576)) ;;
    g | G) n=$((n * 1073741824)) ;;
    esac
    echo "$n"
}

# channel_buffers_check <mode>: 0, or 2 with why on stderr, for what Aeron
# refuses of a subscription (aeron_driver_conductor.c: a receiver window,
# rcv-wnd, over its socket's receive buffer: the channel's so-rcvbuf, else
# AERON_SOCKET_SO_RCVBUF, vm.sh's 2 MiB unless the mode sets it, a number
# only under DPDK, which has no socket), and in a kernel mode a socket
# buffer over vm.sh bootstrap's net.core.rmem_max and wmem_max (16 MiB),
# which the kernel would cap unsaid
channel_buffers_check() {
    local m=$1 pub sub key v wnd buf why
    pub=$(mode_get "$m" PUB_PARAMS) || pub=
    sub=$(mode_get "$m" SUB_PARAMS) || sub=
    if [[ ${m%%@*} != dpdk* ]]; then
        for key in so-rcvbuf so-sndbuf; do
            for v in "+$pub+" "+$sub+"; do
                [[ $v =~ \+$key=([^+]+)\+ ]] || continue
                v=$(aeron_size "${BASH_REMATCH[1]}") || return 2
                ((v <= 16777216)) ||
                    { echo "mode $m: $key over 16 MiB, net.core's maximum (vm.sh bootstrap): the kernel would cap it unsaid" >&2; return 2; }
            done
        done
    fi
    [[ +$sub+ =~ \+rcv-wnd=([^+]+)\+ ]] || return 0
    wnd=$(aeron_size "${BASH_REMATCH[1]}") || return 2
    if [[ +$sub+ =~ \+so-rcvbuf=([^+]+)\+ ]]; then
        buf=$(aeron_size "${BASH_REMATCH[1]}") why="its so-rcvbuf"
    elif v=$(mode_get "$m" AERON_SOCKET_SO_RCVBUF); then
        buf=$(aeron_size "$v") why="its AERON_SOCKET_SO_RCVBUF"
    else
        buf=2097152 why="AERON_SOCKET_SO_RCVBUF, vm.sh's 2 MiB (SUB_PARAMS so-rcvbuf= raises it)"
    fi
    ((wnd <= buf)) ||
        { echo "mode $m: rcv-wnd $wnd bytes is over $why, $buf: Aeron refuses the subscription (initial window greater than SO_RCVBUF)" >&2; return 2; }
}

# jumbo_mtus <placement>: "<data NIC MTU> <Aeron MTU>" under JUMBO=1: 9001
# and 8960 (8960 + 28 bytes of IP and UDP fit, 32-aligned), or across
# inter-Region VPC peering (cross-region) 8500 and 8448
jumbo_mtus() {
    if [[ $1 == cross-region ]]; then echo "8500 8448"; else echo "9001 8960"; fi
}

# mode_mtu <mode> <placement>: the MTU the mode's publications use: a channel
# mtu (PUB_PARAMS), else @AERON_MTU_LENGTH, else jumbo frames' (JUMBO=1),
# else vm.sh common_env's 1408
mode_mtu() {
    local mode=$1 v
    if v=$(mode_get "$mode" PUB_PARAMS) && [[ +$v+ =~ \+mtu=([0-9]+)\+ ]]; then
        echo "${BASH_REMATCH[1]}"
    elif v=$(mode_get "$mode" AERON_MTU_LENGTH); then
        echo "$v"
    elif [[ $(mode_get "$mode" JUMBO) == 1 ]]; then
        local mtus
        mtus=$(jumbo_mtus "$2")
        echo "${mtus#* }"
    else
        echo 1408
    fi
}

# mode_payload <mode> <placement>: its largest message, the MTU less Aeron's
# 32-byte data header (the clients refuse a larger one: it would fragment)
mode_payload() {
    echo $(($(mode_mtu "$1" "$2") - 32))
}

# mode_mtu_check <mode> <placement> <cloud>: 0, or 2 with why on stderr, when
# the mode's MTU (mode_mtu) would not get its frames through: Aeron's frames
# are 32-aligned; the DPDK transport refuses an MTU over its
# RUSTERON_DPDK_MAX_AERON_MTU at install (a channel's mtu, its frames at
# send), which is 1408, jumbo frames' or the mode's own; and the path drops
# what it cannot carry, Don't Fragment set: the kernel's data NICs at 1500
# bytes without JUMBO=1 (Azure's too), inter-Region VPC peering's 8500, a
# VPC's 9001 (the DPDK transport sets its port's MTU itself)
mode_mtu_check() {
    local m=$1 placement=$2 cloud=$3 mtu most why
    mtu=$(mode_mtu "$m" "$placement")
    [[ $mtu =~ ^[0-9]+$ ]] || { echo "mode $m: MTU $mtu: a number of bytes" >&2; return 2; }
    ((mtu % 32 == 0)) || { echo "mode $m: MTU $mtu is not a multiple of 32 (Aeron's frame alignment)" >&2; return 2; }
    if [[ ${m%%@*} == dpdk* ]]; then
        if most=$(mode_get "$m" RUSTERON_DPDK_MAX_AERON_MTU); then
            [[ $most =~ ^[0-9]+$ ]] || { echo "mode $m: RUSTERON_DPDK_MAX_AERON_MTU=$most: a number of bytes" >&2; return 2; }
            why="its RUSTERON_DPDK_MAX_AERON_MTU"
        elif [[ $(mode_get "$m" JUMBO) == 1 ]]; then
            most=$(jumbo_mtus "$placement")
            most=${most#* } why="jumbo frames' RUSTERON_DPDK_MAX_AERON_MTU"
        else
            most=1408 why="the DPDK transport's RUSTERON_DPDK_MAX_AERON_MTU (1408 unless the mode sets it, or JUMBO=1)"
        fi
        ((mtu <= most)) || { echo "mode $m: MTU $mtu is over $why, $most: the transport refuses it" >&2; return 2; }
        if [[ $cloud == azure ]]; then
            most=1472 why="Azure's 1500-byte path"
        else
            most=$(jumbo_mtus "$placement")
            most=${most#* } why="the path's $(jumbo_mtus "$placement" | cut -d' ' -f1) bytes"
        fi
    elif [[ $(mode_get "$m" JUMBO) == 1 ]]; then
        most=$(jumbo_mtus "$placement")
        most=${most#* } why="the jumbo MTU's $(jumbo_mtus "$placement" | cut -d' ' -f1) bytes"
    else
        most=1472 why="the data NICs' 1500 bytes (JUMBO=1 raises them)"
    fi
    ((mtu <= most)) || { echo "mode $m: MTU $mtu is over $most, $why less IP and UDP: its larger frames would be dropped" >&2; return 2; }
}

# spare_takers <mode>: the mode's settings that take the layout's fifth core
# (IRQ=spare, NAPI_BUSY=spare, CONDUCTOR=own), space-separated; empty for none
spare_takers() {
    local out=
    [[ $(mode_get "$1" IRQ) != spare ]] || out+=" IRQ=spare"
    [[ $(mode_get "$1" NAPI_BUSY) != spare ]] || out+=" NAPI_BUSY=spare"
    [[ $(mode_get "$1" CONDUCTOR) != own ]] || out+=" CONDUCTOR=own"
    printf '%s' "${out# }"
}

# mode_spare <mode>: 0 when the mode takes the layout's fifth core
# (spare_takers): lib.sh runs no second flow there, and vm.sh ready asks for one
mode_spare() {
    [[ -n $(spare_takers "$1") ]]
}

# Physical cores by instance type, for the settings that take the layout's
# fifth core (spare_takers): vm.sh ready finds a host without one only after
# the build, an hour in, so lab.sh asks this first, before any cloud call.
# AWS: the size's vCPUs (medium 1, large 2, xlarge 4, <n>xlarge and
# metal-<n>xl 4n; a family's bare metal, whose count varies, has none here)
# over the family's threads a core: two for Intel's (c5, c6i, c7i, c8i and
# kin) and AMD's before C7a, one for Graviton's (every vCPU a core) and AMD's
# from C7a on. Azure: the D, E and F series' v5 and v6 sizes
# (Standard_D8s_v6: 8 vCPUs), two threads a core. Anything else unknown.
AWS_TWO_THREADS=" c5 c5a c5ad c5d c5n c6a c6i c6id c6in c7i c7i-flex c8i c8i-flex m5 m5n m6a m6i m6in m7i m7i-flex m8i r5 r6a r6i r7i r8i "
AWS_ONE_THREAD=" c7a m7a r7a c8a m8a r8a "

# type_cores <aws|azure> <type>: its physical cores; 1, saying why, when the
# table above does not know it
type_cores() {
    local cloud=$1 type=$2 family size vcpus threads
    case $cloud in
    aws)
        family=${type%%.*} size=${type#*.}
        if [[ $family =~ ^[a-z]+[0-9]+g[a-z]*$ || $family == a1 ]]; then
            threads=1
        elif [[ $AWS_TWO_THREADS == *" $family "* ]]; then
            threads=2
        elif [[ $AWS_ONE_THREAD == *" $family "* ]]; then
            threads=1
        else
            echo "$type: no core count for family $family in modes.sh's table (Intel's c5, c6i, c7i, c8i and kin two threads a core; Graviton's and AMD's C7a and later one): give a type it knows" >&2
            return 1
        fi
        [[ $type == *.* ]] || { echo "$type: no size (a type is <family>.<size>)" >&2; return 1; }
        case $size in
        medium) vcpus=1 ;;
        large) vcpus=2 ;;
        xlarge) vcpus=4 ;;
        *)
            if [[ $size =~ ^([1-9][0-9]{0,2})xlarge$ ]]; then
                vcpus=$((BASH_REMATCH[1] * 4))
            elif [[ $size =~ ^metal-([1-9][0-9]{0,2})xl$ ]]; then
                vcpus=$((BASH_REMATCH[1] * 4))
            else
                echo "$type: no vCPU count for size $size in modes.sh's table (medium, large, xlarge, <n>xlarge, metal-<n>xl)" >&2
                return 1
            fi
            ;;
        esac
        ;;
    azure)
        [[ $type =~ ^Standard_[DEF]([1-9][0-9]{0,2})[a-z]*_v[56]$ ]] ||
            { echo "$type: no core count for it in modes.sh's table (the D, E and F series' v5 and v6 sizes)" >&2; return 1; }
        vcpus=${BASH_REMATCH[1]} threads=2
        ;;
    *)
        echo "type_cores: aws or azure, not $cloud" >&2
        return 1
        ;;
    esac
    echo $((vcpus / threads))
}

# spare_types_check <mode> <aws|azure> <type>...: 0, or 2 saying why, for a
# mode that takes the layout's fifth core (spare_takers) on a type with
# fewer than five physical cores, or one type_cores does not know; 0 for
# any other mode, whatever the types
spare_types_check() {
    local m=$1 cloud=$2 takers t cores more
    shift 2
    takers=$(spare_takers "$m")
    [[ -n $takers ]] || return 0
    more="c7i.4xlarge, c8g.2xlarge and up have five or more"
    [[ $cloud == azure ]] && more="Standard_D16s_v6 and up have five or more"
    for t in "$@"; do
        if ! cores=$(type_cores "$cloud" "$t" 2>&1); then
            echo "mode $m takes the layout's fifth core ($takers): $cores" >&2
            return 2
        fi
        ((cores >= 5)) || { echo "mode $m takes the layout's fifth core ($takers): $t has $cores physical cores ($more)" >&2; return 2; }
    done
}

# precision-time's types: the families AWS places in a precision-time
# placement group, each of which has the ENA's PTP hardware clock there (AWS's
# EC2 user guide, "Access the enhanced Amazon Time Sync Service": general
# purpose M7a, M7g, M7gd, M7i, M7i-flex, M8a, M8g, M8i, M8i-flex, M9g, M9gd;
# compute optimised C7a, C7gd, C7i, C7i-flex, C8a, C8g, C8gd, C8i, C8i-flex,
# C9g, C9gd; memory optimised R7a, R7g, R7i, R8a, R8g, R9g, R9gd, X8g, X8i;
# storage optimised I8g, I8ge). A c7g, c7gn, c8gn, c6in or m6i has none: its
# NIC gives no RX timestamps, and chrony no PHC
PHC_FAMILIES=" m7a m7g m7gd m7i m7i-flex m8a m8g m8i m8i-flex m9g m9gd c7a c7gd c7i c7i-flex c8a c8g c8gd c8i c8i-flex c9g c9gd r7a r7g r7i r8a r8g r9g r9gd x8g x8i i8g i8ge "
# phc_type <type>: 0 when its family has the PTP hardware clock
phc_type() { [[ $PHC_FAMILIES == *" ${1%%.*} "* ]]; }

# precision_modes_check <mode>...: under precision-time vm.sh asks every DPDK
# port for NIC RX timestamps, and requesting them changes the ENA's RX
# descriptors (each completion then carries its timestamp), so a DPDK mode
# there is not the cluster placement's: a run with DPDK modes carries a DPDK
# baseline with them off (@RUSTERON_DPDK_RX_HW_TIMESTAMP=0) beside one with
# them on. 0, or 2 saying which to add
precision_modes_check() {
    local m v on= off=
    for m in "$@"; do
        [[ ${m%%@*} == dpdk* ]] || continue
        v=$(mode_get "$m" RUSTERON_DPDK_RX_HW_TIMESTAMP) || v=1
        case $v in
        0 | [Ff][Aa][Ll][Ss][Ee]) off=${off:-$m} ;;
        *) on=${on:-$m} ;;
        esac
    done
    if [[ -n $on && -z $off ]]; then
        echo "precision-time asks every DPDK port for NIC RX timestamps, which changes the ENA's RX descriptors, so the DPDK modes need a baseline with them off too: add ${on%+idle}@RUSTERON_DPDK_RX_HW_TIMESTAMP=0 beside $on" >&2
        return 2
    fi
    if [[ -n $off && -z $on ]]; then
        echo "precision-time: every DPDK mode has NIC RX timestamps off (@RUSTERON_DPDK_RX_HW_TIMESTAMP=0): add one with them on, the placement's point, beside $off" >&2
        return 2
    fi
}

# The settings vm.sh host_setting takes (a mode's @KEY=value for the host, the
# clients or the driver's placement); any other key must be an environment
# variable the driver reads, AERON_* (Aeron's) or RUSTERON_* (the transport's)
LAB_SETTINGS=" THP BIN CSTATE BUSY_POLL COALESCE RX_RING MIN_FREE_PCT CLOCKSOURCE RELIABLE AGENTS SMT JUMBO PUB_PARAMS SUB_PARAMS IRQ NAPI_DEFER NAPI_BUSY CONDUCTOR HAIRPIN RTT_GAP_US "

# dpdk_setting <mode> <name> <default> <least> <most> <why> [pow2]: the
# mode's RUSTERON_DPDK_<name>, else <default>, when a whole number from
# <least> to <most> (a power of two with pow2); else 2, saying so with <why>
dpdk_setting() {
    local m=$1 name=RUSTERON_DPDK_$2 v
    v=$(mode_get "$m" "$name") || { echo "$3"; return 0; }
    if [[ $v =~ ^[0-9]{1,10}$ ]] && ((10#$v >= $4 && 10#$v <= $5)) && { [[ -z ${7:-} ]] || (((10#$v & (10#$v - 1)) == 0)); }; then
        echo $((10#$v))
        return 0
    fi
    echo "mode $m: $name=$v: $4 to $5${7:+, a power of two} ($6)" >&2
    return 2
}

# dpdk_values_check <mode> <aws|azure> <placement>: 0, or 2 saying why, for a
# DPDK mode's RUSTERON_DPDK_* values that md-dpdk's config.rs refuses at
# install (DpdkTransportConfig::validate: rings 64 to 8192, a mempool cache
# of 512 at most, bursts 1 to 256, mbufs for both rings, four bursts and two
# caches, an Aeron MTU 32-aligned up to 8960; env.rs: a timestamp flag 0, 1,
# true or false) or the NIC refuses at start (its PMD's ring limits, each a
# power of two: ENA's receive 8192, its low-latency transmit queues 256
# under the transport's large LLQ entries and 512 under any other LLQ policy
# (@RUSTERON_DPDK_ENA_DEVARGS=llq_policy=1 or 2), as 1024 made DPDK refuse the
# queue; MANA's 256 each), and NIC RX timestamps where the NIC offers none
# (anywhere but AWS's precision-time placement). The defaults are vm.sh's
# and env.rs's (rings: min(4096, the NIC's) and 256 on AWS, 256 and 256 on
# Azure; 16384 mbufs a port) and config.rs's (bursts of 32, a mempool cache
# of 256)
dpdk_values_check() {
    local m=$1 cloud=$2 placement=${3:-} nic=ENA rx_most=8192 tx_most=256 rx_default=4096 tx_default=256 rx tx cache burst mbufs need v
    local ring="ENA's transmit ring holds 256 at most under large LLQ entries (the transport's llq_policy=3; llq_policy=1 takes 512)"
    if v=$(mode_get "$m" RUSTERON_DPDK_ENA_DEVARGS) && [[ ,${v//+/,}, =~ ,llq_policy=0*([12]), ]]; then
        tx_most=512 ring="ENA's transmit ring holds 512 at most under llq_policy=${BASH_REMATCH[1]}"
    fi
    if [[ $cloud == azure ]]; then nic=MANA rx_most=256 tx_most=256 rx_default=256 tx_default=256 ring="MANA's transmit ring holds 256 at most"; fi
    rx=$(dpdk_setting "$m" RX_DESCRIPTORS "$rx_default" 64 "$rx_most" "$nic's receive ring holds $rx_most at most: the transport refuses more at start" pow2) || return 2
    tx=$(dpdk_setting "$m" TX_DESCRIPTORS "$tx_default" 64 "$tx_most" "$ring: the transport refuses more at start" pow2) || return 2
    cache=$(dpdk_setting "$m" MEMPOOL_CACHE 256 0 512 "DPDK's per-lcore mempool cache: the transport's configuration refuses more") || return 2
    burst=$(dpdk_setting "$m" BURST_SIZE 32 1 256 "packets a receive or transmit burst, as the transport's configuration takes them") || return 2
    mbufs=$(dpdk_setting "$m" MBUFS_PER_PORT 16384 1 4294967295 "mbufs in each port's pool") || return 2
    need=$((rx + tx + 4 * burst + 3 * cache))
    ((mbufs >= need)) || {
        echo "mode $m: RUSTERON_DPDK_MBUFS_PER_PORT=$mbufs is below the $need the transport needs: a receive ring of $rx, a transmit ring of $tx, four bursts of $burst and two mempool caches of up to $((3 * cache / 2)) each" >&2
        return 2
    }
    if v=$(mode_get "$m" RUSTERON_DPDK_MAX_AERON_MTU); then
        [[ $v =~ ^[0-9]{1,5}$ ]] && ((10#$v > 0 && 10#$v % 32 == 0 && 10#$v <= 8960)) || {
            echo "mode $m: RUSTERON_DPDK_MAX_AERON_MTU=$v: 32 to 8960, a multiple of 32 (the transport's configuration: Aeron's frames are 32-aligned, and 8960 fills a 9001-byte jumbo frame)" >&2
            return 2
        }
    fi
    if v=$(mode_get "$m" RUSTERON_DPDK_RX_HW_TIMESTAMP); then
        case $v in
        0 | [Ff][Aa][Ll][Ss][Ee]) ;;
        1 | [Tt][Rr][Uu][Ee])
            [[ $cloud == aws && $placement == precision-time ]] || {
                echo "mode $m: RUSTERON_DPDK_RX_HW_TIMESTAMP=$v: NIC RX timestamps only in AWS's precision-time placement (LAB_PLACEMENT=precision-time): elsewhere the NIC offers none, and the transport refuses to start" >&2
                return 2
            }
            ;;
        *)
            echo "mode $m: RUSTERON_DPDK_RX_HW_TIMESTAMP=$v: 0, 1, true or false" >&2
            return 2
            ;;
        esac
    fi
}

# modes_want_ynl <mode>...: 0 when one of them busy-polls its NAPIs
# (NAPI_BUSY), which needs the kernel's ynl on the hosts (vm.sh build-ynl)
modes_want_ynl() {
    local m
    for m in "$@"; do mode_get "$m" NAPI_BUSY >/dev/null && return 0; done
    return 1
}

# The DPDK transport's refusals at install (src/ffi.rs validate_context)
# that a mode's overrides can bring about: the network agent's idle strategy
# must busy-spin (DEDICATED: the sender's and the receiver's;
# SHARED_NETWORK's and SHARED's one agent's; INVOKER's is the client's own,
# noop), and under the modes whose one agent also receives, no send may wait
# out a pacing debt (RUSTERON_DPDK_TX_PACE_SPIN_US 0 or unset)
dpdk_busy_idle() { [[ $1 =~ ^(spin|noop|aeron_idle_strategy_busy_spinning|aeron_idle_strategy_noop)$ ]]; }

# mode_check <mode> <remote|local> <arch: x86_64|arm64> <cloud: aws|azure>
# <threads per core, LAB_THREADS_PER_CORE> [<placement>]: 0, or 2 with why
# on stderr
mode_check() {
    local m=$1 where=$2 arch=$3 cloud=$4 tpc=$5 placement=${6:-} p key value threading=() v idle seen=" " transport settings
    [[ $m != *,* ]] || { echo "mode $m: a comma would split its results.csv rows (COALESCE takes <rx>/<tx>)" >&2; return 2; }
    # the transport, as vm.sh driver takes it (any other stops the run there)
    transport=${m%+idle}
    transport=${transport%%@*}
    [[ $transport =~ ^(kernel|kernel-busy|dpdk|dpdk-spin|dpdk-adaptive|dpdk-epoll|kernel-raw|dpdk-raw)(-mono)?$ ]] ||
        { echo "mode $m: transport $transport: kernel, kernel-busy, dpdk, dpdk-spin, dpdk-adaptive, dpdk-epoll, or the raw floor's kernel-raw or dpdk-raw, optionally -mono (vm.sh refuses any other at the driver's start)" >&2; return 2; }
    # the raw floor: no Aeron, so nothing only Aeron reads, and its round trip
    # alone, cross-host (floor.rs)
    if mode_raw "$m"; then
        [[ $m != *+idle ]] || { echo "mode $m: the raw floor runs no Aeron, so no idle subscription (+idle)" >&2; return 2; }
        [[ $where != local ]] || { echo "mode $m: the raw floor is cross-host only: its round trip between the hosts' data NICs" >&2; return 2; }
        while IFS= read -r p; do
            key=${p%%=*}
            case $key in
            AERON_* | PUB_PARAMS | SUB_PARAMS | RELIABLE | CONDUCTOR)
                echo "mode $m: $key is Aeron's, and the raw floor runs no Aeron (no media driver, no client): it would measure its baseline under another name" >&2
                return 2
                ;;
            esac
        done < <(mode_pairs "$m")
    fi
    settings=${LAB_SETTINGS# }
    settings=${settings% }
    while IFS= read -r p; do
        key=${p%%=*} value=${p#*=}
        # vm.sh driver's own test of an override, then the key: a setting
        # the lab knows, or a variable the driver reads
        [[ $p =~ ^[A-Z][A-Z0-9_]*=[^\'[:space:]]*$ ]] ||
            { echo "mode $m: $p is not <KEY>=<value> (an upper-case key; no quote or space): vm.sh would refuse it at the driver's start" >&2; return 2; }
        case $key in
        AERON_* | RUSTERON_*) ;;
        *)
            [[ $LAB_SETTINGS == *" $key "* ]] || {
                echo "mode $m: $key is neither a setting the lab knows ($settings) nor an AERON_* or RUSTERON_* variable: vm.sh would export it to a driver that never reads it, and the mode would measure its baseline under another name" >&2
                return 2
            }
            ;;
        esac
        case $key in
        IRQ | NAPI_DEFER | NAPI_BUSY | CONDUCTOR | HAIRPIN | RTT_GAP_US)
            [[ $seen != *" $key "* ]] || { echo "mode $m: one $key at most" >&2; return 2; }
            seen+="$key "
            ;;
        esac
        case $key in
        IRQ | NAPI_DEFER | NAPI_BUSY)
            [[ ${m%%@*} != dpdk* ]] ||
                { echo "mode $m: $key is for kernel modes only (DPDK takes the data NICs' queues from the kernel: no interrupts)" >&2; return 2; }
            ;;
        esac
        case $key in
        AERON_THREADING_MODE) threading+=("$value") ;;
        IRQ)
            [[ $value =~ ^(hk|receiver|spare)$ ]] || { echo "mode $m: IRQ=hk, receiver or spare (the CPU of the data NICs' queue IRQs)" >&2; return 2; }
            [[ $cloud == aws ]] || { echo "mode $m: IRQ is AWS only (ENA's queue IRQs, named after their interfaces)" >&2; return 2; }
            [[ $value != spare || $where != local ]] ||
                { echo "mode $m: IRQ=spare with both ends on one host (localhost): the run takes the fifth core for its second end" >&2; return 2; }
            ;;
        NAPI_DEFER)
            [[ $value =~ ^([0-9]{1,10})/[0-9]{1,18}$ ]] && ((10#${BASH_REMATCH[1]} <= 2147483647)) ||
                { echo "mode $m: NAPI_DEFER=<n>/<ns>: napi_defer_hard_irqs (0 to 2147483647) and gro_flush_timeout in ns" >&2; return 2; }
            ;;
        NAPI_BUSY)
            [[ $value == spare ]] || { echo "mode $m: NAPI_BUSY=spare (the busy-polling kthreads on the fifth core) or nothing" >&2; return 2; }
            [[ $where != local ]] ||
                { echo "mode $m: NAPI_BUSY=spare with both ends on one host (localhost): the run takes the fifth core for its second end" >&2; return 2; }
            ;;
        CONDUCTOR)
            [[ $value == own ]] || { echo "mode $m: CONDUCTOR=own (the conductor on the layout's fifth core, spinning) or nothing" >&2; return 2; }
            [[ $where != local ]] ||
                { echo "mode $m: CONDUCTOR=own with both ends on one host (localhost): the run takes the fifth core for its second end" >&2; return 2; }
            ;;
        HAIRPIN)
            [[ $value == 1 ]] || { echo "mode $m: HAIRPIN=1 or nothing" >&2; return 2; }
            mode_raw "$m" ||
                { echo "mode $m: HAIRPIN is the raw floor's (kernel-raw, dpdk-raw): the floor client runs both ends in one process; Aeron's would need two clients on one driver" >&2; return 2; }
            [[ $cloud == aws ]] || { echo "mode $m: HAIRPIN is AWS only (two ENIs on the data subnet)" >&2; return 2; }
            ;;
        RTT_GAP_US)
            [[ $value =~ ^[1-9][0-9]{0,6}$ ]] && ((value <= 1000000)) ||
                { echo "mode $m: RTT_GAP_US=$value: whole microseconds between pings, 1 to 1000000 (a rep of at most 60 s holds 60 round trips at 1 s)" >&2; return 2; }
            [[ $where != local ]] ||
                { echo "mode $m: RTT_GAP_US is cross-host only: a sparse flow is the network's (lib.sh paces only the cross-host round trip)" >&2; return 2; }
            ;;
        JUMBO)
            [[ $value == 1 ]] || { echo "mode $m: JUMBO=1 or nothing" >&2; return 2; }
            [[ $cloud == aws ]] || { echo "mode $m: JUMBO is AWS only (ENA's 9001-byte frames)" >&2; return 2; }
            ;;
        SMT)
            [[ $value == off ]] || { echo "mode $m: SMT=off or nothing" >&2; return 2; }
            [[ $arch != arm64 ]] || { echo "mode $m: SMT=off on Graviton, which has no SMT (each CPU a core)" >&2; return 2; }
            [[ $tpc != 1 ]] || { echo "mode $m: SMT=off with LAB_THREADS_PER_CORE=1: the instances have no SMT to turn off" >&2; return 2; }
            ;;
        PUB_PARAMS) channel_params pub "$value" >/dev/null || { echo "mode $m: see above" >&2; return 2; } ;;
        SUB_PARAMS) channel_params sub "$value" >/dev/null || { echo "mode $m: see above" >&2; return 2; } ;;
        esac
    done < <(mode_pairs "$m")
    ((${#threading[@]} <= 1)) || { echo "mode $m: one AERON_THREADING_MODE at most" >&2; return 2; }
    mode_mtu_check "$m" "$placement" "$cloud" || return 2
    channel_buffers_check "$m" || return 2
    v=${threading[0]:-DEDICATED}
    [[ $v =~ ^(DEDICATED|SHARED_NETWORK|SHARED|INVOKER)$ ]] ||
        { echo "mode $m: AERON_THREADING_MODE is DEDICATED, SHARED_NETWORK, SHARED or INVOKER (Aeron would take anything else as DEDICATED)" >&2; return 2; }
    # the conductor on a core of its own: one the driver has as a thread of
    # its own (not SHARED's shared agent, nor INVOKER's client thread), on a
    # core nothing else of the mode takes
    if [[ $(mode_get "$m" CONDUCTOR) == own ]]; then
        case $v in
        SHARED) echo "mode $m: CONDUCTOR=own under SHARED: the conductor is the one shared agent, on the sender's core" >&2; return 2 ;;
        INVOKER) echo "mode $m: CONDUCTOR=own under INVOKER: the driver has no conductor thread (each client runs it on its own)" >&2; return 2 ;;
        esac
        for p in IRQ NAPI_BUSY; do
            [[ $(mode_get "$m" "$p") != spare ]] || { echo "mode $m: CONDUCTOR=own with $p=spare: both take the layout's fifth core" >&2; return 2; }
        done
    fi
    if [[ ${m%%@*} == dpdk* ]]; then
        dpdk_values_check "$m" "$cloud" "$placement" || return 2
        local agents=()
        case $v in
        DEDICATED) agents=(AERON_SENDER_IDLE_STRATEGY AERON_RECEIVER_IDLE_STRATEGY) ;;
        SHARED_NETWORK) agents=(AERON_SHAREDNETWORK_IDLE_STRATEGY) ;;
        SHARED) agents=(AERON_SHARED_IDLE_STRATEGY) ;;
        esac
        for p in ${agents[@]+"${agents[@]}"}; do
            if idle=$(mode_get "$m" "$p") && ! dpdk_busy_idle "$idle"; then
                echo "mode $m: $p=$idle: the DPDK transport refuses a network agent that does not busy-spin (spin or noop)" >&2
                return 2
            fi
        done
        if [[ $v != DEDICATED ]] && idle=$(mode_get "$m" RUSTERON_DPDK_TX_PACE_SPIN_US) && [[ $idle =~ [1-9] ]]; then
            echo "mode $m: RUSTERON_DPDK_TX_PACE_SPIN_US=$idle under $v: the DPDK transport refuses a pacing wait on the agent that also receives (0 or unset)" >&2
            return 2
        fi
    fi
    [[ $v == INVOKER ]] || return 0
    [[ $where != local ]] ||
        { echo "mode $m: INVOKER with both ends on one host (localhost) is out of scope: each client would embed a driver of its own on one AERON_DIR" >&2; return 2; }
    [[ $m != *+idle ]] ||
        { echo "mode $m: INVOKER with +idle: the idle subscription's client would embed a second driver on one AERON_DIR" >&2; return 2; }
}
