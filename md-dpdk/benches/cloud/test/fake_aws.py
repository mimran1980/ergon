#!/usr/bin/env python3
"""A stand-in for the aws CLI, for test-lab.sh: aws/lab.sh runs against it end
to end (create, launch, attach, teardown, verification) with no cloud.

Every call is appended to $AWS_CALLS as "aws <arguments>", as the CLI was
given them. Resources live in a JSON file ($FAKE_AWS_STATE), one region each
(a VPC peering connection is visible from both of its regions), with their
tags, so describe calls answer from what was created and not yet deleted, and
a delete that AWS would refuse for a dependency (a subnet with an instance in
it, a VPC with a subnet left) fails as AWS's does. Answers are the text
lab.sh's --query and --output text would print; a query this fake does not
know fails loudly, so a new call in lab.sh needs a line here.

  FAKE_ZONES          zones per region, "<region>=<zone> <zone>;..." (default
                      <region>a and <region>b)
  FAKE_NO_CAPACITY    where run-instances fails for capacity: <zone> (any
                      type) or <zone>/<instance type>
  FAKE_ARCH           what describe-instance-types reports for a Graviton-named
                      type (default arm64)
  FAKE_PENDING=1      instances stay pending (never run) until terminated
  FAKE_SPOT_LEAVE_OPEN=1  a spot launch refused for capacity leaves its spot
                      request open (tagged), for the teardown to cancel
  FAKE_RECLAIM        <n>: AWS reclaims a spot instance (state terminated,
                      reason Server.SpotInstanceTermination, its request
                      closed, its data ENIs deleted) at the n-th look at the
                      instances' state reasons, the last instance it asks
                      about; ssh: at the first look once an SSH attempt was
                      made (a fake ssh logging to $AWS_CALLS.ssh); 0: every
                      spot instance as it launches
  FAKE_STS_FAIL=1     sts get-caller-identity fails (expired credentials)
  FAKE_TERM_AT        an ec2 verb: once the first such call is answered, TERM to
                      its caller (an operator's kill -TERM of lab.sh mid-run)
"""
import json
import os
import re
import signal
import sys

args = sys.argv[1:]
with open(os.environ["AWS_CALLS"], "a") as log:
    log.write("aws " + " ".join(args) + "\n")
STATE = os.environ["FAKE_AWS_STATE"]


def load():
    try:
        with open(STATE) as f:
            return json.load(f)
    except FileNotFoundError:
        return {"next": 1, "res": {}}


st = load()


def save():
    with open(STATE, "w") as f:
        json.dump(st, f, indent=1, sort_keys=True)


def fail(code, msg):
    print(f"An error occurred ({code}): {msg}", file=sys.stderr)
    save()
    sys.exit(254)


def new_id(prefix):
    # spot requests numbered apart, so every other id is as without them
    key = "next_sir" if prefix == "sir" else "next"
    n = st.get(key, 1)
    st[key] = n + 1
    return f"{prefix}-{n:04d}"


def opt(name, default=None):
    """The value after --<name> (the first)."""
    for i, a in enumerate(args):
        if a == f"--{name}" and i + 1 < len(args):
            return args[i + 1]
    return default


def opts(name):
    """Every value after --<name>, up to the next option."""
    out = []
    for i, a in enumerate(args):
        if a == f"--{name}":
            j = i + 1
            while j < len(args) and not args[j].startswith("--"):
                out.append(args[j])
                j += 1
    return out


def tag_specs():
    """{resource type: {key: value}} from --tag-specifications."""
    out = {}
    for spec in opts("tag-specifications"):
        m = re.match(r"ResourceType=([a-z-]+),Tags=\[(.*)\]$", spec)
        if not m:
            fail("InvalidParameterValue", f"tag specification {spec}")
        out[m.group(1)] = dict(re.findall(r"\{Key=([^,]+),Value=([^}]*)\}", m.group(2)))
    return out


def filters():
    """[(name, [values])] from --filters."""
    out = []
    for f in opts("filters"):
        m = re.match(r"Name=([^,]+),Values=(.*)$", f)
        if not m:
            fail("InvalidParameterValue", f"filter {f}")
        out.append((m.group(1), m.group(2).split(",")))
    return out


