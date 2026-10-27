#!/usr/bin/env bash
# What ENA Express (SRD) does with UDP above its 8,900-byte limit, on real ENA.
# Two spot instances of an ENA Express type (c6gn.4xlarge: the cheapest
# supported Graviton size; EXP_TYPE to change) in one subnet of the default
# VPC, Amazon Linux 2023 arm64, the primary ENI's ENA Express and its UDP mode
# on (EXP_SRD=0: both off, the control). Host a sends 2,000 UDP datagrams of
# each IP size in EXP_SIZES to host b, Don't Fragment set and then clear, and
# b counts what arrives; both hosts' ena_srd_* ethtool counters are read
# around each size, so a size that arrives without moving the SRD counters
# fell back to plain ENA.
#
#   md-dpdk/benches/cloud/aws/ena-express-mtu.sh
#   EXP_ESTIMATE=1 ...                  the plan and its price, nothing created
#
# Everything is tagged owner=ergon-lab run=ena-express-<stamp>; an EXIT/INT/
# TERM trap terminates the instances and deletes the security group and key
# pair, then lists by tag to verify. Each instance also terminates itself after
# EXP_MAX_MIN minutes (shutdown -h, InstanceInitiatedShutdownBehavior=
# terminate), should this machine never delete it.
set -uo pipefail
export AWS_PAGER=""
region=${EXP_REGION:-us-east-1}
type=${EXP_TYPE:-c6gn.4xlarge}
az=${EXP_AZ:-}
srd=${EXP_SRD:-1}
max_min=${EXP_MAX_MIN:-50}
sizes=${EXP_SIZES:-1500 8800 8900 8901 8950 9001 9100}
count=${EXP_COUNT:-2000}
stamp=$(date -u +%Y%m%d-%H%M%S)
run=ena-express-$stamp
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
out=${EXP_OUT:-$repo/target/md-dpdk-bench/$run}
if [[ -n ${EXP_ESTIMATE:-} ]]; then
    echo "plan: 2 x $type spot in $region${az:+ ($az)}, ENA Express UDP ${srd}, sizes $sizes, at most $max_min min each"
    echo "price: about 2 x 0.25 \$/h spot for well under an hour (us-east-1 c6gn.4xlarge spot 2026-10-25: 0.235-0.253 \$/h); nothing created"
    exit 0
fi
mkdir -p "$out"
log() { echo "[$(date -u +%H:%M:%S)] $*" | tee -a "$out/log.txt" >&2; }
tags="ResourceType=instance,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$run},{Key=Name,Value=ergon-lab-$run}]"
vtags="ResourceType=volume,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$run}]"
ntags="ResourceType=network-interface,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$run}]"
stags="ResourceType=spot-instances-request,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$run}]"
key=ergon-lab-$run sg= ids=()

