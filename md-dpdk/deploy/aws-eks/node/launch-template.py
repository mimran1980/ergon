#!/usr/bin/env python3
"""Launch-template data for an x86 latency node group on EC2 (EKS, self-managed
or a managed node group's launch template).

usage: launch-template.py <ami> <instance-type> <profile-arn> <userdata.gz> \\
          <stamp> <name> <default-vcpus> <default-cores> <default-threads-per-core> <arch>

Prints the JSON for `aws ec2 create-launch-template --launch-template-data
file://...`. Its CpuOptions turn SMT (hyper-threading) off on x86 latency
nodes, so one Kubernetes CPU is one physical core, the same as on Graviton:
ThreadsPerCore=1 (option 1 of the SMT decision, 2026-10-17). EC2 rules
("Rules for specifying CPU options", "CPU options for Amazon EC2 instances"):

  * you must specify both the number of CPU cores and threads per core, so
    CoreCount is the instance type's default core count and ThreadsPerCore is
    1; there is no extra charge for either;
  * bare metal takes no CPU options at all;
  * the threads per core of T2, C7a, M7a, R7a, Apple silicon Macs and
    Graviton cannot be changed (those types already run one thread per core,
    except T2 and the Macs: bare metal and those families never reach here
    with SMT to turn off -- Graviton groups get no CpuOptions at all);
  * the number of vCPUs is cores x threads per core and may not exceed the
    default, which CoreCount=default cores, ThreadsPerCore=1 does not.

A bare metal x86 node cannot take CPU options, so its SMT is turned off by
provision-node.sh instead (the kernel's nosmt, and sysfs before kubelet's
first start). The live test (benches/cloud/aws/eks.sh) fails a node whose
/sys/devices/system/cpu/smt/active is 1.

Used by benches/cloud/aws/eks.sh; asserted by node/test/test-bootstrap.sh.
"""
import base64
import json
import sys


def main(argv):
    if len(argv) != 11:
        print("usage: launch-template.py <ami> <instance-type> <profile-arn> <userdata.gz> "
              "<stamp> <name> <default-vcpus> <default-cores> <default-threads-per-core> <arch>",
              file=sys.stderr)
        return 2
    ami, itype, profile, userdata, stamp, name, vcpus, cores, tpc, arch = argv[1:]
    try:
        vcpus, cores, tpc = int(vcpus), int(cores), int(tpc)
    except ValueError:
        print(f"launch-template: vcpus/cores/threads-per-core not numbers: {argv[7:]}", file=sys.stderr)
        return 2
    if min(vcpus, cores, tpc) < 1 or cores * tpc > vcpus:
        print(f"launch-template: {itype}: {cores} core(s) x {tpc} thread(s) over {vcpus} vCPUs", file=sys.stderr)
        return 2

    def tags(rtype, what, extra=()):
        base = [("owner", "ergon-lab"), ("run", stamp), ("Name", f"{name}-{what}")]
        return {"ResourceType": rtype, "Tags": [{"Key": k, "Value": v} for k, v in base + list(extra)]}

    data = {
        "ImageId": ami,
        "InstanceType": itype,
        "IamInstanceProfile": {"Arn": profile},
        "BlockDeviceMappings": [{"DeviceName": "/dev/xvda",
                                 "Ebs": {"VolumeSize": 40, "VolumeType": "gp3", "DeleteOnTermination": True}}],
        "MetadataOptions": {"HttpTokens": "required", "HttpEndpoint": "enabled", "HttpPutResponseHopLimit": 2},
        # a node shutting itself down (the dead-man's switch) is deleted
        "InstanceInitiatedShutdownBehavior": "terminate",
        "UserData": base64.b64encode(open(userdata, "rb").read()).decode(),
        "TagSpecifications": [tags("instance", "node", [(f"kubernetes.io/cluster/{name}", "owned")]),
                              tags("volume", "disk"), tags("network-interface", "primary-eni")],
    }
    # Bare metal is "<family>.metal" or "<family>.metal-<size>".
    metal = any(part == "metal" or part.startswith("metal-") for part in itype.split(".")[1:])
    # One thread per core where the type has SMT and EC2 accepts the change:
    # x86 only (Graviton is arm64 and already one thread per core), never bare
    # metal (no CPU options there), never a type whose threads per core cannot
    # be changed (T2, C7a, M7a, R7a; C7a/M7a/R7a already run one thread per
    # core).
    if tpc >= 2 and arch == "x86_64" and not metal:
        data["CpuOptions"] = {"CoreCount": cores, "ThreadsPerCore": 1}
    print(json.dumps(data))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
