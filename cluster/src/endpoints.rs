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
    if map.trim().is_empty() {
        return Err(ClusterError::connect("ingress_endpoints is empty"));
    }
    let mut out = Vec::new();
    for part in map.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(ClusterError::connect(
                "ingress_endpoints empty entry (leading, trailing, or repeated comma)",
            ));
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
    out.sort_by_key(|e| e.member_id);
    // Detect duplicate member IDs after sorting.
    for w in out.windows(2) {
        if w[0].member_id == w[1].member_id {
            return Err(ClusterError::connect(format!(
                "ingress_endpoints duplicate member id {} (endpoints {} and {})",
                w[0].member_id, w[0].endpoint, w[1].endpoint,
            )));
        }
    }
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

    #[test]
    fn rejects_duplicate_member_ids_before_sort() -> Result<(), Box<dyn std::error::Error>> {
        let err = parse_ingress_endpoints("0=a:1,1=b:2,0=c:3").expect_err("duplicate id 0");
        let s = err.to_string();
        assert!(s.contains("duplicate member id 0"), "{s}");
        assert!(s.contains("a:1") || s.contains("c:3"), "{s}");
        Ok(())
    }

    #[test]
    fn rejects_duplicate_member_ids_after_sort_order() -> Result<(), Box<dyn std::error::Error>> {
        // Already sorted input with a later duplicate.
        let err = parse_ingress_endpoints("0=host:1,1=host:2,1=host:3").expect_err("dup 1");
        let s = err.to_string();
        assert!(s.contains("duplicate member id 1"), "{s}");
        Ok(())
    }

    #[test]
    fn rejects_duplicate_identical_endpoints() -> Result<(), Box<dyn std::error::Error>> {
        let err = parse_ingress_endpoints("2=x:9,2=x:9").expect_err("identical dup");
        assert!(err.to_string().contains("duplicate member id 2"), "{err}");
        Ok(())
    }

    #[test]
    fn rejects_duplicate_with_whitespace() -> Result<(), Box<dyn std::error::Error>> {
        let err = parse_ingress_endpoints(" 0 = a:1 , 0 = b:2 ").expect_err("ws dup");
        assert!(err.to_string().contains("duplicate member id 0"), "{err}");
        Ok(())
    }

    #[test]
    fn accepts_non_duplicate_control() -> Result<(), Box<dyn std::error::Error>> {
        let v = parse_ingress_endpoints("0=a:1,1=b:2,2=c:3")?;
        assert_eq!(v.len(), 3);
        assert_eq!(v[2].member_id, 2);
        Ok(())
    }

    #[test]
    fn rejects_empty_comma_separated_entries() -> Result<(), Box<dyn std::error::Error>> {
        for bad in ["0=a:1,,1=b:2", ",0=a:1", "0=a:1,", "0=a:1, ,1=b:2"] {
            let err = parse_ingress_endpoints(bad).expect_err(bad);
            let s = err.to_string();
            assert!(
                s.contains("empty") || s.contains("empty entry"),
                "expected empty-entry error for {bad:?}, got {s}"
            );
        }
        Ok(())
    }
}