teardown() {
    trap '' INT TERM
    log "teardown: run $run"
    local i
    if ((${#ids[@]})); then
        aws ec2 terminate-instances --region "$region" --instance-ids "${ids[@]}" >/dev/null 2>&1
        aws ec2 wait instance-terminated --region "$region" --instance-ids "${ids[@]}" 2>/dev/null ||
            log "teardown: wait for termination failed"
    fi
    for i in $(aws ec2 describe-spot-instance-requests --region "$region" --filters "Name=tag:run,Values=$run" \
        "Name=state,Values=open,active" --query 'SpotInstanceRequests[].SpotInstanceRequestId' --output text 2>/dev/null); do
        aws ec2 cancel-spot-instance-requests --region "$region" --spot-instance-request-ids "$i" >/dev/null 2>&1
    done
    if [[ -n $sg ]]; then
        for i in 1 2 3 4 5 6; do
            aws ec2 delete-security-group --region "$region" --group-id "$sg" 2>/dev/null && break
            sleep 10
        done
    fi
    aws ec2 delete-key-pair --region "$region" --key-name "$key" >/dev/null 2>&1
    # verification, by tag and name; a failed listing is UNVERIFIED, never 0
    local inst vols enis sgs keys ok=1
    inst=$(aws ec2 describe-instances --region "$region" --filters "Name=tag:run,Values=$run" \
        "Name=instance-state-name,Values=pending,running,stopping,stopped,shutting-down" \
        --query 'Reservations[].Instances[].InstanceId' --output text 2>&1) || ok=0
    vols=$(aws ec2 describe-volumes --region "$region" --filters "Name=tag:run,Values=$run" --query 'Volumes[].VolumeId' --output text 2>&1) || ok=0
    enis=$(aws ec2 describe-network-interfaces --region "$region" --filters "Name=tag:run,Values=$run" --query 'NetworkInterfaces[].NetworkInterfaceId' --output text 2>&1) || ok=0
    sgs=$(aws ec2 describe-security-groups --region "$region" --filters "Name=group-name,Values=ergon-lab-$run" --query 'SecurityGroups[].GroupId' --output text 2>&1) || ok=0
    keys=$(aws ec2 describe-key-pairs --region "$region" --filters "Name=key-name,Values=$key" --query 'KeyPairs[].KeyName' --output text 2>&1) || ok=0
    if ((ok)); then
        log "verify $region run=$run: instances [${inst}] volumes [${vols}] enis [${enis}] security groups [${sgs}] key pairs [${keys}]"
        [[ -z $inst$vols$enis$sgs$keys ]] && log "verify: nothing of $run remains" || log "verify: LEFTOVERS above"
    else
        log "verify: UNVERIFIED, a listing failed: $inst $vols $enis $sgs $keys"
    fi
}
trap teardown EXIT
trap 'exit 130' INT TERM

aws sts get-caller-identity >/dev/null || { log "aws sign-in failed: nothing created"; trap - EXIT; exit 1; }
priv=~/.ssh/id_ed25519; [[ -f $priv ]] || priv=~/.ssh/id_rsa
myip=$(curl -fsS https://checkip.amazonaws.com) || { log "no public IP"; exit 1; }
vpc=$(aws ec2 describe-vpcs --region "$region" --filters Name=is-default,Values=true --query 'Vpcs[0].VpcId' --output text)
[[ $vpc == vpc-* ]] || { log "no default VPC in $region"; exit 1; }
if [[ -z $az ]]; then
    az=$(aws ec2 describe-spot-price-history --region "$region" --instance-types "$type" --product-descriptions Linux/UNIX \
        --start-time "$(date -u +%Y-%m-%dT%H:%M:%S)" --query 'sort_by(SpotPriceHistory,&SpotPrice)[0].AvailabilityZone' --output text)
fi
subnet=$(aws ec2 describe-subnets --region "$region" --filters "Name=vpc-id,Values=$vpc" "Name=availability-zone,Values=$az" \
    "Name=default-for-az,Values=true" --query 'Subnets[0].SubnetId' --output text)
[[ $subnet == subnet-* ]] || { log "no default subnet in $az"; exit 1; }
ami=$(aws ssm get-parameter --region "$region" --name /aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64 \
    --query Parameter.Value --output text) || exit 1
log "run $run: 2 x $type spot in $az ($subnet), AMI $ami, ENA Express UDP $srd, SSH from $myip"
pubfile=~/.ssh/id_ed25519.pub; [[ -f $pubfile ]] || pubfile=~/.ssh/id_rsa.pub
aws ec2 import-key-pair --region "$region" --key-name "$key" --public-key-material "fileb://$pubfile" \
    --tag-specifications "ResourceType=key-pair,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$run}]" >/dev/null || exit 1
sg=$(aws ec2 create-security-group --region "$region" --vpc-id "$vpc" --group-name "ergon-lab-$run" \
    --description "ergon-lab $run" --tag-specifications "ResourceType=security-group,Tags=[{Key=owner,Value=ergon-lab},{Key=run,Value=$run}]" \
    --query GroupId --output text) || exit 1
aws ec2 authorize-security-group-ingress --region "$region" --group-id "$sg" --protocol tcp --port 22 --cidr "$myip/32" >/dev/null || exit 1
aws ec2 authorize-security-group-ingress --region "$region" --group-id "$sg" --ip-permissions \
    "IpProtocol=-1,UserIdGroupPairs=[{GroupId=$sg}]" >/dev/null || exit 1
userdata=$(printf '#!/bin/bash\nshutdown -h +%d\n' "$max_min" | base64)
srdspec="EnaSrdEnabled=false"
((srd)) && srdspec="EnaSrdEnabled=true,EnaSrdUdpSpecification={EnaSrdUdpEnabled=true}"
for h in a b; do
    id=$(aws ec2 run-instances --region "$region" --image-id "$ami" --instance-type "$type" --key-name "$key" \
        --network-interfaces "DeviceIndex=0,SubnetId=$subnet,Groups=$sg,AssociatePublicIpAddress=true,DeleteOnTermination=true,EnaSrdSpecification={$srdspec}" \
        --instance-market-options 'MarketType=spot,SpotOptions={SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}' \
        --instance-initiated-shutdown-behavior terminate --user-data "$userdata" \
        --tag-specifications "$tags" "$vtags" "$ntags" "$stags" --query 'Instances[0].InstanceId' --output text) ||
        { log "run-instances $h failed"; exit 1; }
    ids+=("$id")
    log "host $h: $id"
done
aws ec2 wait instance-running --region "$region" --instance-ids "${ids[@]}" || exit 1
read -r ipa pa <<<"$(aws ec2 describe-instances --region "$region" --instance-ids "${ids[0]}" --query 'Reservations[0].Instances[0].[PublicIpAddress,PrivateIpAddress]' --output text)"
read -r ipb pb <<<"$(aws ec2 describe-instances --region "$region" --instance-ids "${ids[1]}" --query 'Reservations[0].Instances[0].[PublicIpAddress,PrivateIpAddress]' --output text)"
aws ec2 describe-instances --region "$region" --instance-ids "${ids[@]}" \
    --query 'Reservations[].Instances[].[InstanceId,InstanceType,Placement.AvailabilityZone,NetworkInterfaces[0].Attachment.EnaSrdSpecification]' \
    --output json >"$out/instances.json"
ssh_opts=(-i "$priv" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10 -o ServerAliveInterval=15 -o LogLevel=ERROR)
on() { local h=$1; shift; ssh "${ssh_opts[@]}" "ec2-user@$h" "$@"; }
for h in "$ipa" "$ipb"; do
    for i in $(seq 1 40); do on "$h" true 2>/dev/null && break; sleep 5; done
done
for h in "$ipa" "$ipb"; do
    on "$h" 'uname -r; ethtool -i $(ip -o route get 1.1.1.1 | sed "s/.* dev \([^ ]*\).*/\1/") | head -3; ip -o link show | grep -o "mtu [0-9]*" | head -2' 2>&1 | sed "s/^/$h /" | tee -a "$out/log.txt"
done
# the receiver: counts datagrams by length on UDP 40000 until told to stop
recv='import socket,sys,collections
s=socket.socket(socket.AF_INET,socket.SOCK_DGRAM); s.setsockopt(socket.SOL_SOCKET,socket.SO_RCVBUF,1<<25); s.bind(("0.0.0.0",40000)); s.settimeout(1.0)
c=collections.Counter(); idle=0
while idle<int(sys.argv[1]):
    try: d=s.recv(65535); c[len(d)]+=1; idle=0
    except socket.timeout: idle+=1
print(" ".join(f"{k}:{v}" for k,v in sorted(c.items())))'
send='import socket,sys,time
dst,ip_size,n,df=sys.argv[1],int(sys.argv[2]),int(sys.argv[3]),sys.argv[4]=="1"
s=socket.socket(socket.AF_INET,socket.SOCK_DGRAM)
s.setsockopt(socket.IPPROTO_IP,10,2 if df else 0)  # IP_MTU_DISCOVER: DO or DONT
p=bytes(ip_size-28); ok=err=0; last=""
for i in range(n):
    try: s.sendto(p,(dst,40000)); ok+=1
    except OSError as e: err+=1; last=str(e)
    time.sleep(0.0005)
print(f"sent {ok} errors {err} {last}")'
srdstats='ethtool -S $(ip -o route get 1.1.1.1 | sed "s/.* dev \([^ ]*\).*/\1/") | grep -E "ena_srd|bw_|pps_allowance|conntrack" | tr -s " " | tr "\n" ";"'
echo "ip_size,df,sent,send_errors,received_by_length,a_srd_before,a_srd_after,b_srd_before,b_srd_after" >"$out/results.csv"
for df in 1 0; do
    for size in $sizes; do
        abefore=$(on "$ipa" "$srdstats") bbefore=$(on "$ipb" "$srdstats")
        on "$ipb" "python3 -c '$recv' 4" >"$out/recv.txt" 2>&1 &
        rp=$!
        sleep 2
        sent=$(on "$ipa" "python3 -c '$send' $pb $size $count $df" 2>&1)
        wait $rp
        aafter=$(on "$ipa" "$srdstats") bafter=$(on "$ipb" "$srdstats")
        got=$(tr -d '\n' <"$out/recv.txt")
        log "ip size $size df $df: $sent; received {$got}"
        echo "$size,$df,\"$sent\",,\"$got\",\"$abefore\",\"$aafter\",\"$bbefore\",\"$bafter\"" >>"$out/results.csv"
    done
done
on "$ipa" 'ip -s link show; tracepath -n '"$pb"' 2>&1 | tail -3' >"$out/a-link.txt" 2>&1
log "done: $out"
