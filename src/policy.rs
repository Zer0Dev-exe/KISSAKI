use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use anyhow::{Result, bail};
use ipnet::IpNet;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use url::{Host, Url};

pub const MAX_URL_LENGTH: usize = 4096;

pub fn normalize_url(value: &str) -> Result<Url> {
    if value.len() > MAX_URL_LENGTH {
        bail!("URL exceeds {MAX_URL_LENGTH} bytes");
    }
    let mut url = Url::parse(value)?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        bail!("only absolute HTTP(S) URLs are supported");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("credentials in URLs are not supported; use a scoped header instead");
    }
    url.set_fragment(None);
    Ok(url)
}

pub fn is_public_ip(address: IpAddr) -> bool {
    let address = match address {
        IpAddr::V6(address) => match address.to_ipv4_mapped() {
            Some(mapped) => IpAddr::V4(mapped),
            None => IpAddr::V6(address),
        },
        address => address,
    };
    let denied = match address {
        IpAddr::V4(_) => &[
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
        ][..],
        IpAddr::V6(address) => {
            if address.segments()[0] & 0xe000 != 0x2000 {
                return false;
            }
            &["2001::/23", "2001:db8::/32", "2002::/16", "3fff::/20"][..]
        }
    };
    !denied
        .iter()
        .filter_map(|network| network.parse::<IpNet>().ok())
        .any(|network| network.contains(&address))
}

pub fn validate_destination(url: &Url, allow_private: bool) -> Result<()> {
    if allow_private {
        return Ok(());
    }
    match url.host() {
        Some(Host::Ipv4(address)) if !is_public_ip(IpAddr::V4(address)) => {
            bail!("non-public IP destination blocked")
        }
        Some(Host::Ipv6(address)) if !is_public_ip(IpAddr::V6(address)) => {
            bail!("non-public IP destination blocked")
        }
        Some(Host::Domain(domain)) if domain == "localhost" || domain.ends_with(".localhost") => {
            bail!("localhost destination blocked")
        }
        _ => Ok(()),
    }
}

#[derive(Debug)]
pub struct SafeResolver {
    pub allow_private: bool,
}

fn checked_addresses(
    addresses: Vec<SocketAddr>,
    allow_private: bool,
) -> io::Result<Vec<SocketAddr>> {
    if addresses.is_empty()
        || (!allow_private && addresses.iter().any(|entry| !is_public_ip(entry.ip())))
    {
        return Err(io::Error::other(
            "DNS returned a blocked or empty address set",
        ));
    }
    Ok(addresses)
}

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let allow_private = self.allow_private;
        Box::pin(async move {
            let addresses: Vec<_> = tokio::net::lookup_host((name.as_str(), 0)).await?.collect();
            let addresses = checked_addresses(addresses, allow_private)?;
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

pub fn resolver(allow_private: bool) -> Arc<SafeResolver> {
    Arc::new(SafeResolver { allow_private })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