def live(kind, region):
    return [(i, r) for i, r in sorted(st["res"].items())
            if r["kind"] == kind and (region in r.get("regions", [r.get("region")])) and not r.get("deleted")]


def matches(r, region, fs):
    for name, values in fs:
        if name.startswith("tag:"):
            tags = r.get("tags_by_region", {}).get(region, r.get("tags", {}))
            if tags.get(name[4:]) not in values:
                return False
        elif name == "vpc-id":
            if r.get("vpc") not in values:
                return False
        elif name == "attachment.vpc-id":
            if r.get("attached") not in values:
                return False
        elif name == "instance-state-name":
            if r.get("state") not in values:
                return False
        elif name == "state" and r["kind"] == "sir":
            if r.get("state") not in values:
                return False
        elif name == "instance-type":
            pass
        else:
            fail("InvalidParameterValue", f"fake: filter {name}")
    return True


def found(kind, region):
    return [(i, r) for i, r in live(kind, region) if matches(r, region, filters())]


def gone(iid, reason):
    """An instance terminated (by a call, or by AWS), with its data ENIs
    (DeleteOnTermination) and, a spot one, its spot request (closed)."""
    inst = st["res"][iid]
    inst["state"], inst["reason"] = "terminated", reason
    for _, e in live("eni", inst["region"]):
        if e.get("instance") == iid:
            e["deleted"] = True
    if inst.get("sir") and st["res"][inst["sir"]]["state"] == "active":
        st["res"][inst["sir"]]["state"] = "closed"


def reclaim(iid):
    """AWS interrupts a spot instance: terminated, Server.SpotInstanceTermination."""
    gone(iid, "Server.SpotInstanceTermination")


def text(values):
    print("\t".join(values))


def zones(region):
    spec = os.environ.get("FAKE_ZONES", "")
    for part in spec.split(";"):
        if part.startswith(region + "="):
            return part.split("=", 1)[1].split()
    return [region + "a", region + "b"]


def cores(itype):
    size = itype.split(".", 1)[1]
    return {"large": 1, "xlarge": 2, "2xlarge": 4, "4xlarge": 8, "8xlarge": 16}.get(size, 48)


def in_cidr(ip, cidr):
    net, bits = cidr.split("/")
    to = lambda a: sum(int(x) << (8 * (3 - k)) for k, x in enumerate(a.split(".")))
    mask = (0xFFFFFFFF << (32 - int(bits))) & 0xFFFFFFFF
    return to(ip) & mask == to(net) & mask


def overlaps(a, b):
    return in_cidr(a.split("/")[0], b) or in_cidr(b.split("/")[0], a)


def create(kind, prefix, region, rtype, **fields):
    rid = new_id(prefix)
    st["res"][rid] = dict(kind=kind, region=region, tags=tag_specs().get(rtype, {}), **fields)
    return rid


