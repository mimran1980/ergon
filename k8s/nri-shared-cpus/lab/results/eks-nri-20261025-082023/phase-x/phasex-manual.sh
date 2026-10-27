#!/usr/bin/env bash
# phase X by hand on run eks-nri-20261025-082023 (1 exclusive CPU a pod: a 4-vCPU node cannot fit cpu: 2)
export KUBECONFIG=<repo>/target/md-dpdk-bench/eks-nri-20261025-082023/kubeconfig
out=<repo>/target/md-dpdk-bench/eks-nri-20261025-082023/manual-x
mkdir -p "$out"
csv=$out/phase-x.csv
secs=${SECS:-15}
img=<account>.dkr.ecr.us-east-1.amazonaws.com/ergon-lab-eks-nri-20261025-082023/workload:f71c09638bed-91aaf9d1201a
na=ip-10-31-11-189.ec2.internal nb=ip-10-31-11-125.ec2.internal
ipa=$(kubectl get node $na -o jsonpath='{.status.addresses[?(@.type=="InternalIP")].address}')
ipb=$(kubectl get node $nb -o jsonpath='{.status.addresses[?(@.type=="InternalIP")].address}')
x() { kubectl exec -n ergon-latency-pods "ergon-lab-x$1" -- sh -c "$2"; }
load_on() { # <node> <name>
    kubectl apply -f - >/dev/null <<YAML
apiVersion: v1
kind: Pod
metadata: {name: $2, namespace: default, labels: {app: ergon-lab-load}}
spec:
  restartPolicy: Never
  nodeSelector: {kubernetes.io/hostname: "$1"}
  tolerations: [{operator: Exists}]
  terminationGracePeriodSeconds: 1
  containers:
    - name: load
      image: $img
      command: ["sh", "-c", "taskset -c 0 sh -c 'while :; do :; done'"]
      resources: {requests: {cpu: 100m, memory: 32Mi}}
YAML
    kubectl wait --for=condition=Ready pod/$2 -n default --timeout=120s >/dev/null
}
load_off() { kubectl delete pod -n default -l app=ergon-lab-load --wait=true >/dev/null 2>&1; }
busy() { # <probe pod>: CPU 0 busy % over 2 s
    kubectl exec "$1" -- sh -c "a=\$(grep '^cpu0 ' /proc/stat); sleep 2; b=\$(grep '^cpu0 ' /proc/stat); echo \$a \$b" |
        awk '{t1=0; for (i=2;i<=11;i++) t1+=$i; t2=0; for (i=13;i<=22;i++) t2+=$i; printf "%.0f\n", 100*(1-(($16+$17)-($5+$6))/(t2-t1))}'
}
probe_a=$(kubectl get pods -l app=ergon-lab-probe --field-selector spec.nodeName=$na -o jsonpath='{.items[0].metadata.name}')
probe_b=$(kubectl get pods -l app=ergon-lab-probe --field-selector spec.nodeName=$nb -o jsonpath='{.items[0].metadata.name}')
for w in a b; do
    x $w 'env | grep ^ERGON_ | sort; echo "pid1 $(grep Cpus_allowed_list /proc/1/status)"; echo "cpuset $(cat /sys/fs/cgroup/cpuset.cpus.effective)"; echo "exec $(grep Cpus_allowed_list /proc/self/status)"; grep Seccomp /proc/self/status' >"$out/placement-$w.txt" 2>&1
done
load_off
for cell in "idle 50000 slack50us-A" "idle 1 slack1ns" "idle 50000 slack50us-B" "load 50000 load-slack50us" "load 1 load-slack1ns"; do
    set -- $cell
    if [[ $1 == load ]] && [[ -z $(kubectl get pod -n default ergon-lab-load-a -o name 2>/dev/null) ]]; then load_on $na ergon-lab-load-a; fi
    echo "cell $3: cpu0 busy $(busy $probe_a)%" >>"$out/busy.txt"
    x a "taskset -c \$ERGON_SHARED_CPUS wakelat $secs 1000 $2 $3" | tee -a "$csv"
done
for cell in load idle; do
    [[ $cell == idle ]] && load_off
    x a "taskset -c \$ERGON_SHARED_CPUS pincost 10000" | sed "s/^pincost,/pincost-$cell,/" | tee -a "$csv"
done
load_off
port=23100
for cell in "busy - U0-busy-A" "sqshared - U1-sqpoll-shared" "busy - U0-busy-B" "sqshared - U1-sqpoll-shared-B" "busy load U0-busy-load" "sqshared load U1-sqpoll-shared-load"; do
    set -- $cell
    if [[ $2 == load ]] && [[ -z $(kubectl get pod -n default ergon-lab-load-b -o name 2>/dev/null) ]]; then load_on $na ergon-lab-load-a; load_on $nb ergon-lab-load-b; fi
    port=$((port + 2))
    if [[ $1 == busy ]]; then m=busy sq=-; else m=sqpoll sq=0; fi
    x b "uring-pp pong $((port + 1)) $ipa $port \$ERGON_ISOLATED_CPUS $m $sq $((secs + 20))" >"$out/$3-pong.log" 2>&1 &
    pb=$!
    sleep 2
    x a "uring-pp ping $port $ipb $((port + 1)) \$ERGON_ISOLATED_CPUS $m $sq $secs 10000 10000 $3" 2>"$out/$3-ping.err" | tee -a "$csv"
    wait $pb
done
load_off
kubectl logs -n ergon-system -l app=ergon-shared-cpus --tail=200 --prefix >"$out/plugin-logs.txt" 2>&1
echo done
