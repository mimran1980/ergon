#!/usr/bin/env bash
# Which Tokyo availability zone is closest to Binance: one small instance per
# zone measures the TCP handshake round trip (SYN → SYN-ACK, no TLS) to every
# address of Binance's REST and WebSocket endpoints, p50 / p99 / max per zone
# and address, plus the TLS handshake for context.
#
# Every resource carries owner=ergon-lab and run=<stamp>; the EXIT trap
# terminates and deletes them and verifies nothing tagged remains.
# Results go to results/azscan-<stamp>/.
set -euo pipefail
cd "$(dirname "$0")/.."

STAMP=${STAMP:-$(date -u +%Y%m%dT%H%M%SZ)}
REGION=${REGION:-ap-northeast-1}
TYPE=${TYPE:-c7i.large}
SAMPLES=${SAMPLES:-400}
NAME=ergon-lab-azscan-$STAMP
OUT=results/azscan-$STAMP
mkdir -p "$OUT/ssh"
LOG=$OUT/run.log
exec > >(trap '' INT TERM; exec tee -a "$LOG") 2>&1
( sleep 3600; echo "[watchdog] 3600s reached, stopping"; kill -TERM $$ ) &
WATCHDOG=$!
log() { echo "[$(date -u +%H:%M:%S)] $*"; }
ec2() { aws --region "$REGION" ec2 "$@"; }
tags() { echo "ResourceType=$1,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$STAMP},{Key=Name,Value=$NAME${2:+-$2}}]"; }
live() {
  ec2 describe-instances --filters "Name=tag:run,Values=$STAMP" \
    Name=instance-state-name,Values=pending,running,stopping,stopped,shutting-down \
    --query 'Reservations[].Instances[].InstanceId' --output text
}

teardown() {
  local rc=$?
  set +e
  trap '' PIPE INT TERM
  trap - EXIT
  kill "$WATCHDOG" 2>/dev/null
  log "teardown: terminating everything tagged run=$STAMP (exit code $rc)"
  local ids
  ids=$(live)
  # shellcheck disable=SC2086
  [[ -n $ids ]] && ec2 terminate-instances --instance-ids $ids > /dev/null && ec2 wait instance-terminated --instance-ids $ids
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    [[ -n ${SG:-} ]] && ec2 delete-security-group --group-id "$SG" 2>/dev/null && break
    [[ -z ${SG:-} ]] && break
    sleep 15
  done
  ec2 delete-key-pair --key-name "$NAME" > /dev/null 2>&1
  local i v e s k
  i=$(live | wc -w | tr -d ' ')
  v=$(ec2 describe-volumes --filters "Name=tag:run,Values=$STAMP" --query 'length(Volumes)' --output text)
  e=$(ec2 describe-network-interfaces --filters "Name=tag:run,Values=$STAMP" --query 'length(NetworkInterfaces)' --output text)
  s=$(ec2 describe-security-groups --filters "Name=tag:run,Values=$STAMP" --query 'length(SecurityGroups)' --output text)
  k=$(ec2 describe-key-pairs --filters "Name=tag:run,Values=$STAMP" --query 'length(KeyPairs)' --output text)
  log "verify ($REGION, tag run=$STAMP): instances $i, volumes $v, interfaces $e, security groups $s, key pairs $k"
  [[ "$i$v$e$s$k" == 00000 ]] || { log "!!! RESOURCES LEFT FOR run=$STAMP: delete them by hand"; rc=1; }
  exit "$rc"
}
trap teardown EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