def ec2(region, verb):
    q = opt("query", "")
    if verb == "describe-instance-type-offerings":
        text(zones(region))
    elif verb == "describe-instance-types":
        t = opt("instance-types")
        if "SupportedArchitectures" in q:
            arm = re.match(r"^[a-z]+[0-9]+g", t.split(".")[0]) or t.startswith("a1.")
            text([os.environ.get("FAKE_ARCH", "arm64")] if arm else ["i386", "x86_64"])
        elif "DefaultCores" in q:
            text([str(cores(t))])
        else:
            fail("InvalidParameterValue", f"fake: describe-instance-types --query {q}")
    elif verb == "describe-images":
        arm = any("arm64" in f for f in opts("filters"))
        text([f"ami-{'arm' if arm else 'x86'}-{region}"] + (["/dev/xvda"] if "RootDeviceName" in q else []))
    elif verb == "create-vpc":
        cidr = opt("cidr-block")
        print(create("vpc", "vpc", region, "vpc", cidr=cidr))
    elif verb == "create-internet-gateway":
        print(create("igw", "igw", region, "internet-gateway"))
    elif verb == "attach-internet-gateway":
        st["res"][opt("internet-gateway-id")]["attached"] = opt("vpc-id")
    elif verb == "detach-internet-gateway":
        st["res"][opt("internet-gateway-id")].pop("attached", None)
    elif verb == "create-route-table":
        print(create("rt", "rtb", region, "route-table", vpc=opt("vpc-id"), routes=[]))
    elif verb == "create-route":
        rt = st["res"][opt("route-table-id")]
        target = opt("gateway-id") or opt("vpc-peering-connection-id")
        if not target or target not in st["res"]:
            fail("InvalidParameterValue", "route target")
        rt["routes"].append([opt("destination-cidr-block"), target])
        print('{"Return": true}')
    elif verb == "create-security-group":
        print(create("sg", "sg", region, "security-group", vpc=opt("vpc-id"), name=opt("group-name"), rules=[]))
    elif verb == "authorize-security-group-ingress":
        st["res"][opt("group-id")]["rules"].append([opt("protocol"), opt("port"), opt("cidr")])
        print('{"Return": true}')
    elif verb == "import-key-pair":
        name = opt("key-name")
        st["res"][f"key-{region}-{name}"] = dict(kind="key", region=region, name=name, tags=tag_specs().get("key-pair", {}))
        print(json.dumps({"KeyName": name}))
    elif verb == "create-placement-group":
        name = opt("group-name")
        if any(r.get("name") == name for _, r in live("pg", region)):
            fail("InvalidPlacementGroup.Duplicate", name)
        rid = create("pg", "pg", region, "placement-group", name=name, strategy=opt("strategy"))
        print(rid if "GroupId" in q else json.dumps({"PlacementGroup": {"GroupName": name}}))
    elif verb == "create-subnet":
        vpc, cidr, az = opt("vpc-id"), opt("cidr-block"), opt("availability-zone")
        if az not in zones(region):
            fail("InvalidParameterValue", f"zone {az} is not in {region}")
        if not in_cidr(cidr.split("/")[0], st["res"][vpc]["cidr"]):
            fail("InvalidSubnet.Range", cidr)
        for _, s in live("subnet", region):
            if s["vpc"] == vpc and overlaps(s["cidr"], cidr):
                fail("InvalidSubnet.Conflict", cidr)
        print(create("subnet", "subnet", region, "subnet", vpc=vpc, cidr=cidr, az=az))
    elif verb == "modify-subnet-attribute":
        pass
    elif verb == "associate-route-table":
        st["res"][opt("subnet-id")]["rt"] = opt("route-table-id")
        print("rtbassoc-" + opt("subnet-id"))
    elif verb == "run-instances":
        subnet = st["res"][opt("subnet-id")]
        full = os.environ.get("FAKE_NO_CAPACITY", "").split()
        if subnet["az"] in full or f"{subnet['az']}/{opt('instance-type')}" in full:
            if (opt("instance-market-options") or "").startswith("MarketType=spot") and os.environ.get("FAKE_SPOT_LEAVE_OPEN") == "1":
                rid = new_id("sir")
                st["res"][rid] = dict(kind="sir", region=region, tags=tag_specs().get("spot-instances-request", {}),
                                      state="open", instance=None)
            fail("InsufficientInstanceCapacity", f"no {opt('instance-type')} in {subnet['az']}")
        pg = (opt("placement") or "").removeprefix("GroupName=")
        if pg and not any(r.get("name") == pg for _, r in live("pg", region)):
            fail("InvalidPlacementGroup.Unknown", pg)
        n = st["next"]
        tags = tag_specs()
        market = opt("instance-market-options") or ""
        spot = market.startswith("MarketType=spot")
        if spot and market != "MarketType=spot,SpotOptions={SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}":
            fail("InvalidParameterValue", f"fake: market options {market}")
        if spot and opt("instance-initiated-shutdown-behavior") != "terminate":
            fail("InvalidParameterCombination", "a one-time spot instance terminates on shutdown")
        iid = new_id("i")
        base = subnet["cidr"].split("/")[0].rsplit(".", 1)[0]
        st["res"][iid] = dict(kind="instance", region=region, tags=tags.get("instance", {}), subnet=opt("subnet-id"),
                              state="pending" if os.environ.get("FAKE_PENDING") == "1" else "running",
                              type=opt("instance-type"), cpu=opt("cpu-options"),
                              public=f"198.51.100.{n % 250}", private=f"{base}.{10 + n % 200}")
        if spot:
            rid = new_id("sir")
            st["res"][rid] = dict(kind="sir", region=region, tags=tags.get("spot-instances-request", {}),
                                  state="active", instance=iid)
            st["res"][iid]["sir"] = rid
            if os.environ.get("FAKE_RECLAIM") == "0":
                reclaim(iid)
        print(iid)
    elif verb == "terminate-instances":
        for iid in opts("instance-ids"):
            gone(iid, "Client.UserInitiatedShutdown")
        print("{}")
    elif verb == "describe-instances" and "StateReason" in q:
        # each instance's state and why (a spot watch): AWS may reclaim one first
        ids = opts("instance-ids")
        st["looks"] = st.get("looks", 0) + 1
        at = os.environ.get("FAKE_RECLAIM")
        if (at == str(st["looks"]) or at == "ssh" and os.path.exists(os.environ["AWS_CALLS"] + ".ssh")) and \
                ids and st["res"][ids[-1]].get("sir") and st["res"][ids[-1]]["state"] == "running":
            reclaim(ids[-1])
        if q != "Reservations[].Instances[].[InstanceId,State.Name,StateReason.Code]":
            fail("InvalidParameterValue", f"fake: describe-instances --query {q}")
        for i in ids:
            print(f"{i}\t{st['res'][i]['state']}\t{st['res'][i].get('reason') or 'None'}")
    elif verb == "describe-instances":
        ids = opts("instance-ids")
        rows = [(i, st["res"][i]) for i in ids] if ids else found("instance", region)
        field = {"PublicIpAddress": "public", "PrivateIpAddress": "private", "InstanceId": None,
                 "State.Name": "state"}
        key = next((k for k in field if q.endswith(k) or f".{k}" in q or q.endswith(k + "]")), None)
        if key is None:
            fail("InvalidParameterValue", f"fake: describe-instances --query {q}")
        text([i if field[key] is None else r[field[key]] for i, r in rows])
    elif verb == "create-network-interface":
        subnet = st["res"][opt("subnet-id")]
        ip = opt("private-ip-address")
        if not in_cidr(ip, subnet["cidr"]):
            fail("InvalidParameterValue", f"{ip} is not in {subnet['cidr']}")
        print(create("eni", "eni", region, "network-interface", subnet=opt("subnet-id"), ip=ip))
    elif verb == "attach-network-interface":
        eni = st["res"][opt("network-interface-id")]
        inst = st["res"][opt("instance-id")]
        if st["res"][eni["subnet"]]["az"] != st["res"][inst["subnet"]]["az"]:
            fail("InvalidParameterCombination", "the network interface and the instance are in different zones")
        eni["instance"] = opt("instance-id")
        print("eni-attach-" + opt("network-interface-id"))
    elif verb == "modify-network-interface-attribute":
        pass
    elif verb == "describe-network-interfaces":
        text([i for i, _ in found("eni", region)])
    elif verb == "delete-network-interface":
        st["res"][opt("network-interface-id")]["deleted"] = True
    elif verb == "describe-vpcs":
        ids = [i for i, _ in found("vpc", region)]
        text(ids[:1] or ["None"]) if "[0]" in q else text(ids)
    elif verb == "describe-internet-gateways":
        text([i for i, _ in found("igw", region)])
    elif verb == "delete-internet-gateway":
        if st["res"][opt("internet-gateway-id")].get("attached"):
            fail("DependencyViolation", "attached")
        st["res"][opt("internet-gateway-id")]["deleted"] = True
    elif verb == "describe-subnets":
        text([i for i, _ in found("subnet", region)])
    elif verb == "delete-subnet":
        sid = opt("subnet-id")
        if any(r["subnet"] == sid and r["state"] != "terminated" for _, r in live("instance", region)) or \
                any(r["subnet"] == sid for _, r in live("eni", region)):
            fail("DependencyViolation", f"{sid} has dependencies")
        st["res"][sid]["deleted"] = True
    elif verb == "describe-route-tables":
        text([i for i, _ in found("rt", region)])
    elif verb == "delete-route-table":
        rid = opt("route-table-id")
        if any(r.get("rt") == rid for _, r in live("subnet", region)):
            fail("DependencyViolation", f"{rid} is associated")
        st["res"][rid]["deleted"] = True
    elif verb == "describe-security-groups":
        text([i for i, _ in found("sg", region)])
    elif verb == "delete-security-group":
        st["res"][opt("group-id")]["deleted"] = True
    elif verb == "delete-vpc":
        vpc = opt("vpc-id")
        for kind in ("subnet", "sg", "rt"):
            if any(r.get("vpc") == vpc for _, r in live(kind, region)):
                fail("DependencyViolation", f"{vpc} has a {kind}")
        if any(r.get("attached") == vpc for _, r in live("igw", region)):
            fail("DependencyViolation", f"{vpc} has an internet gateway")
        if any(vpc in (r["vpc"], r["peer_vpc"]) and r["status"] in ("pending-acceptance", "active")
               for _, r in live("pcx", region)):
            fail("DependencyViolation", f"{vpc} has a peering connection")
        st["res"][vpc]["deleted"] = True
    elif verb == "delete-placement-group":
        name = opt("group-name")
        pgs = [r for _, r in live("pg", region) if r["name"] == name]
        if not pgs:
            fail("InvalidPlacementGroup.Unknown", name)
        pgs[0]["deleted"] = True
    elif verb == "delete-key-pair":
        k = st["res"].get(f"key-{region}-{opt('key-name')}")
        if k:
            k["deleted"] = True
    elif verb in ("describe-volumes", "describe-addresses"):
        text([])
    elif verb == "describe-placement-groups":
        text([r["name"] for _, r in found("pg", region)])
    elif verb == "describe-key-pairs":
        text([r["name"] for _, r in found("key", region)])
    elif verb == "create-vpc-peering-connection":
        peer_region = opt("peer-region", region)
        rid = create("pcx", "pcx", region, "vpc-peering-connection", vpc=opt("vpc-id"), peer_vpc=opt("peer-vpc-id"),
                     regions=[region, peer_region], status="pending-acceptance")
        st["res"][rid]["tags_by_region"] = {region: st["res"][rid]["tags"]}
        print(rid)
    elif verb == "accept-vpc-peering-connection":
        r = st["res"][opt("vpc-peering-connection-id")]
        if region != r["regions"][1]:
            fail("OperationNotPermitted", "accept from the accepter's region")
        r["status"] = "active"
        print("active")
    elif verb == "describe-vpc-peering-connections":
        ids = opts("vpc-peering-connection-ids")
        rows = [(i, st["res"][i]) for i in ids if region in st["res"][i]["regions"]] if ids else \
            [(i, r) for i, r in found("pcx", region) if r["status"] != "deleted"]
        text([r["status"] if q.endswith("Status.Code") else i for i, r in rows])
    elif verb == "delete-vpc-peering-connection":
        r = st["res"][opt("vpc-peering-connection-id")]
        r["status"] = "deleted"
        r["deleted"] = True
        print('{"Return": true}')
    elif verb == "create-tags":
        tags = dict(re.findall(r"Key=([^,]+),Value=([^ ]*)", " ".join(opts("tags"))))
        for rid in opts("resources"):
            r = st["res"][rid]
            if r["kind"] == "pcx":
                r.setdefault("tags_by_region", {})[region] = tags
            else:
                r["tags"].update(tags)
    elif verb == "describe-spot-instance-requests":
        if q != "SpotInstanceRequests[].SpotInstanceRequestId":
            fail("InvalidParameterValue", f"fake: describe-spot-instance-requests --query {q}")
        text([i for i, _ in found("sir", region)])
    elif verb == "cancel-spot-instance-requests":
        for rid in opts("spot-instance-request-ids"):
            if st["res"][rid]["state"] in ("open", "active"):
                st["res"][rid]["state"] = "cancelled"
        print("{}")
    elif verb == "wait":
        pass
    else:
        fail("InvalidAction", f"fake: ec2 {verb}")


if args[:1] == ["sts"]:
    if os.environ.get("FAKE_STS_FAIL") == "1":
        fail("ExpiredToken", "the security token included in the request is expired")
    print('{"Account": "000000000000"}')
elif args[:2] == ["ec2", "--region"] and len(args) >= 4:
    ec2(args[2], args[3])
else:
    fail("InvalidAction", "fake: " + " ".join(args))
save()
if os.environ.get("FAKE_TERM_AT") in args[3:4] and not os.path.exists(STATE + ".termed"):
    open(STATE + ".termed", "w").close()
    os.kill(os.getppid(), signal.SIGTERM)
