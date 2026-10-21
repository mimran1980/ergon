//! The quick-ACK route check: the `RTM_GETROUTE` request and reply against
//! bytes a Linux kernel produced, the rendering of routes and fix commands,
//! the policy through an injected lookup, and (on Linux) the real lookup and
//! the driver's per-connection check.
#![allow(missing_docs)]

use std::cell::{Cell, RefCell};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use ergon_io::route::{
    self, Flow, QuickAck, QuickAckCheck, QuickAckError, REQUEST_MAX, RTAX_QUICKACK, RTAX_RTO_MIN,
    Route, RouteCheck, RouteError, Warned,
};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

// Captured on Linux 6.12 (Docker Desktop's linuxkit kernel) with iproute2
// 6.1: each request was built by a Python script independent of ergon-io
// (RTM_GETROUTE, RTM_F_FIB_MATCH | RTM_F_LOOKUP_TABLE, RTA_DST, RTA_SRC,
// optionally RTA_OIF and RTA_MARK, then RTA_IP_PROTO, RTA_SPORT, RTA_DPORT)
// and sent on a NETLINK_ROUTE socket; each reply is the kernel's answer. The
// comment above each pair is what `ip route get fibmatch <dst> from <src>`
// printed for the same lookup. The container's eth0 is 172.17.0.2/16
// (ifindex 11), its gateway 172.17.0.1; dum0 (ifindex 12) is a dummy
// interface holding 2001:db8:1::2/64. The host was little-endian (aarch64):
// netlink fields are in host byte order, so these bytes hold for x86_64 and
// aarch64, not for big-endian hosts.

// default via 172.17.0.1 dev eth0
// (203.0.113.7:443 from 172.17.0.2:40000, seq 1)
const V4_DEFAULT_PLAIN_REQUEST: [u8; 68] = [
    0x44, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x20, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00,
    0xcb, 0x00, 0x71, 0x07, 0x08, 0x00, 0x02, 0x00, 0xac, 0x11, 0x00, 0x02, 0x05, 0x00, 0x1b, 0x00,
    0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x1c, 0x00, 0x9c, 0x40, 0x00, 0x00, 0x06, 0x00, 0x1d, 0x00,
    0x01, 0xbb, 0x00, 0x00,
];
const V4_DEFAULT_PLAIN_REPLY: [u8; 52] = [
    0x34, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0xac, 0x11, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00,
    0x0b, 0x00, 0x00, 0x00,
];

// The same lookup with the socket bound to eth0 (oif 11) and marked 7
// (port 40001, seq 2); the kernel answered with the same route.
const V4_OIF_MARK_REQUEST: [u8; 84] = [
    0x54, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x20, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00,
    0xcb, 0x00, 0x71, 0x07, 0x08, 0x00, 0x02, 0x00, 0xac, 0x11, 0x00, 0x02, 0x08, 0x00, 0x04, 0x00,
    0x0b, 0x00, 0x00, 0x00, 0x08, 0x00, 0x10, 0x00, 0x07, 0x00, 0x00, 0x00, 0x05, 0x00, 0x1b, 0x00,
    0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x1c, 0x00, 0x9c, 0x41, 0x00, 0x00, 0x06, 0x00, 0x1d, 0x00,
    0x01, 0xbb, 0x00, 0x00,
];

// After `ip route change default via 172.17.0.1 dev eth0 quickack 1 rto_min 5ms`:
// default via 172.17.0.1 dev eth0 rto_min lock 5ms quickack 1
// (seq 3)
const V4_DEFAULT_QUICKACK_REPLY: [u8; 80] = [
    0x50, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x08, 0x00, 0x08, 0x00, 0x01, 0x00, 0x00, 0x20, 0x00, 0x00,
    0x08, 0x00, 0x0d, 0x00, 0x05, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x05, 0x00, 0xac, 0x11, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00, 0x0b, 0x00, 0x00, 0x00,
];

// 172.17.0.0/16 dev eth0 proto kernel scope link src 172.17.0.2 rto_min lock 5ms
// (172.17.0.99:443 from 172.17.0.2:40002, seq 4)
const V4_SUBNET_RTO_MIN_REPLY: [u8; 80] = [
    0x50, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x44, 0x00, 0x00, 0x00,
    0x02, 0x10, 0x00, 0x00, 0xfe, 0x02, 0xfd, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0xac, 0x11, 0x00, 0x00, 0x14, 0x00, 0x08, 0x00,
    0x08, 0x00, 0x01, 0x00, 0x00, 0x20, 0x00, 0x00, 0x08, 0x00, 0x0d, 0x00, 0x05, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x07, 0x00, 0xac, 0x11, 0x00, 0x02, 0x08, 0x00, 0x04, 0x00, 0x0b, 0x00, 0x00, 0x00,
];

// Added with `quickack 0` (which the kernel stores as no quickack) and found
// through `ip rule add to 198.51.100.0/24 lookup 100`:
// 198.51.100.0/24 via 172.17.0.1 dev eth0 table 100 proto static metric 50 mtu lock 1400 initcwnd 10 congctl cubic
// (198.51.100.9:443 from 172.17.0.2:40003, seq 5)
const V4_TABLE_LOCKED_REPLY: [u8; 108] = [
    0x6c, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00,
    0x02, 0x18, 0x00, 0x00, 0x64, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0x64, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0xc6, 0x33, 0x64, 0x00, 0x08, 0x00, 0x06, 0x00,
    0x32, 0x00, 0x00, 0x00, 0x28, 0x00, 0x08, 0x00, 0x08, 0x00, 0x01, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x02, 0x00, 0x78, 0x05, 0x00, 0x00, 0x08, 0x00, 0x0b, 0x00, 0x0a, 0x00, 0x00, 0x00,
    0x0a, 0x00, 0x10, 0x00, 0x63, 0x75, 0x62, 0x69, 0x63, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00,
    0xac, 0x11, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00, 0x0b, 0x00, 0x00, 0x00,
];

// local 127.0.0.1 dev lo table local proto kernel scope host src 127.0.0.1
// (127.0.0.1:443 from 127.0.0.1:40004, seq 6)
const V4_LOCAL_REQUEST: [u8; 68] = [
    0x44, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x01, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x20, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00,
    0x7f, 0x00, 0x00, 0x01, 0x08, 0x00, 0x02, 0x00, 0x7f, 0x00, 0x00, 0x01, 0x05, 0x00, 0x1b, 0x00,
    0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x1c, 0x00, 0x9c, 0x44, 0x00, 0x00, 0x06, 0x00, 0x1d, 0x00,
    0x01, 0xbb, 0x00, 0x00,
];
const V4_LOCAL_REPLY: [u8; 60] = [
    0x3c, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x4e, 0x00, 0x00, 0x00,
    0x02, 0x20, 0x00, 0x00, 0xff, 0x02, 0xfe, 0x02, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xff, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0x7f, 0x00, 0x00, 0x01, 0x08, 0x00, 0x07, 0x00,
    0x7f, 0x00, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00,
];

// After `ip route add unreachable 192.0.2.0/24`: "No route to host"
// (192.0.2.1:443 from 172.17.0.2:40005, seq 7): NLMSG_ERROR, -EHOSTUNREACH.
const V4_UNREACHABLE_REPLY: [u8; 88] = [
    0x58, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x53, 0x00, 0x00, 0x00,
    0x8f, 0xff, 0xff, 0xff, 0x44, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x01, 0x00, 0x07, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x02, 0x20, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00,
    0x08, 0x00, 0x01, 0x00, 0xc0, 0x00, 0x02, 0x01, 0x08, 0x00, 0x02, 0x00, 0xac, 0x11, 0x00, 0x02,
    0x05, 0x00, 0x1b, 0x00, 0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x1c, 0x00, 0x9c, 0x45, 0x00, 0x00,
    0x06, 0x00, 0x1d, 0x00, 0x01, 0xbb, 0x00, 0x00,
];

