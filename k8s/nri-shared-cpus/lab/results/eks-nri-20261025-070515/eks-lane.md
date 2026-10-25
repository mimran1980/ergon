| item | node a | node b | evidence |
|---|---|---|---|
| NRI socket | VERIFIED | VERIFIED | /var/run/nri/nri.sock is a socket; ergon_shared_cpus_nri_connected 1 |
| nodeadm TOML merge | VERIFIED | VERIFIED | containerd config dump shows the NodeConfig's NRI table (default_validator enable = true) |
| kubelet drop-in merge (--kubelet-config-dir) | VERIFIED | VERIFIED | kubelet runs with --config-dir; ergon_shared_cpus_contract_ok 1 (the merged reservedSystemCPUs and static policy pass) |
| /opt writable | VERIFIED | VERIFIED | the plugin installed /opt/ergon/bin/ergon-cpus on the node |
| system-node-critical outside kube-system | VERIFIED | VERIFIED | the plugin pod in ergon-system with priorityClassName system-node-critical, Running |
| device plugin registered | VERIFIED | VERIFIED | ergon_shared_cpus_device_plugin_registered 1 |
| quiet-CPU boot unit | VERIFIED | VERIFIED | ergon-quiet-cpus.service enabled and active; quiet-cpus.sh --check rc 0 |

User data: userdata: 3505 bytes gzipped, 12879 bytes under EC2's 16384-byte limit
