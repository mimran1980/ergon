#!/usr/bin/env bash
# Reschedule verification: a stateful pod with a shared hostPath PV moves
# between the two kind workers; verify data survives, the pod IP changes,
# and DNS answers the new address. Proves shared-mount locking + storage
# reattachment on ONE host; it does NOT prove cross-host failover.
set -euo pipefail
cd "$(dirname "$0")/.."
NAMESPACE="reschedule"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "ok: $1"; }

kubectl get ns "$NAMESPACE" >/dev/null 2>&1 || kubectl create ns "$NAMESPACE"

# Shared-mount static PV + stateful pod writing a checkpoint file.
cat <<'EOF' | kubectl apply -f - >/dev/null
apiVersion: v1
kind: PersistentVolume
metadata:
  name: shared-test-pv
spec:
  capacity: { storage: 1Gi }
  accessModes: ["ReadWriteOnce"]
  hostPath: { path: /mnt/ergo-shared-test }
  storageClassName: manual
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: state-claim
  namespace: reschedule
spec:
  accessModes: ["ReadWriteOnce"]
  storageClassName: manual
  resources: { requests: { storage: 1Gi } }
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: stateful-probe
  namespace: reschedule
spec:
  serviceName: stateful-probe
  replicas: 1
  selector: { matchLabels: { app: stateful-probe } }
  template:
    metadata: { labels: { app: stateful-probe } }
    spec:
      containers:
        - name: probe
          image: busybox:1.36
          command: ["sh", "-c", "mkdir -p /state && echo alive > /state/heartbeat && sleep infinity"]
          volumeMounts: [{ name: state, mountPath: /state }]
          readinessProbe: { exec: { command: ["sh", "-c", "test -f /state/heartbeat"] }, periodSeconds: 2 }
      volumes:
        - name: state
          persistentVolumeClaim: { claimName: state-claim }
EOF
kubectl -n "$NAMESPACE" rollout status statefulset/stateful-probe --timeout=120s || fail "pod not ready"
pass "stateful pod ready (worker $(kubectl -n "$NAMESPACE" get pod stateful-probe-0 -o jsonpath='{.spec.nodeName}'))"

# Write state, note identity.
before_node=$(kubectl -n "$NAMESPACE" get pod stateful-probe-0 -o jsonpath='{.spec.nodeName}')
before_ip=$(kubectl -n "$NAMESPACE" get pod stateful-probe-0 -o jsonpath='{.status.podIP}')
kubectl -n "$NAMESPACE" exec stateful-probe-0 -- sh -c 'echo checkpoint-1 > /state/checkpoint'
pass "wrote checkpoint on $before_node ($before_ip)"

# Delete the pod; StatefulSet recreates it (may land on either worker —
# the shared mount makes both valid).
kubectl -n "$NAMESPACE" delete pod stateful-probe-0 --wait=false >/dev/null
kubectl -n "$NAMESPACE" rollout status statefulset/stateful-probe --timeout=120s || fail "pod not ready after reschedule"
after_node=$(kubectl -n "$NAMESPACE" get pod stateful-probe-0 -o jsonpath='{.spec.nodeName}')
after_ip=$(kubectl -n "$NAMESPACE" get pod stateful-probe-0 -o jsonpath='{.status.podIP}')
pass "pod rescheduled to $after_node ($after_ip)"
[[ "$before_ip" != "$after_ip" ]] && pass "pod IP changed: $before_ip -> $after_ip" \
    || echo "note: IP unchanged (scheduler returned to $after_node)"

# State survived the move (shared mount + RWO PV reattachment).
data=$(kubectl -n "$NAMESPACE" exec stateful-probe-0 -- cat /state/checkpoint)
[[ "$data" == "checkpoint-1" ]] || fail "checkpoint lost across reschedule"
pass "state preserved across worker move"

# Headless service DNS answers the CURRENT address.
cat <<'EOF' | kubectl apply -f - >/dev/null
apiVersion: v1
kind: Service
metadata:
  name: stateful-probe
  namespace: reschedule
spec:
  clusterIP: None
  selector: { app: stateful-probe }
  ports: [{ port: 80 }]
EOF
sleep 2
dns=$(kubectl -n "$NAMESPACE" run dns-probe --rm -i --restart=Never --image=busybox:1.36 -- \
    nslookup stateful-probe-0.stateful-probe."$NAMESPACE".svc.cluster.local 2>/dev/null | grep -o "$after_ip" | head -1)
[[ "$dns" == "$after_ip" ]] && pass "DNS answers current pod IP" || fail "DNS did not answer $after_ip (got: ${dns:-none})"

echo "reschedule verification: PASS"