// 100.64.0.0/10 via 10.99.0.1 dev eth0 onlink
// (100.64.0.9:443 from 172.17.0.2:40010, seq 12)
const V4_ONLINK_REPLY: [u8; 60] = [
    0x3c, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x58, 0x00, 0x00, 0x00,
    0x02, 0x0a, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x04, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0x64, 0x40, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00,
    0x0a, 0x63, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00, 0x0b, 0x00, 0x00, 0x00,
];

// 198.18.0.0/15 nexthop via 172.17.0.1 dev eth0 weight 1 nexthop via 172.17.0.3 dev eth0 weight 1
// (198.18.0.5:443 from 172.17.0.2:40011, seq 13)
const V4_MULTIPATH_REPLY: [u8; 80] = [
    0x50, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x5d, 0x00, 0x00, 0x00,
    0x02, 0x0f, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0xc6, 0x12, 0x00, 0x00, 0x24, 0x00, 0x09, 0x00,
    0x10, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0xac, 0x11, 0x00, 0x01,
    0x10, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0xac, 0x11, 0x00, 0x03,
];

// 198.19.0.0/16 nhid 5 via 172.17.0.1 dev eth0
// (198.19.0.5:443 from 172.17.0.2:40012, seq 14)
const V4_NHID_REPLY: [u8; 68] = [
    0x44, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x63, 0x00, 0x00, 0x00,
    0x02, 0x10, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0xc6, 0x13, 0x00, 0x00, 0x08, 0x00, 0x1e, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0xac, 0x11, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00,
    0x0b, 0x00, 0x00, 0x00,
];

// In table 1000 (above 255, so the route message's 8-bit table is 252,
// RT_TABLE_COMPAT, and RTA_TABLE carries it), found through
// `ip rule add to 192.0.2.0/24 lookup 1000`, from 172.17.0.3 (another run):
// 192.0.2.0/24 via 172.17.0.1 dev eth0 table 1000 metric 300 window 65535 rtt 4ms rttvar 1ms features ecn
// (added as `... rtt 33 rttvar 7 window 65535 features ecn`, which iproute2
// stores unscaled; 192.0.2.9:443 from 172.17.0.3:40013, seq 15)
const V4_RTT_TABLE_REPLY: [u8; 104] = [
    0x68, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00,
    0x02, 0x18, 0x00, 0x00, 0xfc, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xe8, 0x03, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0xc0, 0x00, 0x02, 0x00, 0x08, 0x00, 0x06, 0x00,
    0x2c, 0x01, 0x00, 0x00, 0x24, 0x00, 0x08, 0x00, 0x08, 0x00, 0x03, 0x00, 0xff, 0xff, 0x00, 0x00,
    0x08, 0x00, 0x04, 0x00, 0x21, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0x07, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x0c, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0xac, 0x11, 0x00, 0x01,
    0x08, 0x00, 0x04, 0x00, 0x0b, 0x00, 0x00, 0x00,
];

