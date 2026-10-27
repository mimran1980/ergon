| item | node a | node b | evidence |
|---|---|---|---|
| NRI socket | VERIFIED | VERIFIED | /var/run/nri/nri.sock is a socket; ergon_shared_cpus_nri_connected 1 |
| nodeadm TOML merge | VERIFIED | VERIFIED | containerd config dump shows the NodeConfig's NRI table (default_validator enable = true) |
| kubelet drop-in merge (--kubelet-config-dir) | VERIFIED | VERIFIED | kubelet runs with --config-dir; ergon_shared_cpus_contract_ok 1 (the merged reservedSystemCPUs and static policy pass) |
| /opt writable | VERIFIED | VERIFIED | the plugin installed /opt/ergon/bin/ergon-cpus on the node |
| system-node-critical outside kube-system | VERIFIED | VERIFIED | the plugin pod in ergon-system with priorityClassName system-node-critical, Running |
| device plugin registered | VERIFIED | VERIFIED | ergon_shared_cpus_device_plugin_registered 1 |
| quiet-CPU boot unit | VERIFIED | VERIFIED | ergon-quiet-cpus.service enabled and active; quiet-cpus.sh --check rc 0 |

User data: userdata: 3497 bytes gzipped, 12887 bytes under EC2's 16384-byte limit

Correction (review): the script that wrote this table matched each metric
as a substring, which also hits its `# HELP <name> 1 while ...` line, so the
metric verdicts held whatever the value. The values in `plugin-metrics-*.txt`:
`nri_connected` 1 and `device_plugin_registered` 1 on both nodes;
`contract_ok` **0 on node a**, 1 on node b. Node a's only refusals were the
late ENI's IRQs (`ens35` irqs 99-103 effective on CPU 0,
`plugin-logs-start.txt`). The plugin checks NIC affinity only after the
merged kubelet config loads and passes, so the kubelet drop-in merge stands
on both nodes. The check is fixed in `eks-nri.sh`.
