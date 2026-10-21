#!/usr/bin/env python3
"""A stand-in for the aws CLI, for test-eks-nri.sh: eks-nri.sh's cost gate,
its stamp check and its teardown/verify run against it with no cloud.

Every call is appended to $AWS_CALLS as "aws <arguments>". Answers are the
text eks-nri.sh's --query and --output text would print; a verb this fake
does not know fails loudly, so a new call in eks-nri.sh needs a line here.
Nothing is created: any create-*/run-instances verb fails.

  FAKE_SPOT_PRICES    spot prices to report (default "0.0456"); the script
                      takes the highest of the last hour
  FAKE_NO_SPOT=1      empty spot price history (the on-demand floor is used)
  FAKE_ON_DEMAND_PRICE  on-demand USD per hour (default 0.1361)
  FAKE_ARCH           describe-instance-types architecture (default arm64)
  FAKE_VCPUS          vCPUs (default 4) FAKE_CORES (2) FAKE_TPC (1)
  FAKE_STATE          JSON file of leftovers: {"clusters": {name: status},
                      "roles": [..], "profiles": [..], "repos": [..]}
  FAKE_STICKY_ROLE=1  iam delete-role "succeeds" but the role stays (a
                      deletion AWS refuses), so verify_run must report LEFT
"""
import json
import os
import sys

args = sys.argv[1:]
log_path = os.environ.get("AWS_CALLS")
if log_path:
    with open(log_path, "a") as log:
        log.write("aws " + " ".join(args) + "\n")

STATE = os.environ.get("FAKE_AWS_STATE", "")


def load():
    if not STATE:
        return {"clusters": {}, "roles": [], "profiles": [], "repos": []}
    try:
        with open(STATE) as f:
            st = json.load(f)
    except FileNotFoundError:
        st = {}
    st.setdefault("clusters", {})
    st.setdefault("roles", [])
    st.setdefault("profiles", [])
    st.setdefault("repos", [])
    return st


st = load()


def save():
    if STATE:
        with open(STATE, "w") as f:
            json.dump(st, f, indent=1, sort_keys=True)


def fail(code, msg):
    print(f"An error occurred ({code}): {msg}", file=sys.stderr)
    save()
    sys.exit(254)


def opt(name, default=None):
    for i, a in enumerate(args):
        if a == f"--{name}" and i + 1 < len(args):
            return args[i + 1]
    return default


def text(values):
    print("\t".join(values))


def empty_text():
    save()
    sys.exit(0)


OPS = {
    "get-caller-identity",
    "describe-instance-types",
    "describe-regions",
    "describe-instances",
    "describe-volumes",
    "describe-addresses",
    "describe-nat-gateways",
    "describe-vpcs",
    "describe-subnets",
    "describe-route-tables",
    "describe-security-groups",
    "describe-internet-gateways",
    "describe-network-interfaces",
    "describe-placement-groups",
    "describe-launch-templates",
    "describe-fleets",
    "describe-spot-price-history",
    "describe-cluster",
    "list-clusters",
    "delete-cluster",
    "describe-repositories",
    "delete-repository",
    "get-role",
    "get-instance-profile",
    "list-roles",
    "list-instance-profiles-for-role",
    "list-attached-role-policies",
    "list-role-policies",
    "list-open-id-connect-providers",
    "delete-role",
    "delete-instance-profile",
    "describe-log-groups",
    "get-resources",
    "get-parameter",
    "get-products",
}

svc = args[0] if args else ""
op = next((a for a in args[1:] if a in OPS), None)
if op is None:
    fail("UnknownOperation", f"fake: not implemented: aws {' '.join(args)}")
if any(a.startswith("create-") or a == "run-instances" for a in args):
    fail("FakeRefuses", f"fake: this test never creates: aws {' '.join(args)}")

# ---- sts --------------------------------------------------------------------
if svc == "sts" and op == "get-caller-identity":
    text(["123456789012", "arn:aws:iam::123456789012:root"])
    save()
    sys.exit(0)

# ---- ec2 --------------------------------------------------------------------
if svc == "ec2" and op == "describe-instance-types":
    text([
        os.environ.get("FAKE_VCPUS", "4"),
        os.environ.get("FAKE_CORES", "2"),
        os.environ.get("FAKE_TPC", "1"),
        os.environ.get("FAKE_ARCH", "arm64"),
    ])
    save()
    sys.exit(0)