// 2001:db8:2::/48 via 2001:db8:1::1 dev dum0 metric 256 rto_min lock 5ms quickack 1 pref medium
// ([2001:db8:2::5]:443 from [2001:db8:1::2]:40006, seq 8)
const V6_QUICKACK_REQUEST: [u8; 92] = [
    0x5c, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0a, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00,
    0x20, 0x01, 0x0d, 0xb8, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05,
    0x14, 0x00, 0x02, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x02, 0x05, 0x00, 0x1b, 0x00, 0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x1c, 0x00,
    0x9c, 0x46, 0x00, 0x00, 0x06, 0x00, 0x1d, 0x00, 0x01, 0xbb, 0x00, 0x00,
];
const V6_QUICKACK_REPLY: [u8; 164] = [
    0xa4, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x6b, 0x00, 0x00, 0x00,
    0x0a, 0x30, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x02, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x08, 0x00, 0x08, 0x00, 0x01, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x08, 0x00, 0x0d, 0x00, 0x05, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x06, 0x00, 0x00, 0x01, 0x00, 0x00, 0x14, 0x00, 0x05, 0x00,
    0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x00, 0x04, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x24, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x14, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

// 2001:db8:1::/64 dev dum0 proto kernel metric 256 pref medium
// ([2001:db8:1::9]:443 from [2001:db8:1::2]:40007, seq 9)
const V6_PLAIN_REPLY: [u8; 116] = [
    0x74, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x6f, 0x00, 0x00, 0x00,
    0x0a, 0x40, 0x00, 0x00, 0xfe, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x06, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x08, 0x00, 0x04, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x24, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x14, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

// local ::1 dev lo table local proto kernel metric 0 pref medium
// ([::1]:443 from [::1]:40008, seq 10)
const V6_LOCAL_REPLY: [u8; 116] = [
    0x74, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x73, 0x00, 0x00, 0x00,
    0x0a, 0x80, 0x00, 0x00, 0xff, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xff, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x08, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x24, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x14, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

// No IPv6 default route: "Network is unreachable"
// ([2001:db8:ffff::1]:443 from [2001:db8:1::2]:40009, seq 11): -ENETUNREACH.
const V6_NO_ROUTE_REPLY: [u8; 112] = [
    0x70, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x77, 0x00, 0x00, 0x00,
    0x9b, 0xff, 0xff, 0xff, 0x5c, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x01, 0x00, 0x0b, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x0a, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00,
    0x14, 0x00, 0x01, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01, 0x14, 0x00, 0x02, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x05, 0x00, 0x1b, 0x00, 0x06, 0x00, 0x00, 0x00,
    0x06, 0x00, 0x1c, 0x00, 0x9c, 0x49, 0x00, 0x00, 0x06, 0x00, 0x1d, 0x00, 0x01, 0xbb, 0x00, 0x00,
];

// The fixtures below were captured on the same kernel (Linux 6.12, aarch64)
// with no iproute2 in the container: the routes and rules were added over
// rtnetlink by a Python script, and each request was built by another one
// independent of ergon-io, then sent as above. eth0 is 172.17.0.2/16
// (ifindex 11), the peer 172.17.0.4; dum0 (ifindex 12) holds 2001:db8:1::2/64.

// A route for one type of service, which only flows with that TOS use:
// 172.17.0.4 tos 0x10 dev eth0 proto static scope link metric 100 rto_min lock 29ms
// (added beside `172.17.0.4 dev eth0 ... metric 100`; 172.17.0.4:443 from
// 172.17.0.2:40015 with TOS 0x10, seq 16). A socket with IP_TOS 0x10 got
// this route's 29 ms RTO floor; the request without the TOS is answered with
// the other route.
const V4_TOS_REQUEST: [u8; 68] = [
    0x44, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x01, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x20, 0x20, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00,
    0xac, 0x11, 0x00, 0x04, 0x08, 0x00, 0x02, 0x00, 0xac, 0x11, 0x00, 0x02, 0x05, 0x00, 0x1b, 0x00,
    0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x1c, 0x00, 0x9c, 0x4f, 0x00, 0x00, 0x06, 0x00, 0x1d, 0x00,
    0x01, 0xbb, 0x00, 0x00,
];
const V4_TOS_REPLY: [u8; 80] = [
    0x50, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0xa8, 0x0e, 0x00, 0x00,
    0x02, 0x20, 0x00, 0x10, 0xfe, 0x04, 0xfd, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00, 0xac, 0x11, 0x00, 0x04, 0x08, 0x00, 0x06, 0x00,
    0x64, 0x00, 0x00, 0x00, 0x14, 0x00, 0x08, 0x00, 0x08, 0x00, 0x01, 0x00, 0x00, 0x20, 0x00, 0x00,
    0x08, 0x00, 0x0d, 0x00, 0x1d, 0x00, 0x00, 0x00, 0x08, 0x00, 0x04, 0x00, 0x0b, 0x00, 0x00, 0x00,
];

// An IPv6 policy rule on the traffic class (`ip -6 rule add tos 0x10 lookup
// 600`) to 2001:db8:8::/48 dev dum0 table 600 proto static metric 1024
// rto_min lock 41ms pref medium ([2001:db8:8::9]:443 from
// [2001:db8:1::2]:40016 with traffic class 0x10, seq 17; without it the
// kernel finds no route).
const V6_TCLASS_RULE_REQUEST: [u8; 92] = [
    0x5c, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x01, 0x00, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0a, 0x80, 0x80, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00,
    0x20, 0x01, 0x0d, 0xb8, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09,
    0x14, 0x00, 0x02, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x02, 0x05, 0x00, 0x1b, 0x00, 0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x1c, 0x00,
    0x9c, 0x50, 0x00, 0x00, 0x06, 0x00, 0x1d, 0x00, 0x01, 0xbb, 0x00, 0x00,
];
const V6_TCLASS_RULE_REPLY: [u8; 136] = [
    0x88, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0xa8, 0x0e, 0x00, 0x00,
    0x0a, 0x30, 0x00, 0x00, 0xfc, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0x58, 0x02, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x08, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x08, 0x00, 0x08, 0x00, 0x01, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x08, 0x00, 0x0d, 0x00, 0x29, 0x00, 0x00, 0x00, 0x08, 0x00, 0x06, 0x00,
    0x00, 0x04, 0x00, 0x00, 0x08, 0x00, 0x04, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x24, 0x00, 0x0c, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// A route with an expiry (added with RTA_EXPIRES 300, as router
// advertisements set one): 2001:db8:5::/48 via 2001:db8:1::1 dev dum0 proto
// static metric 1024 expires 122sec pref medium. The expiry is in
// RTA_CACHEINFO (rta_expires, at bytes 104..108).
// ([2001:db8:5::9]:443 from [2001:db8:1::2]:40017, seq 18)
const V6_EXPIRES_REPLY: [u8; 136] = [
    0x88, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0xa8, 0x0e, 0x00, 0x00,
    0x0a, 0x30, 0x00, 0x00, 0xfe, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x05, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x06, 0x00, 0x00, 0x04, 0x00, 0x00,
    0x14, 0x00, 0x05, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x24, 0x00, 0x0c, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf6, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// A source-specific route: 2001:db8:6::/48 from 2001:db8:1::/64 dev dum0
// proto static metric 1024 pref medium (RTA_SRC's address at bytes 60..76).
// ([2001:db8:6::9]:443 from [2001:db8:1::2]:40018, seq 19)
const V6_FROM_REPLY: [u8; 136] = [
    0x88, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0xa8, 0x0e, 0x00, 0x00,
    0x0a, 0x30, 0x40, 0x00, 0xfe, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x06, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x02, 0x00, 0x20, 0x01, 0x0d, 0xb8,
    0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x06, 0x00,
    0x00, 0x04, 0x00, 0x00, 0x08, 0x00, 0x04, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x24, 0x00, 0x0c, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// A route of protocol `ra` (9), as the kernel installs from router
// advertisements: 2001:db8:9::/48 via 2001:db8:1::1 dev dum0 proto ra metric
// 1024 pref medium ([2001:db8:9::9]:443 from [2001:db8:1::2]:40019, seq 20).
const V6_PROTO_RA_REPLY: [u8; 136] = [
    0x88, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0xa8, 0x0e, 0x00, 0x00,
    0x0a, 0x30, 0x00, 0x00, 0xfe, 0x09, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00,
    0xfe, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x09, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x06, 0x00, 0x00, 0x04, 0x00, 0x00,
    0x14, 0x00, 0x05, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01, 0x08, 0x00, 0x04, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x24, 0x00, 0x0c, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Every reply fixture with the sequence number it answers.
const REPLIES: [(&[u8], u32); 19] = [
    (&V4_DEFAULT_PLAIN_REPLY, 1),
    (&V4_DEFAULT_QUICKACK_REPLY, 3),
    (&V4_SUBNET_RTO_MIN_REPLY, 4),
    (&V4_TABLE_LOCKED_REPLY, 5),
    (&V4_LOCAL_REPLY, 6),
    (&V4_UNREACHABLE_REPLY, 7),
    (&V4_ONLINK_REPLY, 12),
    (&V4_MULTIPATH_REPLY, 13),
    (&V4_NHID_REPLY, 14),
    (&V4_RTT_TABLE_REPLY, 15),
    (&V6_QUICKACK_REPLY, 8),
    (&V6_PLAIN_REPLY, 9),
    (&V6_LOCAL_REPLY, 10),
    (&V6_NO_ROUTE_REPLY, 11),
    (&V4_TOS_REPLY, 16),
    (&V6_TCLASS_RULE_REPLY, 17),
    (&V6_EXPIRES_REPLY, 18),
    (&V6_FROM_REPLY, 19),
    (&V6_PROTO_RA_REPLY, 20),
];

const ME: Ipv4Addr = Ipv4Addr::new(172, 17, 0, 2);
const GW: Ipv4Addr = Ipv4Addr::new(172, 17, 0, 1);

const fn v4(ip: Ipv4Addr, port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(ip), port)
}

fn v6(ip: &str, port: u16) -> R<SocketAddr> {
    Ok(SocketAddr::new(IpAddr::V6(ip.parse::<Ipv6Addr>()?), port))
}

fn encode(flow: &Flow, seq: u32) -> Vec<u8> {
    let mut buf = [0u8; REQUEST_MAX];
    let n = route::encode_request(flow, seq, &mut buf);
    buf[..n].to_vec()
}

/// The reply decoded, with the interface name a live lookup would add.
fn decoded(reply: &[u8], seq: u32, dev: &str) -> R<Route> {
    let mut r = route::decode_reply(reply, seq)?;
    r.dev = Some(dev.to_owned());
    Ok(r)
}

fn plain_flow() -> Flow {
    Flow::new(v4(ME, 40000), v4(Ipv4Addr::new(203, 0, 113, 7), 443))
}

// ---- the request ----------------------------------------------------------

#[test]
fn requests_match_the_bytes_the_kernel_answered() -> R {
    assert_eq!(encode(&plain_flow(), 1), V4_DEFAULT_PLAIN_REQUEST);
    let bound = Flow::new(v4(ME, 40001), v4(Ipv4Addr::new(203, 0, 113, 7), 443))
        .with_oif(11)
        .with_mark(7);
    assert_eq!(encode(&bound, 2), V4_OIF_MARK_REQUEST);
    let local = Flow::new(v4(Ipv4Addr::LOCALHOST, 40004), v4(Ipv4Addr::LOCALHOST, 443));
    assert_eq!(encode(&local, 6), V4_LOCAL_REQUEST);
    let six = Flow::new(v6("2001:db8:1::2", 40006)?, v6("2001:db8:2::5", 443)?);
    assert_eq!(encode(&six, 8), V6_QUICKACK_REQUEST);
    Ok(())
}

#[test]
fn ipv4_mapped_addresses_are_looked_up_as_ipv4() -> R {
    let mapped = Flow::new(
        v6("::ffff:172.17.0.2", 40000)?,
        v6("::ffff:203.0.113.7", 443)?,
    );
    assert_eq!(mapped, plain_flow());
    assert_eq!(encode(&mapped, 1), V4_DEFAULT_PLAIN_REQUEST);
    Ok(())
}

// ---- the reply ------------------------------------------------------------

#[test]
fn a_default_route_with_quickack_and_rto_min() -> R {
    let r = route::decode_reply(&V4_DEFAULT_QUICKACK_REPLY, 3)?;
    assert_eq!(r.dst, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    assert_eq!(r.dst_len, 0);
    assert_eq!(r.gateway, Some(IpAddr::V4(GW)));
    assert_eq!(r.oif, Some(11));
    assert_eq!(r.table, 254);
    assert!(r.quickack());
    assert_eq!(r.metrics.get(RTAX_QUICKACK), Some(1));
    assert_eq!(r.rto_min_ms(), Some(5));
    assert!(r.metrics.locked(RTAX_RTO_MIN), "ip locks rto_min");
    assert!(!r.is_local());
    let check = RouteCheck::Route(Box::new(r));
    assert_eq!(check.quickack(), QuickAck::On);
    assert_eq!(check.rto_min_ms(), Some(5));
    Ok(())
}

#[test]
fn a_default_route_without_quickack() -> R {
    let r = route::decode_reply(&V4_DEFAULT_PLAIN_REPLY, 1)?;
    assert_eq!(r.gateway, Some(IpAddr::V4(GW)));
    assert_eq!(r.protocol, 3, "boot: what `ip route add` sets by default");
    assert!(!r.quickack());
    assert_eq!(r.rto_min_ms(), None);
    assert_eq!(r.priority, None);
    assert_eq!(
        RouteCheck::Route(Box::new(r)).quickack(),
        QuickAck::Off,
        "no RTAX_QUICKACK means delayed ACKs"
    );
    Ok(())
}

#[test]
fn rto_min_without_quickack() -> R {
    let r = route::decode_reply(&V4_SUBNET_RTO_MIN_REPLY, 4)?;
    assert_eq!(r.dst, IpAddr::V4(Ipv4Addr::new(172, 17, 0, 0)));
    assert_eq!(r.dst_len, 16);
    assert_eq!(r.gateway, None);
    assert_eq!(r.scope, 253);
    assert_eq!(r.prefsrc, Some(IpAddr::V4(ME)));
    assert_eq!(r.rto_min_ms(), Some(5));
    assert!(!r.quickack());
    Ok(())
}

#[test]
fn tables_locked_metrics_and_congestion_control() -> R {
    let r = route::decode_reply(&V4_TABLE_LOCKED_REPLY, 5)?;
    assert_eq!(r.table, 100);
    assert_eq!(r.priority, Some(50));
    assert_eq!(r.protocol, 4);
    assert_eq!(r.metrics.get(2), Some(1400), "mtu");
    assert!(r.metrics.locked(2));
    assert_eq!(r.metrics.get(11), Some(10), "initcwnd");
    assert!(!r.metrics.locked(11));
    assert_eq!(r.metrics.cc_algo(), Some("cubic"));
    assert!(!r.quickack(), "quickack 0 is stored as no quickack");
    Ok(())
}

#[test]
fn ipv6_routes() -> R {
    let q = route::decode_reply(&V6_QUICKACK_REPLY, 8)?;
    assert_eq!(q.dst, "2001:db8:2::".parse::<IpAddr>()?);
    assert_eq!(q.dst_len, 48);
    assert_eq!(q.gateway, Some("2001:db8:1::1".parse::<IpAddr>()?));
    assert_eq!(q.oif, Some(12));
    assert_eq!(q.priority, Some(256));
    assert_eq!(q.pref, Some(0));
    assert!(q.quickack());
    assert_eq!(q.rto_min_ms(), Some(5));
    let p = route::decode_reply(&V6_PLAIN_REPLY, 9)?;
    assert_eq!(p.dst, "2001:db8:1::".parse::<IpAddr>()?);
    assert_eq!(p.dst_len, 64);
    assert_eq!(p.protocol, 2);
    assert!(!p.quickack());
    assert_eq!(p.rto_min_ms(), None);
    Ok(())
}

#[test]
fn routes_to_this_host_are_local() -> R {
    let r = route::decode_reply(&V4_LOCAL_REPLY, 6)?;
    assert!(r.is_local());
    assert_eq!(r.table, 255);
    assert_eq!(r.oif, Some(1));
    assert_eq!(r.prefsrc, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    assert!(route::decode_reply(&V6_LOCAL_REPLY, 10)?.is_local());
    assert!(!route::decode_reply(&V4_DEFAULT_PLAIN_REPLY, 1)?.is_local());
    Ok(())
}

#[test]
fn kernel_errors_are_reported_with_their_errno() {
    assert_eq!(
        route::decode_reply(&V4_UNREACHABLE_REPLY, 7),
        Err(RouteError::Kernel(113)),
        "EHOSTUNREACH"
    );
    assert_eq!(
        route::decode_reply(&V6_NO_ROUTE_REPLY, 11),
        Err(RouteError::Kernel(101)),
        "ENETUNREACH"
    );
}

#[test]
fn a_reply_to_another_request_is_rejected() {
    assert!(matches!(
        route::decode_reply(&V4_DEFAULT_PLAIN_REPLY, 2),
        Err(RouteError::Malformed(_))
    ));
}

#[test]
fn every_truncated_reply_is_rejected() {
    for (reply, seq) in REPLIES {
        // Cut short with the header's length unchanged.
        for n in 0..reply.len() {
            assert!(
                matches!(
                    route::decode_reply(&reply[..n], seq),
                    Err(RouteError::Malformed(_))
                ),
                "seq {seq}: {n} of {} bytes accepted",
                reply.len()
            );
        }
        // Cut short with the header's length cut to match: at least the
        // fixed part (header and route message) must be complete.
        for n in 16..28.min(reply.len()) {
            let mut cut = reply[..n].to_vec();
            cut[..4].copy_from_slice(&u32::try_from(n).unwrap_or(0).to_ne_bytes());
            let r = route::decode_reply(&cut, seq);
            assert!(r.is_err(), "seq {seq}: header cut to {n} accepted: {r:?}");
        }
    }
}

/// `reply` with `f` applied.
fn patched(reply: &[u8], f: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut v = reply.to_vec();
    f(&mut v);
    v
}

#[test]
fn malformed_replies_are_rejected() {
    let bad = |what: &str, reply: &[u8], seq: u32| {
        let r = route::decode_reply(reply, seq);
        assert!(matches!(r, Err(RouteError::Malformed(_))), "{what}: {r:?}");
    };
    let p = &V4_DEFAULT_PLAIN_REPLY;
    bad("length below the header", &patched(p, |v| v[0] = 8), 1);
    bad("unknown message type", &patched(p, |v| v[4] = 25), 1);
    bad("unknown family", &patched(p, |v| v[16] = 7), 1);
    bad("prefix longer than 32 bits", &patched(p, |v| v[17] = 33), 1);
    // RTA_GATEWAY (at 36) with a 2-byte address.
    bad("short address", &patched(p, |v| v[36] = 6), 1);
    // An attribute length below its own header.
    bad("attribute length 2", &patched(p, |v| v[36] = 2), 1);
    // An attribute running past the message.
    bad("attribute past the end", &patched(p, |v| v[44] = 12), 1);
    // RTA_OIF (at 44) of 2 bytes.
    bad("short interface index", &patched(p, |v| v[44] = 6), 1);
    let q = &V4_DEFAULT_QUICKACK_REPLY;
    // RTAX_RTO_MIN (at 48, inside RTA_METRICS at 36) of 2 bytes.
    bad("short metric", &patched(q, |v| v[48] = 6), 3);
    // RTAX_QUICKACK (at 56) running past RTA_METRICS.
    bad("metric past its nest", &patched(q, |v| v[56] = 12), 3);
    // An error message whose code is 0: an acknowledgement, not an answer.
    bad(
        "acknowledgement",
        &patched(&V4_UNREACHABLE_REPLY, |v| v[16..20].fill(0)),
        7,
    );
    bad(
        "error without a code",
        &patched(&V4_UNREACHABLE_REPLY, |v| {
            v.truncate(18);
            v[0] = 18;
        }),
        7,
    );
}

// ---- rendering --------------------------------------------------------------

#[test]
fn routes_print_as_iproute2_prints_them() -> R {
    // Each expected line is what `ip route get fibmatch` printed for the
    // same lookup.
    let cases: [(&[u8], u32, &str, &str); 9] = [
        (
            &V4_DEFAULT_PLAIN_REPLY,
            1,
            "eth0",
            "default via 172.17.0.1 dev eth0",
        ),
        (
            &V4_DEFAULT_QUICKACK_REPLY,
            3,
            "eth0",
            "default via 172.17.0.1 dev eth0 rto_min lock 5ms quickack 1",
        ),
        (
            &V4_SUBNET_RTO_MIN_REPLY,
            4,
            "eth0",
            "172.17.0.0/16 dev eth0 proto kernel scope link src 172.17.0.2 rto_min lock 5ms",
        ),
        (
            &V4_TABLE_LOCKED_REPLY,
            5,
            "eth0",
            "198.51.100.0/24 via 172.17.0.1 dev eth0 table 100 proto static metric 50 mtu lock 1400 initcwnd 10 congctl cubic",
        ),
        (
            &V4_LOCAL_REPLY,
            6,
            "lo",
            "local 127.0.0.1 dev lo table local proto kernel scope host src 127.0.0.1",
        ),
        (
            &V4_ONLINK_REPLY,
            12,
            "eth0",
            "100.64.0.0/10 via 10.99.0.1 dev eth0 onlink",
        ),
        (
            &V6_QUICKACK_REPLY,
            8,
            "dum0",
            "2001:db8:2::/48 via 2001:db8:1::1 dev dum0 metric 256 rto_min lock 5ms quickack 1 pref medium",
        ),
        (
            &V6_PLAIN_REPLY,
            9,
            "dum0",
            "2001:db8:1::/64 dev dum0 proto kernel metric 256 pref medium",
        ),
        (
            &V6_LOCAL_REPLY,
            10,
            "lo",
            "local ::1 dev lo table local proto kernel metric 0 pref medium",
        ),
    ];
    for (reply, seq, dev, want) in cases {
        assert_eq!(decoded(reply, seq, dev)?.to_string(), want);
    }
    Ok(())
}

#[test]
fn fix_commands_keep_every_attribute_and_add_quickack() -> R {
    let cases: [(&[u8], u32, &str, &str); 6] = [
        (
            &V4_DEFAULT_PLAIN_REPLY,
            1,
            "eth0",
            "ip route change default via 172.17.0.1 dev eth0 quickack 1",
        ),
        (
            &V4_SUBNET_RTO_MIN_REPLY,
            4,
            "eth0",
            "ip route change 172.17.0.0/16 dev eth0 proto kernel scope link src 172.17.0.2 rto_min 5ms quickack 1",
        ),
        (
            &V4_TABLE_LOCKED_REPLY,
            5,
            "eth0",
            "ip route change 198.51.100.0/24 via 172.17.0.1 dev eth0 table 100 proto static metric 50 mtu lock 1400 initcwnd 10 congctl cubic quickack 1",
        ),
        (
            &V4_ONLINK_REPLY,
            12,
            "eth0",
            "ip route change 100.64.0.0/10 via 10.99.0.1 dev eth0 onlink quickack 1",
        ),
        (
            &V6_PLAIN_REPLY,
            9,
            "dum0",
            "ip -6 route change 2001:db8:1::/64 dev dum0 proto kernel metric 256 pref medium quickack 1",
        ),
        (
            // Already on: the command is the same route with quickack 1 kept
            // once.
            &V4_DEFAULT_QUICKACK_REPLY,
            3,
            "eth0",
            "ip route change default via 172.17.0.1 dev eth0 rto_min 5ms quickack 1",
        ),
    ];
    for (reply, seq, dev, want) in cases {
        assert_eq!(
            decoded(reply, seq, dev)?.fix_command().as_deref(),
            Some(want)
        );
    }
    Ok(())
}

#[test]
fn tables_above_255_and_times_kept_exact() -> R {
    let r = decoded(&V4_RTT_TABLE_REPLY, 15, "eth0")?;
    assert_eq!(r.table, 1000, "from RTA_TABLE, not the 8-bit field");
    assert_eq!(r.priority, Some(300));
    assert_eq!(r.metrics.get(3), Some(65535), "window");
    assert_eq!(r.metrics.get(4), Some(33), "rtt, 1/8 ms");
    assert_eq!(r.metrics.get(5), Some(7), "rttvar, 1/4 ms");
    assert_eq!(r.metrics.get(12), Some(1), "features ecn");
    assert_eq!(
        r.to_string(),
        "192.0.2.0/24 via 172.17.0.1 dev eth0 table 1000 metric 300 window 65535 rtt 4ms rttvar 1ms features ecn"
    );
    // ip shows rtt rounded to whole ms; the command passes the stored value
    // unscaled, which iproute2 takes as is.
    assert_eq!(
        r.fix_command().as_deref(),
        Some(
            "ip route change 192.0.2.0/24 via 172.17.0.1 dev eth0 table 1000 metric 300 window 65535 rtt 33 rttvar 7 features ecn quickack 1"
        )
    );
    Ok(())
}

#[test]
fn routes_one_command_cannot_reproduce_get_none() -> R {
    for (reply, seq) in [(&V4_MULTIPATH_REPLY[..], 13), (&V4_NHID_REPLY[..], 14)] {
        let r = decoded(reply, seq, "eth0")?;
        assert!(r.unrendered.is_some(), "seq {seq}");
        assert_eq!(r.fix_command(), None, "seq {seq}");
        assert!(!r.quickack());
    }
    // Without the interface's name there is no exact command either.
    let r = route::decode_reply(&V4_DEFAULT_PLAIN_REPLY, 1)?;
    assert_eq!(r.fix_command(), None);
    assert_eq!(r.to_string(), "default via 172.17.0.1 dev if11");
    Ok(())
}

// ---- the type of service ----------------------------------------------------

#[test]
fn requests_carry_the_sockets_tos() -> R {
    let four = Flow::new(v4(ME, 40015), v4(Ipv4Addr::new(172, 17, 0, 4), 443)).with_tos(0x10);
    assert_eq!(encode(&four, 16), V4_TOS_REQUEST);
    let six = Flow::new(v6("2001:db8:1::2", 40016)?, v6("2001:db8:8::9", 443)?).with_tos(0x10);
    assert_eq!(encode(&six, 17), V6_TCLASS_RULE_REQUEST);
    // ECN bits are not part of the lookup.
    assert_eq!(Flow::new(v4(ME, 1), v4(ME, 2)).with_tos(0x13).tos, 0x10);
    Ok(())
}

#[test]
fn routes_chosen_by_tos_are_read_and_kept() -> R {
    let r = decoded(&V4_TOS_REPLY, 16, "eth0")?;
    assert_eq!(r.tos, 0x10);
    assert_eq!(r.rto_min_ms(), Some(29));
    assert_eq!(
        r.fix_command().as_deref(),
        Some(
            "ip route change 172.17.0.4 tos 0x10 dev eth0 proto static scope link metric 100 rto_min 29ms quickack 1"
        )
    );
    let rule = decoded(&V6_TCLASS_RULE_REPLY, 17, "dum0")?;
    assert_eq!(rule.table, 600);
    assert_eq!(rule.rto_min_ms(), Some(41));
    assert_eq!(
        rule.fix_command().as_deref(),
        Some(
            "ip -6 route change 2001:db8:8::/48 dev dum0 table 600 proto static metric 1024 rto_min 41ms pref medium quickack 1"
        )
    );
    Ok(())
}

// ---- routes the network configuration owns -----------------------------------

#[test]
fn routes_with_an_expiry_or_from_router_advertisements_get_no_command() -> R {
    for (reply, seq) in [(&V6_EXPIRES_REPLY[..], 18), (&V6_PROTO_RA_REPLY[..], 20)] {
        let r = decoded(reply, seq, "dum0")?;
        assert!(r.unrendered.is_some(), "seq {seq}: {r}");
        assert_eq!(r.fix_command(), None, "seq {seq}: {r}");
        assert!(!r.quickack());
    }
    // The same routes without the expiry, and installed by `ip route`: one
    // command reproduces them.
    let no_expiry = decoded(
        &patched(&V6_EXPIRES_REPLY, |v| v[104..108].fill(0)),
        18,
        "dum0",
    )?;
    assert_eq!(
        no_expiry.fix_command().as_deref(),
        Some(
            "ip -6 route change 2001:db8:5::/48 via 2001:db8:1::1 dev dum0 proto static metric 1024 pref medium quickack 1"
        )
    );
    let static_proto = decoded(&patched(&V6_PROTO_RA_REPLY, |v| v[21] = 4), 20, "dum0")?;
    assert_eq!(
        static_proto.fix_command().as_deref(),
        Some(
            "ip -6 route change 2001:db8:9::/48 via 2001:db8:1::1 dev dum0 proto static metric 1024 pref medium quickack 1"
        )
    );
    Ok(())
}

#[test]
fn source_specific_routes_are_reported_each() -> R {
    let one = decoded(&V6_FROM_REPLY, 19, "dum0")?;
    assert_eq!(
        one.to_string(),
        "2001:db8:6::/48 from 2001:db8:1::/64 dev dum0 proto static metric 1024 pref medium"
    );
    assert_eq!(
        one.fix_command().as_deref(),
        Some(
            "ip -6 route change 2001:db8:6::/48 from 2001:db8:1::/64 dev dum0 proto static metric 1024 pref medium quickack 1"
        )
    );
    // The same destination from another source prefix is another route.
    let other = decoded(&patched(&V6_FROM_REPLY, |v| v[65] = 2), 19, "dum0")?;
    assert_eq!(other.src, Some("2001:db8:2::".parse::<IpAddr>()?));
    let w = Warned::new();
    for r in [one, other] {
        let (out, _, logged) = run(QuickAckCheck::Warn, Ok(r), &w);
        assert!(out.is_ok());
        assert_eq!(logged.len(), 1, "{logged:?}");
    }
    Ok(())
}

// ---- the policy, through an injected lookup ------------------------------

/// Runs the check with `found` as the lookup's answer; returns the outcome,
/// whether the lookup ran and the lines logged.
fn run(
    policy: QuickAckCheck,
    found: Result<Route, RouteError>,
    warned: &Warned,
) -> (Result<RouteCheck, QuickAckError>, bool, Vec<String>) {
    let looked = Cell::new(false);
    let logged = RefCell::new(Vec::new());
    let out = route::check_with(
        policy,
        Ok(plain_flow()),
        |f| {
            looked.set(true);
            assert_eq!(*f, plain_flow());
            found
        },
        warned,
        |line| logged.borrow_mut().push(line),
    );
    (out, looked.get(), logged.into_inner())
}

#[test]
fn off_looks_nothing_up() -> R {
    let w = Warned::new();
    let plain = route::decode_reply(&V4_DEFAULT_PLAIN_REPLY, 1)?;
    let (out, looked, logged) = run(QuickAckCheck::Off, Ok(plain), &w);
    assert_eq!(out, Ok(RouteCheck::NotChecked));
    assert!(!looked);
    assert_eq!(logged.len(), 0, "{logged:?}");
    Ok(())
}

#[test]
fn warn_logs_each_route_without_quickack_once() -> R {
    let w = Warned::new();
    let plain = decoded(&V4_DEFAULT_PLAIN_REPLY, 1, "eth0")?;
    let (out, looked, logged) = run(QuickAckCheck::Warn, Ok(plain.clone()), &w);
    assert!(looked);
    assert_eq!(out, Ok(RouteCheck::Route(Box::new(plain.clone()))));
    assert_eq!(logged.len(), 1, "{logged:?}");
    let line = &logged[0];
    for part in [
        "172.17.0.2:40000",
        "203.0.113.7:443",
        "`default via 172.17.0.1 dev eth0`",
        "`ip route change default via 172.17.0.1 dev eth0 quickack 1`",
        "quickack_check=off",
    ] {
        assert!(line.contains(part), "{part:?} not in {line:?}");
    }
    // The same route again: nothing more.
    for _ in 0..3 {
        let (out, _, logged) = run(QuickAckCheck::Warn, Ok(plain.clone()), &w);
        assert!(out.is_ok());
        assert_eq!(logged.len(), 0, "{logged:?}");
    }
    // Another route is reported once too.
    let subnet = decoded(&V4_SUBNET_RTO_MIN_REPLY, 4, "eth0")?;
    let (_, _, logged) = run(QuickAckCheck::Warn, Ok(subnet.clone()), &w);
    assert_eq!(logged.len(), 1);
    assert!(logged[0].contains("rto_min 5ms"), "{logged:?}");
    let (_, _, logged) = run(QuickAckCheck::Warn, Ok(subnet), &w);
    assert_eq!(logged.len(), 0, "{logged:?}");
    // A different `Warned` (another process, as far as the check knows).
    let (_, _, logged) = run(QuickAckCheck::Warn, Ok(plain), &Warned::new());
    assert_eq!(logged.len(), 1);
    Ok(())
}

#[test]
fn quickack_routes_pass_silently() -> R {
    let on = decoded(&V4_DEFAULT_QUICKACK_REPLY, 3, "eth0")?;
    for policy in [QuickAckCheck::Warn, QuickAckCheck::Require] {
        let (out, looked, logged) = run(policy, Ok(on.clone()), &Warned::new());
        assert!(looked);
        let check = out?;
        assert_eq!(check.quickack(), QuickAck::On);
        assert_eq!(check.rto_min_ms(), Some(5));
        assert_eq!(logged.len(), 0, "{logged:?}");
    }
    Ok(())
}

#[test]
fn require_refuses_a_route_without_quickack() -> R {
    let plain = decoded(&V4_DEFAULT_PLAIN_REPLY, 1, "eth0")?;
    let (out, _, logged) = run(QuickAckCheck::Require, Ok(plain.clone()), &Warned::new());
    assert_eq!(
        logged.len(),
        0,
        "require fails instead of logging: {logged:?}"
    );
    let Err(e) = out else {
        return Err("a route without quickack was accepted".into());
    };
    assert_eq!(
        e,
        QuickAckError::Missing {
            flow: plain_flow(),
            route: Box::new(plain),
        }
    );
    let text = ergon_io::Error::QuickAck(Box::new(e)).to_string();
    for part in [
        "203.0.113.7:443",
        "`default via 172.17.0.1 dev eth0`",
        "`ip route change default via 172.17.0.1 dev eth0 quickack 1`",
        "quickack_check=require",
    ] {
        assert!(text.contains(part), "{part:?} not in {text:?}");
    }
    Ok(())
}

#[test]
fn routes_to_this_host_are_exempt() -> R {
    let local = decoded(&V4_LOCAL_REPLY, 6, "lo")?;
    for policy in [QuickAckCheck::Warn, QuickAckCheck::Require] {
        let (out, _, logged) = run(policy, Ok(local.clone()), &Warned::new());
        let check = out?;
        assert_eq!(check.quickack(), QuickAck::Off, "reported as it is");
        assert!(check.route().is_some_and(Route::is_local));
        assert_eq!(logged.len(), 0, "{logged:?}");
    }
    Ok(())
}

#[test]
fn lookup_failures_warn_once_or_fail() {
    let w = Warned::new();
    let (out, _, logged) = run(QuickAckCheck::Warn, Err(RouteError::Kernel(101)), &w);
    assert_eq!(out, Ok(RouteCheck::Failed(RouteError::Kernel(101))));
    assert_eq!(logged.len(), 1);
    assert!(logged[0].contains("203.0.113.7:443"), "{logged:?}");
    let (_, _, logged) = run(QuickAckCheck::Warn, Err(RouteError::Kernel(101)), &w);
    assert_eq!(logged.len(), 0, "{logged:?}");
    let (_, _, logged) = run(QuickAckCheck::Warn, Err(RouteError::Timeout), &w);
    assert_eq!(logged.len(), 1, "another failure is reported too");
    let (out, _, _) = run(QuickAckCheck::Require, Err(RouteError::Timeout), &w);
    assert_eq!(
        out,
        Err(QuickAckError::Lookup {
            flow: Some(plain_flow()),
            error: RouteError::Timeout,
        })
    );
}

#[test]
fn an_unreadable_socket_is_a_lookup_failure() {
    for policy in [QuickAckCheck::Warn, QuickAckCheck::Require] {
        let looked = Cell::new(false);
        let logged = Cell::new(0);
        let out = route::check_with(
            policy,
            Err(RouteError::Socket(107)),
            |_| {
                looked.set(true);
                Err(RouteError::Timeout)
            },
            &Warned::new(),
            |_| logged.set(logged.get() + 1),
        );
        assert!(!looked.get());
        if policy == QuickAckCheck::Require {
            assert_eq!(
                out,
                Err(QuickAckError::Lookup {
                    flow: None,
                    error: RouteError::Socket(107),
                })
            );
            assert_eq!(logged.get(), 0);
        } else {
            assert_eq!(out, Ok(RouteCheck::Failed(RouteError::Socket(107))));
            assert_eq!(logged.get(), 1);
        }
    }
}

#[test]
fn an_unsupported_platform_never_fails_unless_required() {
    let none = RouteError::Unsupported("route metrics can only be read on Linux");
    let w = Warned::new();
    let (out, _, logged) = run(QuickAckCheck::Warn, Err(none), &w);
    assert_eq!(
        out,
        Ok(RouteCheck::Unsupported(
            "route metrics can only be read on Linux"
        ))
    );
    assert_eq!(
        out.as_ref().map(RouteCheck::quickack),
        Ok(QuickAck::Unknown)
    );
    assert_eq!(logged.len(), 1);
    let (_, _, logged) = run(QuickAckCheck::Warn, Err(none), &w);
    assert_eq!(logged.len(), 0, "{logged:?}");
    let (out, _, _) = run(QuickAckCheck::Require, Err(none), &w);
    assert_eq!(
        out,
        Err(QuickAckError::Lookup {
            flow: Some(plain_flow()),
            error: none,
        }),
        "require cannot be satisfied where quickack cannot be confirmed"
    );
}

#[test]
fn policies_parse_and_print() -> R {
    for p in [
        QuickAckCheck::Off,
        QuickAckCheck::Warn,
        QuickAckCheck::Require,
    ] {
        assert_eq!(p.name().parse::<QuickAckCheck>()?, p);
        assert_eq!(p.to_string(), p.name());
    }
    assert_eq!(QuickAckCheck::default(), QuickAckCheck::Warn);
    assert!("strict".parse::<QuickAckCheck>().is_err());
    Ok(())
}

// ---- the real lookup ----------------------------------------------------

#[cfg(not(target_os = "linux"))]
#[test]
fn off_linux_the_lookup_is_unsupported() {
    let flow = plain_flow();
    assert!(matches!(
        route::lookup(&flow),
        Err(RouteError::Unsupported(_))
    ));
    let out = route::check_with(
        QuickAckCheck::Require,
        Ok(flow),
        route::lookup,
        &Warned::new(),
        drop,
    );
    assert!(matches!(
        out,
        Err(QuickAckError::Lookup {
            error: RouteError::Unsupported(_),
            ..
        })
    ));
}

#[cfg(target_os = "linux")]
mod linux {
    use std::net::{Ipv4Addr, TcpListener, TcpStream};
    use std::os::fd::AsRawFd;

    use ergon_io::route::{self, Flow, QuickAckCheck, RouteCheck};

    use super::R;

    /// The loopback route exists wherever Linux runs, so this lookup goes
    /// to the kernel in every environment.
    #[test]
    fn the_kernel_reports_the_loopback_route() -> R {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let sock = TcpStream::connect(listener.local_addr()?)?;
        let flow = Flow::of_socket(sock.as_raw_fd())?;
        assert_eq!(flow.local, sock.local_addr()?);
        assert_eq!(flow.peer, listener.local_addr()?);
        assert_eq!((flow.oif, flow.mark), (0, 0));
        let r = route::lookup(&flow)?;
        assert!(r.is_local(), "{r}");
        // No table assertion: without policy rules the kernel keeps the local
        // table merged into main and reports main (254); with rules, local.
        assert!(matches!(r.table, 254 | 255), "{r}");
        assert_eq!(r.dev.as_deref(), Some("lo"), "{r}");
        assert_eq!(r.prefsrc, Some(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST)));
        let check = route::check_socket(QuickAckCheck::Require, sock.as_raw_fd())?;
        assert!(
            check.route().is_some_and(route::Route::is_local),
            "{check:?}"
        );
        assert_eq!(
            route::check_socket(QuickAckCheck::Off, sock.as_raw_fd())?,
            RouteCheck::NotChecked
        );
        Ok(())
    }

    /// The socket's type of service (IPv4) or traffic class (IPv6) selects
    /// routes and rules too, so the flow carries it.
    #[test]
    fn the_flow_carries_the_sockets_tos() -> R {
        use socket2::{Domain, Socket, Type};
        use std::net::{Ipv6Addr, SocketAddr};
        let four = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let s = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        s.set_tos_v4(0x10)?;
        s.connect(&four.local_addr()?.into())?;
        assert_eq!(Flow::of_socket(s.as_raw_fd())?.tos, 0x10);
        let six = TcpListener::bind((Ipv6Addr::LOCALHOST, 0))?;
        let s = Socket::new(Domain::IPV6, Type::STREAM, None)?;
        s.set_tclass_v6(0x20)?;
        s.connect(&six.local_addr()?.into())?;
        assert_eq!(Flow::of_socket(s.as_raw_fd())?.tos, 0x20);
        // IPv4 over an IPv6 socket is routed by its IPv4 TOS.
        let s = Socket::new(Domain::IPV6, Type::STREAM, None)?;
        s.set_tclass_v6(0x20)?;
        s.set_tos_v4(0x08)?;
        let mapped = SocketAddr::new(
            Ipv4Addr::LOCALHOST.to_ipv6_mapped().into(),
            four.local_addr()?.port(),
        );
        s.connect(&mapped.into())?;
        let flow = Flow::of_socket(s.as_raw_fd())?;
        assert!(flow.peer.is_ipv4(), "{flow:?}");
        assert_eq!(flow.tos, 0x08);
        Ok(())
    }

    /// A netlink request the kernel does not answer (no `NLM_F_REQUEST`)
    /// times out, even with a zero timeout (a zero `SO_RCVTIMEO` would wait
    /// forever).
    #[test]
    fn an_unanswered_exchange_times_out() -> R {
        use std::time::Duration;
        for timeout in [
            Duration::ZERO,
            Duration::from_nanos(500),
            Duration::from_millis(20),
        ] {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let mut req = [0u8; 16];
                req[0] = 16;
                req[4] = 26; // RTM_GETROUTE, flags 0: never answered
                let mut reply = [0u8; 64];
                let _ = tx.send(ergon_io::sys::rtnetlink_exchange(&req, &mut reply, timeout));
            });
            let out = rx
                .recv_timeout(Duration::from_secs(2))
                .map_err(|_| format!("{timeout:?}: still waiting after 2 s"))?;
            assert!(
                matches!(&out, Err(e) if e.kind() == std::io::ErrorKind::TimedOut),
                "{timeout:?}: {out:?}"
            );
        }
        Ok(())
    }

    /// A socket that is not connected has no route to look up.
    #[test]
    fn an_unconnected_socket_fails_the_lookup() -> R {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let out = route::check_socket(QuickAckCheck::Require, listener.as_raw_fd());
        assert!(
            matches!(out, Err(route::QuickAckError::Lookup { flow: None, .. })),
            "{out:?}"
        );
        Ok(())
    }
}

// ---- every connection a driver installs --------------------------------

#[cfg(target_os = "linux")]
mod support;

#[cfg(target_os = "linux")]
mod driver {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use crate::support::{Lab, R};
    use ergon_io::route::{QuickAck, QuickAckCheck, RouteCheck};
    use ergon_io::{
        AnyConn, Backend, ClientHandler, Config, ConnId, Driver, Error, Handler, HttpClient,
        HttpClientConfig, HttpConn, HttpDial, HttpHandler, HttpImpl, HttpOptions, Response, RxMeta,
    };

    #[derive(Default)]
    struct Opened {
        opened: Vec<ConnId>,
        closed: Vec<(ConnId, String)>,
    }

    impl Handler for Opened {
        fn on_open(&mut self, id: ConnId) {
            self.opened.push(id);
        }
        fn on_closed(&mut self, id: ConnId, error: Error) {
            self.closed.push((id, error.to_string()));
        }
    }

    impl HttpHandler for Opened {
        fn on_response(&mut self, _: ConnId, _: u64, _: &Response<'_>, _: &RxMeta) {}
    }

    #[derive(Clone, Copy, Debug)]
    enum How {
        Connect,
        Background,
        Add,
    }

    /// Opens one connection to a loopback server and returns what the
    /// driver's check recorded on it.
    fn route_of(policy: QuickAckCheck, how: How) -> R<RouteCheck> {
        let lab = Lab::new()?;
        let mut cfg = Config::new(Backend::EpollSpin);
        cfg.quickack_check = policy;
        // A pod with one CPU has no core for the connect helper: opt in to
        // connecting on the loop there (the route is recorded the same way).
        cfg.inline_connect_fallback = true;
        let mut d: Driver<HttpConn> = Driver::new(cfg)?;
        std::thread::scope(|s| -> R<RouteCheck> {
            let server = s.spawn(|| {
                lab.accept_tls_within(Duration::from_secs(10))
                    .map_err(|e| e.to_string())
            });
            let dial = HttpDial::new(
                lab.target("/")?,
                lab.client_cfg.clone(),
                HttpOptions::default(),
            )?;
            let id = match how {
                How::Connect => d.connect(dial)?,
                How::Background => d.connect_background(dial)?,
                How::Add => d.add(dial.connect_blocking()?)?,
            };
            let mut h = Opened::default();
            let end = Instant::now() + Duration::from_secs(10);
            while !matches!(how, How::Add) && h.opened.is_empty() && h.closed.is_empty() {
                d.poll(&mut h)?;
                if Instant::now() > end {
                    return Err("connect timed out".into());
                }
            }
            let _peer = server.join().map_err(|_| "server panicked")??;
            if let Some((_, e)) = h.closed.first() {
                return Err(format!("{how:?} closed: {e}").into());
            }
            // Under `warn` the lookup runs on ergon-io's warning thread and
            // the check reads `Pending` until it has.
            if !ergon_io::warnings::flush(Duration::from_secs(10)) {
                return Err("the warning thread did not catch up".into());
            }
            Ok(d.get(id).ok_or("not open")?.route().clone())
        })
    }

    #[test]
    fn every_way_in_records_the_route() -> R {
        for how in [How::Connect, How::Background, How::Add] {
            for policy in [QuickAckCheck::Warn, QuickAckCheck::Require] {
                let check = route_of(policy, how)?;
                let r = check
                    .route()
                    .ok_or_else(|| format!("{how:?} {policy}: {check:?}"))?;
                assert!(r.is_local(), "{how:?} {policy}: {r}");
                assert_eq!(r.dev.as_deref(), Some("lo"));
                // Loopback carries no quickack here, and is let through.
                assert_eq!(check.quickack(), QuickAck::Off);
            }
            assert_eq!(route_of(QuickAckCheck::Off, how)?, RouteCheck::NotChecked);
        }
        Ok(())
    }

    #[derive(Default)]
    struct Replies {
        ok: usize,
        errors: Vec<String>,
    }

    impl ClientHandler for Replies {
        fn on_response(&mut self, _: u64, _: &Response<'_>) {
            self.ok += 1;
        }
        fn on_error(&mut self, _: u64, _: ergon_io::Sent, error: Error) {
            self.errors.push(error.to_string());
        }
    }

    /// One request through `imp` under `policy`; returns the pool's checks.
    fn pool_checks(imp: HttpImpl, policy: QuickAckCheck) -> R<Vec<RouteCheck>> {
        let lab = Lab::new()?;
        let mut cfg = HttpClientConfig::new(imp);
        cfg.pool = 2;
        cfg.driver.quickack_check = policy;
        // A pod with one CPU has no core for the connect helper: opt in to
        // opening on the loop there (one route check per connection either
        // way; the refusal without this is driver_checks's `background_
        // refuses`).
        cfg.driver.inline_connect_fallback = true;
        cfg.curl_ca_file = Some(lab.ca_file()?.display().to_string());
        let stop = AtomicBool::new(false);
        std::thread::scope(|s| -> R<Vec<RouteCheck>> {
            let srv = s.spawn(|| lab.serve_http(&stop, None).map_err(|e| e.to_string()));
            let result = (|| -> R<Vec<RouteCheck>> {
                let mut c = HttpClient::new(&cfg, lab.target("")?, lab.client_cfg.clone())?;
                let mut h = Replies::default();
                let end = Instant::now() + Duration::from_secs(10);
                while c.idle() == 0 {
                    c.poll(&mut h)?;
                    if Instant::now() > end {
                        return Err("no connection opened".into());
                    }
                }
                c.request("GET", "/q", &[], &[])?;
                while h.ok == 0 && h.errors.is_empty() {
                    c.poll(&mut h)?;
                    if Instant::now() > end {
                        return Err("no response".into());
                    }
                }
                if !h.errors.is_empty() {
                    return Err(format!("{imp:?} {policy}: {:?}", h.errors).into());
                }
                Ok(c.route_checks())
            })();
            stop.store(true, Ordering::Relaxed);
            srv.join().map_err(|_| "server panicked")??;
            result
        })
    }

    #[test]
    fn the_pool_reports_each_connections_route() -> R {
        let mut imps = vec![HttpImpl::Native];
        if cfg!(feature = "curl") {
            imps.push(HttpImpl::Curl);
        }
        for imp in imps {
            for policy in [QuickAckCheck::Warn, QuickAckCheck::Require] {
                let checks = pool_checks(imp, policy)?;
                assert!(!checks.is_empty(), "{imp:?} {policy}");
                for c in &checks {
                    let r = c
                        .route()
                        .ok_or_else(|| format!("{imp:?} {policy}: {c:?}"))?;
                    assert!(r.is_local(), "{imp:?} {policy}: {r}");
                    assert_eq!(r.dev.as_deref(), Some("lo"));
                }
            }
            let off = pool_checks(imp, QuickAckCheck::Off)?;
            assert!(
                off.iter().all(|c| *c == RouteCheck::NotChecked),
                "{imp:?}: {off:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn the_default_policy_is_warn() {
        assert_eq!(
            Config::new(Backend::RecvBusy).quickack_check,
            QuickAckCheck::Warn
        );
        let _: fn(&AnyConn) -> &RouteCheck = AnyConn::route;
    }
}
