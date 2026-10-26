use super::*;

#[test]
fn ipv4_policy_preserves_every_network_boundary_and_mapped_equivalent() -> Result<()> {
    let reference = [
        "0.0.0.0/8",
        "10.0.0.0/8",
        "100.64.0.0/10",
        "127.0.0.0/8",
        "169.254.0.0/16",
        "172.16.0.0/12",
        "192.0.0.0/24",
        "192.0.2.0/24",
        "192.88.99.0/24",
        "192.168.0.0/16",
        "198.18.0.0/15",
        "198.51.100.0/24",
        "203.0.113.0/24",
        "224.0.0.0/4",
        "240.0.0.0/4",
    ]
    .into_iter()
    .map(str::parse::<Ipv4Net>)
    .collect::<std::result::Result<Vec<_>, _>>()?;
    for network in &reference {
        for boundary in [u32::from(network.network()), u32::from(network.broadcast())] {
            for bits in [
                boundary.saturating_sub(1),
                boundary,
                boundary.saturating_add(1),
            ] {
                let address = Ipv4Addr::from(bits);
                let expected = !reference.iter().any(|network| network.contains(&address));
                assert_eq!(is_public_ip(IpAddr::V4(address)), expected, "{address}");
                assert_eq!(
                    is_public_ip(IpAddr::V6(address.to_ipv6_mapped())),
                    expected,
                    "mapped {address}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn ipv6_policy_preserves_denied_and_public_envelope_boundaries() -> Result<()> {
    let reference = ["2001::/23", "2001:db8::/32", "2002::/16", "3fff::/20"]
        .into_iter()
        .map(str::parse::<Ipv6Net>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let public_envelope: Ipv6Net = "2000::/3".parse()?;
    for network in reference.iter().chain(std::iter::once(&public_envelope)) {
        for boundary in [
            u128::from(network.network()),
            u128::from(network.broadcast()),
        ] {
            for bits in [
                boundary.saturating_sub(1),
                boundary,
                boundary.saturating_add(1),
            ] {
                let address = Ipv6Addr::from(bits);
                let expected = public_envelope.contains(&address)
                    && !reference.iter().any(|network| network.contains(&address));
                assert_eq!(is_public_ip(IpAddr::V6(address)), expected, "{address}");
            }
        }
    }
    Ok(())
}

#[test]
fn rejects_urls_that_expand_beyond_the_limit_during_encoding() {
    let value = format!("https://example.com/{}", "é".repeat(1000));
    assert!(value.len() < MAX_URL_LENGTH);
    assert!(normalize_url(&value).is_err());
}

#[test]
fn normalizes_equivalent_urls_without_erasing_queries() -> Result<()> {
    assert_eq!(
        normalize_url("https://EXAMPLE.com:443/a/../b?q=1#fragment")?.as_str(),
        "https://example.com/b?q=1"
    );
    assert!(normalize_url("https://example.com@attacker.invalid/").is_err());
    Ok(())
}

#[test]
fn rejects_empty_or_mixed_dns_sets() -> Result<()> {
    let public = "1.1.1.1:0".parse()?;
    let private = "127.0.0.1:0".parse()?;
    assert!(checked_addresses(Vec::new(), false).is_err());
    assert!(checked_addresses(Vec::new(), true).is_err());
    assert!(checked_addresses(vec![public, private], false).is_err());
    assert_eq!(checked_addresses(vec![public], false)?, vec![public]);
    assert_eq!(checked_addresses(vec![private], true)?, vec![private]);
    Ok(())
}

#[test]
fn normalization_removes_fragments_and_default_ports() -> Result<()> {
    assert_eq!(
        normalize_url("HTTPS://Example.COM:443/a#b")?.as_str(),
        "https://example.com/a"
    );
    Ok(())
}

#[test]
fn rejects_non_http_credentials_and_long_urls() {
    for value in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "https://user:pass@example.com",
    ] {
        assert!(normalize_url(value).is_err());
    }
    assert!(
        normalize_url(&format!(
            "https://example.com/{}",
            "a".repeat(MAX_URL_LENGTH)
        ))
        .is_err()
    );
}

#[test]
fn blocks_special_address_ranges_and_mapped_ipv4() -> Result<()> {
    for address in [
        "0.0.0.0",
        "127.0.0.1",
        "10.1.1.1",
        "100.64.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.168.1.1",
        "192.0.2.1",
        "198.18.0.1",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "fc00::1",
        "fe80::1",
        "ff02::1",
        "::ffff:127.0.0.1",
        "2002:7f00:1::",
        "2001:db8::1",
        "3fff::1",
    ] {
        assert!(!is_public_ip(address.parse()?), "{address}");
    }
    for address in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
        assert!(is_public_ip(address.parse()?), "{address}");
    }
    Ok(())
}

#[test]
fn private_access_requires_explicit_opt_in() -> Result<()> {
    let url = normalize_url("http://127.0.0.1/")?;
    assert!(validate_destination(&url, false).is_err());
    assert!(validate_destination(&url, true).is_ok());
    assert!(validate_destination(&normalize_url("http://localhost/")?, false).is_err());
    Ok(())
}
