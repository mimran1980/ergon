//! Multi-member ingress endpoint maps (`"0=host:port,1=host:port"`).
//!
//! Used for first-connect when [`crate::SessionBuilder::ingress_endpoints`] is
//! set, and for leader resolution after redirect / `NewLeaderEvent`.

use crate::ClusterError;

/// One `(member_id, host:port)` entry from a Java-style endpoints map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressEndpoint {
    /// Cluster member id (from the endpoints map key).
    pub member_id: i32,
    /// `host:port` (no `aeron:` prefix).
    pub endpoint: String,
}

/// Parse `"0=host:9002,1=host:9003"` into ordered entries.
pub fn parse_ingress_endpoints(map: &str) -> Result<Vec<IngressEndpoint>, ClusterError> {
    let mut out = Vec::new();
    for part in map.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (id_s, ep) = part
            .split_once('=')
            .ok_or_else(|| ClusterError::connect(format!("ingress_endpoints entry missing '=': {part}")))?;
        let member_id: i32 = id_s
            .trim()
            .parse()
            .map_err(|_| ClusterError::connect(format!("ingress_endpoints bad member id: {id_s}")))?;
        let endpoint = ep.trim().to_string();
        if endpoint.is_empty() {
            return Err(ClusterError::connect(format!(
                "ingress_endpoints empty endpoint for member {member_id}"
            )));
        }
        out.push(IngressEndpoint { member_id, endpoint });
    }
    if out.is_empty() {
        return Err(ClusterError::connect("ingress_endpoints is empty"));
    }
    out.sort_by_key(|e| e.member_id);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sorted_by_id() -> Result<(), Box<dyn std::error::Error>> {
        let v = parse_ingress_endpoints("1=b:2,0=a:1")?;
        assert_eq!(v[0].member_id, 0);
        assert_eq!(v[0].endpoint, "a:1");
        assert_eq!(v[1].member_id, 1);
        Ok(())
    }
}
