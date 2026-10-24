#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The EKS AL2023 user data for one node of this sample (c9g.2xlarge: 8 CPUs,
# no SMT), gzip-compressed for `--user-data fileb://`:
#
#   samples/low-latency-k8s/node/eks-userdata.sh <cluster> <apiServerEndpoint> <ca-b64> <service-cidr> <ping|pong> > userdata.gz
#
# It runs md-dpdk/deploy/aws-eks/node/userdata.sh with reserved CPUs "0-1"
# and IRQ CPU "0", the layout helm/values-eks.yaml expects: the kernel's
# housekeeping CPUs (RUSTERON_HOUSEKEEPING_CPUS, the boot line's isolcpus
# complement) are 0-1, so the shared CPU 1 is never isolated and takes no
# IRQs. Its own example, "0", would isolate CPU 1 and the plugin would refuse
# every hot pod. It then swaps that file's NodeConfig part for
# node/eks-nodeconfig.yaml (NRI on, the ergon.io labels, ll-role) with the
# cluster fields and the role filled in, and checks EC2's 16384-byte limit.
#
# Checked offline by tests/manifests.rs. Not run on EC2.
set -euo pipefail

fail() { echo "eks-userdata FAIL: $*" >&2; exit 1; }
(($# == 5)) || { echo "usage: $0 <cluster> <apiServerEndpoint> <ca-b64> <service-cidr> <ping|pong> > userdata.gz" >&2; exit 2; }
cluster=$1 endpoint=$2 ca=$3 cidr=$4 role=$5
[[ $role == ping || $role == pong ]] || fail "role '$role': ping or pong"
HERE=$(cd "$(dirname "$0")/.." && pwd)
UPSTREAM=$HERE/../../md-dpdk/deploy/aws-eks/node/userdata.sh
[[ -f $UPSTREAM ]] || fail "$UPSTREAM missing"
# userdata.sh validates the cluster fields (names, https URL, base64, CIDR)
upstream=$(bash "$UPSTREAM" "$cluster" "$endpoint" "$ca" "$cidr" 0-1 0 | gzip -dc) || fail "userdata.sh failed"
grep -q '^RUSTERON_HOUSEKEEPING_CPUS=0-1$' <<<"$upstream" || fail "userdata.sh did not take reserved CPUs 0-1"
grep -q '^RUSTERON_IRQ_CPUS=0$' <<<"$upstream" || fail "userdata.sh did not take IRQ CPU 0"
grep -q '^Content-Type: application/node.eks.aws$' <<<"$upstream" || fail "no NodeConfig part in userdata.sh's output"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
sed -e "s|CLUSTER_NAME|$cluster|" -e "s|API_SERVER_ENDPOINT|$endpoint|" \
  -e "s|CA_BASE64|$ca|" -e "s|SERVICE_CIDR|$cidr|" -e "s|LL_ROLE|$role|" \
  "$HERE/node/eks-nodeconfig.yaml" >"$work/nodeconfig.yaml"
out=$work/userdata.gz
# keep everything up to the NodeConfig part's header, then ours, then the
# closing boundary
awk -v nc="$work/nodeconfig.yaml" '
  skip && /^--.*--$/ { print ""; print; skip = 0; next }
  skip { next }
  /^Content-Type: application\/node.eks.aws$/ { print; print ""; while ((getline line < nc) > 0) print line; skip = 1; next }
  { print }
' <<<"$upstream" | gzip -9n >"$out"
size=$(($(wc -c <"$out")))
((size <= 16384)) || fail "user data is $size bytes gzipped, over EC2's 16384-byte limit"
cat "$out"
