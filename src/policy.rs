use std::{
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{Result, bail};
use ipnet::{Ipv4Net, Ipv6Net};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use url::{Host, Url};

pub const MAX_URL_LENGTH: usize = 4096;

const DENIED_IPV4: &[Ipv4Net] = &[
    Ipv4Net::new_assert(Ipv4Addr::new(0, 0, 0, 0), 8),
    Ipv4Net::new_assert(Ipv4Addr::new(10, 0, 0, 0), 8),
    Ipv4Net::new_assert(Ipv4Addr::new(100, 64, 0, 0), 10),
    Ipv4Net::new_assert(Ipv4Addr::new(127, 0, 0, 0), 8),
    Ipv4Net::new_assert(Ipv4Addr::new(169, 254, 0, 0), 16),
    Ipv4Net::new_assert(Ipv4Addr::new(172, 16, 0, 0), 12),
    Ipv4Net::new_assert(Ipv4Addr::new(192, 0, 0, 0), 24),
    Ipv4Net::new_assert(Ipv4Addr::new(192, 0, 2, 0), 24),
    Ipv4Net::new_assert(Ipv4Addr::new(192, 88, 99, 0), 24),
    Ipv4Net::new_assert(Ipv4Addr::new(192, 168, 0, 0), 16),
    Ipv4Net::new_assert(Ipv4Addr::new(198, 18, 0, 0), 15),
    Ipv4Net::new_assert(Ipv4Addr::new(198, 51, 100, 0), 24),
    Ipv4Net::new_assert(Ipv4Addr::new(203, 0, 113, 0), 24),
    Ipv4Net::new_assert(Ipv4Addr::new(224, 0, 0, 0), 4),
    Ipv4Net::new_assert(Ipv4Addr::new(240, 0, 0, 0), 4),
];

const DENIED_IPV6: &[Ipv6Net] = &[
    Ipv6Net::new_assert(Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0), 23),
    Ipv6Net::new_assert(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0), 32),
    Ipv6Net::new_assert(Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0), 16),
    Ipv6Net::new_assert(Ipv6Addr::new(0x3fff, 0, 0, 0, 0, 0, 0, 0), 20),
];

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
    if url.as_str().len() > MAX_URL_LENGTH {
        bail!("normalized URL exceeds {MAX_URL_LENGTH} bytes");
    }
    Ok(url)
}

pub(crate) fn deduplication_key(url: &Url, ignore_query: bool) -> Url {
    let mut key = url.clone();
    if ignore_query {
        key.set_query(None);
    }
    key
}

pub fn is_public_ip(address: IpAddr) -> bool {
    let address = match address {
        IpAddr::V6(address) => match address.to_ipv4_mapped() {
            Some(mapped) => IpAddr::V4(mapped),
            None => IpAddr::V6(address),
        },
        address => address,
    };
    match address {
        IpAddr::V4(address) => !DENIED_IPV4.iter().any(|network| network.contains(&address)),
        IpAddr::V6(address) => {
            if address.segments()[0] & 0xe000 != 0x2000 {
                return false;
            }
            !DENIED_IPV6.iter().any(|network| network.contains(&address))
        }
    }
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
#[path = "tests/policy.rs"]
mod tests;
