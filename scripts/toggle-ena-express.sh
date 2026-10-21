#!/usr/bin/env bash
# Toggle ENA Express (Scalable Reliable Datagrams) on a single ENI.
#
# ENA Express must be supported by the instance type and enabled for the
# primary ENI; see md-dpdk/docs/aws-eks-dpdk-ena-runbook.md §12.3. The toggle is
# immediate on an existing ENI and does not require an instance restart.
#
# Usage:
#   toggle-ena-express.sh <eni-id> on|off
#
# Prints the resulting EnaSrdSpecification so callers can verify the state.

set -euo pipefail

ENI="${1:-}"
STATE="${2:-}"
case "$ENI:$STATE" in
    eni-*:on|eni-*:off) ;;
    *) echo "usage: toggle-ena-express.sh <eni-id> on|off" >&2; exit 2 ;;
esac

if [[ "$STATE" == "on" ]]; then
    aws ec2 modify-network-interface-attribute \
        --network-interface-id "$ENI" \
        --ena-srd-enabled \
        --ena-srd-udp-specification Enable=true
else
    aws ec2 modify-network-interface-attribute \
        --network-interface-id "$ENI" \
        --no-ena-srd-enabled \
        --ena-srd-udp-specification Enable=false
fi

# Confirm the new state. EnaSrdSpecification is present only on ENAs that
# support SRD; absence after an "on" means this instance/ENI does not support
# ENA Express.
echo "eni=$ENI ena-srd=$STATE result:"
aws ec2 describe-network-interface-attribute \
    --network-interface-id "$ENI" \
    --attribute enaSrdSpecification \
    --query 'EnaSrdSpecification' \
    --output json
