use std::net::IpAddr;

/// Validate that a hostname is safe for use in outbound HTTP requests.
///
/// Rejects hostnames that resolve to private/loopback/link-local addresses
/// or that look like internal network names, to prevent SSRF attacks.
pub fn validate_public_host(host: &str) -> Result<(), String> {
    let host = host.trim().to_lowercase();

    if host.is_empty() {
        return Err("Host cannot be empty".to_string());
    }

    // Strip port if present
    let hostname = host.split(':').next().unwrap_or(&host);

    // Block obviously private hostnames
    if hostname == "localhost"
        || hostname.ends_with(".local")
        || hostname.ends_with(".internal")
        || hostname.ends_with(".localdomain")
        || hostname == "metadata.google.internal"
        || hostname == "169.254.169.254"
    {
        return Err(format!("Host '{}' is not allowed (private/internal)", host));
    }

    // Block raw IP addresses in private ranges
    if let Ok(ip) = hostname.parse::<IpAddr>() {
        if !is_global_ip(ip) {
            return Err(format!(
                "Host '{}' resolves to a private/reserved IP address",
                host
            ));
        }
    }

    // Block hosts with no dots (likely internal short names) unless it's an IP
    if !hostname.contains('.') {
        return Err(format!(
            "Host '{}' does not appear to be a valid public hostname",
            host
        ));
    }

    Ok(())
}

fn is_global_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !v4.is_loopback()
                && !v4.is_private()
                && !v4.is_link_local()
                && !v4.is_broadcast()
                && !v4.is_unspecified()
                && !v4.is_documentation()
                // 100.64.0.0/10 (CGNAT)
                && !(v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64)
                // 169.254.0.0/16 (link-local, cloud metadata)
                && !(v4.octets()[0] == 169 && v4.octets()[1] == 254)
        }
        IpAddr::V6(v6) => !v6.is_loopback() && !v6.is_unspecified(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_public_hosts() {
        assert!(validate_public_host("mastodon.social").is_ok());
        assert!(validate_public_host("bsky.social").is_ok());
        assert!(validate_public_host("example.com").is_ok());
        assert!(validate_public_host("sub.domain.example.com").is_ok());
    }

    #[test]
    fn test_rejects_localhost() {
        assert!(validate_public_host("localhost").is_err());
        assert!(validate_public_host("something.local").is_err());
        assert!(validate_public_host("something.internal").is_err());
    }

    #[test]
    fn test_rejects_private_ips() {
        assert!(validate_public_host("127.0.0.1").is_err());
        assert!(validate_public_host("10.0.0.1").is_err());
        assert!(validate_public_host("192.168.1.1").is_err());
        assert!(validate_public_host("172.16.0.1").is_err());
        assert!(validate_public_host("169.254.169.254").is_err());
    }

    #[test]
    fn test_rejects_cloud_metadata() {
        assert!(validate_public_host("metadata.google.internal").is_err());
    }

    #[test]
    fn test_rejects_no_dots() {
        assert!(validate_public_host("intranet").is_err());
        assert!(validate_public_host("db-server").is_err());
    }

    #[test]
    fn test_allows_public_ips() {
        assert!(validate_public_host("8.8.8.8").is_ok());
        assert!(validate_public_host("1.1.1.1").is_ok());
    }

    #[test]
    fn test_rejects_empty() {
        assert!(validate_public_host("").is_err());
        assert!(validate_public_host("  ").is_err());
    }
}