if svc == "ec2" and op == "describe-regions":
    text(["us-east-1"])
    save()
    sys.exit(0)

if svc == "ec2" and op == "describe-spot-price-history":
    if os.environ.get("FAKE_NO_SPOT"):
        empty_text()
    text(os.environ.get("FAKE_SPOT_PRICES", "0.0456").split())
    save()
    sys.exit(0)

if svc == "ec2" and op == "describe-vpcs":
    # run_vpc reads Vpcs[0].VpcId: the CLI prints "None" for a missing one.
    # A list query over no VPCs prints nothing.
    if "Vpcs[0].VpcId" in (opt("query") or ""):
        text(["None"])
    else:
        empty_text()

if svc == "ec2" and op in {
    "describe-instances",
    "describe-volumes",
    "describe-addresses",
    "describe-nat-gateways",
    "describe-subnets",
    "describe-route-tables",
    "describe-security-groups",
    "describe-internet-gateways",
    "describe-network-interfaces",
    "describe-placement-groups",
    "describe-launch-templates",
    "describe-fleets",
}:
    empty_text()

if svc == "ec2" and op in {"delete-fleets", "delete-network-interface"}:
    save()
    sys.exit(0)

# ---- eks --------------------------------------------------------------------
if svc == "eks" and op == "list-clusters":
    text(list(st["clusters"]))
    save()
    sys.exit(0)

if svc == "eks" and op == "describe-cluster":
    name = opt("name", "")
    if name not in st["clusters"]:
        fail("ResourceNotFoundException", f"No cluster found with name: {name}.")
    text([st["clusters"][name]])
    save()
    sys.exit(0)

if svc == "eks" and op == "delete-cluster":
    st["clusters"].pop(opt("name", ""), None)
    save()
    sys.exit(0)

# ---- ecr --------------------------------------------------------------------
if svc == "ecr" and op == "describe-repositories":
    names = opt("repository-names")
    if names:
        if names not in st["repos"]:
            fail("RepositoryNotFoundException", f"{names} not found")
        text([names])
    else:
        text(list(st["repos"]))
    save()
    sys.exit(0)

if svc == "ecr" and op == "delete-repository":
    name = opt("repository-name", "")
    if name in st["repos"]:
        st["repos"].remove(name)
    save()
    sys.exit(0)

# ---- iam --------------------------------------------------------------------
if svc == "iam" and op == "get-role":
    name = opt("role-name", "")
    if name not in st["roles"]:
        fail("NoSuchEntity", f"Role not found: {name}")
    text([name])
    save()
    sys.exit(0)

if svc == "iam" and op == "get-instance-profile":
    name = opt("instance-profile-name", "")
    if name not in st["profiles"]:
        fail("NoSuchEntity", f"Instance profile {name} not found")
    text([name])
    save()
    sys.exit(0)

if svc == "iam" and op == "list-roles":
    text(list(st["roles"]))
    save()
    sys.exit(0)

if svc == "iam" and op in {
    "list-instance-profiles-for-role",
    "list-attached-role-policies",
    "list-role-policies",
    "list-open-id-connect-providers",
}:
    empty_text()

if svc == "iam" and op == "delete-role":
    name = opt("role-name", "")
    if not os.environ.get("FAKE_STICKY_ROLE") and name in st["roles"]:
        st["roles"].remove(name)
    save()
    sys.exit(0)

if svc == "iam" and op == "delete-instance-profile":
    name = opt("instance-profile-name", "")
    if name in st["profiles"]:
        st["profiles"].remove(name)
    save()
    sys.exit(0)

# ---- logs / tagging ---------------------------------------------------------
if svc == "logs" and op == "describe-log-groups":
    empty_text()

if svc == "resourcegroupstaggingapi" and op == "get-resources":
    empty_text()

# ---- ssm --------------------------------------------------------------------
if svc == "ssm" and op == "get-parameter":
    text(["ami-0faketest"])
    save()
    sys.exit(0)

# ---- pricing ----------------------------------------------------------------
if svc == "pricing" and op == "get-products":
    usd = os.environ.get("FAKE_ON_DEMAND_PRICE", "0.1361")
    doc = {
        "terms": {
            "OnDemand": {
                "1": {
                    "priceDimensions": {
                        "1": {"pricePerUnit": {"USD": usd}},
                    }
                }
            }
        }
    }
    print(json.dumps(doc))
    save()
    sys.exit(0)

fail("UnknownOperation", f"fake: not implemented: aws {' '.join(args)}")
