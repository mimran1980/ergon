//! What ergon-io's warning thread does for each connection a driver takes
//! (see [`crate::checks`]): the route lookup under `quickack_check=warn`
//! (I2), then whether a NIC-polling backend's socket has a NAPI id (I4) and
//! the checks of the route's egress device (I-dev). Nothing here runs on
//! the polling thread, which only packs a [`ConnJob`] and queues it.

use std::sync::{Arc, OnceLock};

use crate::backend::Backend;
use crate::checks::{self, DevContext};
use crate::endpoint::ConnId;
use crate::route::{self, Flow, QuickAckCheck, RouteCheck, RouteError};
use crate::warn::State;

/// A connection's route, as the warning thread gets it.
pub(super) enum RouteIn {
    /// To look up (`warn`): the connection's addressing, and the cell its
    /// [`RouteCheck`] is read from.
    Lookup(Result<Flow, RouteError>, Arc<OnceLock<RouteCheck>>),
    /// Already looked up where the connection was set up.
    Known(RouteCheck),
}

/// One connection's checks.
pub(super) struct ConnJob {
    pub(super) conn: ConnId,
    pub(super) backend: Backend,
    pub(super) ctx: Arc<DevContext>,
    /// `SO_INCOMING_NAPI_ID` after the handshake (NIC-polling backends).
    pub(super) napi: Option<u32>,
    pub(super) route: RouteIn,
}

impl ConnJob {
    /// Runs on the warning thread.
    pub(super) fn run(self, st: &mut State) {
        let (check, flow) = match self.route {
            RouteIn::Lookup(flow, cell) => {
                let addressing = flow.as_ref().ok().copied();
                let mut line = None;
                // `warn` never refuses a connection, so there is no error
                // to carry; the outcome (a failed lookup included) is Ok.
                let check = route::check_with(
                    QuickAckCheck::Warn,
                    flow,
                    route::lookup,
                    &route::WARNED,
                    |l| line = Some(l),
                )
                .unwrap_or(RouteCheck::NotChecked);
                let _ = cell.set(check.clone());
                if let Some(l) = line {
                    st.print(l);
                }
                (check, addressing)
            }
            RouteIn::Known(check) => (check, None),
        };
        let Some(r) = check.route() else {
            return;
        };
        if r.is_local() {
            return;
        }
        let dev = r.dev.clone().unwrap_or_default();
        if let Some(l) = checks::napi_line(self.backend, self.conn, flow.as_ref(), r, self.napi)
            && st.first(format!("I4 {dev}"))
        {
            st.print(l);
        }
        if !dev.is_empty() {
            st.check_device(&dev, &self.ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use ergon_tune::{Host, HotSet, Role};

    use super::{ConnJob, RouteIn};
    use crate::backend::Backend;
    use crate::checks::{DevContext, SETTLE_READS};
    use crate::endpoint::ConnId;
    use crate::route::{self, RouteCheck};
    use crate::warn::{self, State};

    type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

    /// `default via 172.17.0.1 dev eth0` as the kernel answered it (Linux
    /// 6.12; tests/route.rs checks it against `ip route get fibmatch`).
    const V4_DEFAULT_REPLY: [u8; 52] = [
        0x34, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x33, 0x00, 0x00,
        0x00, 0x02, 0x00, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00,
        0x0f, 0x00, 0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0xac, 0x11, 0x00, 0x01, 0x08,
        0x00, 0x04, 0x00, 0x0b, 0x00, 0x00, 0x00,
    ];

    fn write(root: &Path, abs: &str, text: &str) -> R {
        let p = root.join(abs.trim_start_matches('/'));
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, text)?;
        Ok(())
    }

    fn link(root: &Path, abs: &str, target: &str) -> R {
        let p = root.join(abs.trim_start_matches('/'));
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::os::unix::fs::symlink(target, p)?;
        Ok(())
    }

    /// A host whose `ens5` (PCI 0000:00:05.0) has IRQ 61 on CPU 2, the
    /// loop's: T2 warns about it.
    fn host_with_irq_on_the_loop(root: &Path) -> R {
        write(root, "/sys/devices/system/cpu/online", "0-7\n")?;
        write(root, "/sys/devices/system/cpu/isolated", "1-3,5-7\n")?;
        std::fs::create_dir_all(root.join("sys/bus/pci/drivers"))?;
        let d = "/sys/bus/pci/devices/0000:00:05.0";
        link(root, &format!("{d}/subsystem"), "../../../../bus/pci")?;
        write(root, &format!("{d}/msi_irqs/61"), "msix\n")?;
        write(root, "/proc/irq/61/smp_affinity_list", "2\n")?;
        write(root, "/sys/class/net/ens5/carrier_changes", "2\n")?;
        link(
            root,
            "/sys/class/net/ens5/device",
            "../../../bus/pci/devices/0000:00:05.0",
        )?;
        Ok(())
    }

    fn job(route: &route::Route) -> ConnJob {
        ConnJob {
            conn: ConnId::default(),
            backend: Backend::RecvBusy,
            ctx: Arc::new(DevContext {
                hot: HotSet::new().with(2, Role::Loop),
                interrupt_driven: false,
            }),
            napi: Some(8193),
            route: RouteIn::Known(RouteCheck::Route(Box::new(route.clone()))),
        }
    }

    /// The interface resets and the application reconnects at once, before
    /// the warning thread's next read of `carrier_changes`: the reset's
    /// findings are still written again.
    #[test]
    fn a_reset_is_reported_again_when_the_application_reconnects_at_once() -> R {
        let root: PathBuf =
            std::env::temp_dir().join(format!("ergon-io-conn-checks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        host_with_irq_on_the_loop(&root)?;
        let mut r = route::decode_reply(&V4_DEFAULT_REPLY, 1)?;
        r.dev = Some("ens5".to_owned());
        warn::keep_lines();
        let mut st = State::at(Host::at(&root));
        job(&r).run(&mut st);
        write(&root, "/sys/class/net/ens5/carrier_changes", "4\n")?;
        job(&r).run(&mut st);
        for _ in 0..=SETTLE_READS {
            st.recheck_now();
        }
        let _ = std::fs::remove_dir_all(&root);
        let lines = warn::take_lines();
        let t2 = lines
            .iter()
            .filter(|l| l.contains("T2 warn") && l.contains("IRQ 61"))
            .count();
        assert_eq!(t2, 2, "once at first, once after the reset: {lines:#?}");
        Ok(())
    }
}