KEY=$OUT/ssh/id_ed25519
ssh-keygen -q -t ed25519 -N '' -f "$KEY" -C "$NAME"
SSHO=(-i "$KEY" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ConnectTimeout=15 -o ConnectionAttempts=5)
MYIP=$(curl -s https://api.ipify.org)
log "creating one $TYPE per zone in $REGION"
ec2 import-key-pair --key-name "$NAME" --public-key-material "fileb://$KEY.pub" --tag-specifications "$(tags key-pair)" > /dev/null
VPC=$(ec2 describe-vpcs --filters Name=isDefault,Values=true --query 'Vpcs[0].VpcId' --output text)
SG=$(ec2 create-security-group --group-name "$NAME" --description "ergon-lab az scan $STAMP" --vpc-id "$VPC" \
  --tag-specifications "$(tags security-group)" --query GroupId --output text)
ec2 authorize-security-group-ingress --group-id "$SG" --protocol tcp --port 22 --cidr "$MYIP/32" > /dev/null
# Ubuntu for TYPE's architecture (arm64 for Graviton types).
ARCH=$(ec2 describe-instance-types --instance-types "$TYPE" --query 'InstanceTypes[0].ProcessorInfo.SupportedArchitectures[0]' --output text)
[[ $ARCH == arm64 ]] || ARCH=amd64
AMI=$(aws --region "$REGION" ssm get-parameter --name "/aws/service/canonical/ubuntu/server/24.04/stable/current/$ARCH/hvm/ebs-gp3/ami-id" --query Parameter.Value --output text)
declare -A ID IP
for az in $(ec2 describe-availability-zones --query 'AvailabilityZones[?State==`available`].ZoneId' --output text); do
  subnet=$(ec2 describe-subnets --filters "Name=vpc-id,Values=$VPC" "Name=availability-zone-id,Values=$az" Name=default-for-az,Values=true \
    --query 'Subnets[0].SubnetId' --output text)
  ID[$az]=$(ec2 run-instances --image-id "$AMI" --instance-type "$TYPE" --key-name "$NAME" --subnet-id "$subnet" \
    --security-group-ids "$SG" --block-device-mappings 'DeviceName=/dev/sda1,Ebs={VolumeSize=10,VolumeType=gp3,DeleteOnTermination=true}' \
    --tag-specifications "$(tags instance "$az")" "$(tags volume "$az")" "$(tags network-interface "$az")" \
    --query 'Instances[0].InstanceId' --output text)
done
# shellcheck disable=SC2086
ec2 wait instance-running --instance-ids ${ID[*]}
for az in "${!ID[@]}"; do
  IP[$az]=$(ec2 describe-instances --instance-ids "${ID[$az]}" --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
done

PROBE=$(cat <<'PY'
import socket, ssl, sys, time, statistics
hosts = [("api.binance.com", 443), ("stream.binance.com", 9443), ("fstream.binance.com", 443), ("fapi.binance.com", 443)]
n = int(sys.argv[1])
for host, port in hosts:
    addrs = sorted({a[4][0] for a in socket.getaddrinfo(host, port, socket.AF_INET, socket.SOCK_STREAM)})
    for ip in addrs:
        rtt = []
        for _ in range(n):
            s = socket.socket(); s.settimeout(2)
            t = time.perf_counter_ns()
            try:
                s.connect((ip, port)); rtt.append(time.perf_counter_ns() - t)
            except OSError:
                pass
            s.close(); time.sleep(0.005)
        tls = []
        ctx = ssl.create_default_context()
        for _ in range(20):
            s = socket.create_connection((ip, port), timeout=3)
            t = time.perf_counter_ns()
            try:
                ctx.wrap_socket(s, server_hostname=host).close(); tls.append(time.perf_counter_ns() - t)
            except OSError:
                s.close()
        if not rtt:
            print(f"{host}\t{ip}\tunreachable"); continue
        rtt.sort()
        q = lambda p: rtt[min(len(rtt) - 1, int(p * len(rtt)))] / 1000
        print(f"{host}\t{ip}\t{len(rtt)}\t{q(0.5):.1f}\t{q(0.99):.1f}\t{rtt[-1]/1000:.1f}\t{statistics.median(tls)/1000 if tls else 0:.0f}")
PY
)
for az in "${!ID[@]}"; do
  for _ in $(seq 60); do ssh "${SSHO[@]}" "ubuntu@${IP[$az]}" true 2>/dev/null && break; sleep 5; done
  ssh "${SSHO[@]}" "ubuntu@${IP[$az]}" "cat > probe.py" <<< "$PROBE"
done
log "probing ($SAMPLES handshakes per address, all zones at once)"
for az in "${!ID[@]}"; do
  ( ssh "${SSHO[@]}" "ubuntu@${IP[$az]}" "python3 probe.py $SAMPLES" > "$OUT/$az.tsv" ) &
done
wait
{
  echo "| zone | host | address | handshakes | TCP RTT p50 µs | p99 µs | max µs | TLS handshake p50 µs |"
  echo "|---|---|---|---|---|---|---|---|"
  for az in $(printf '%s\n' "${!ID[@]}" | sort); do
    while IFS=$'\t' read -r host ip n p50 p99 max tls; do
      echo "| $az | $host | $ip | $n | $p50 | $p99 | $max | $tls |"
    done < "$OUT/$az.tsv"
  done
} > "$OUT/tables.md"
cat "$OUT/tables.md"
log "done; results in $OUT"
