use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::extract::Request;
use axum::http::HeaderMap;
use sha2::{Digest, Sha256};

use crate::{auth::ct_eq, routes::bearer};

#[derive(Clone, Copy, Debug)]
pub(super) struct Cidr(IpAddr, u8);

impl Cidr {
    pub(super) fn parse(value: &str) -> Option<Self> {
        let (addr, prefix) = match value.split_once('/') {
            Some((addr, prefix)) => (
                addr.parse::<IpAddr>().ok()?,
                Some(prefix.parse::<u8>().ok()?),
            ),
            None => (value.parse().ok()?, None),
        };
        let bits = if addr.is_ipv4() { 32 } else { 128 };
        let prefix = prefix.unwrap_or(bits);
        if prefix > bits {
            return None;
        }
        let canonical = match addr {
            IpAddr::V4(a) => IpAddr::V4(
                (u32::from(a) & (u32::MAX.checked_shl(u32::from(bits - prefix)).unwrap_or(0)))
                    .into(),
            ),
            IpAddr::V6(a) => IpAddr::V6(
                (u128::from(a) & (u128::MAX.checked_shl(u32::from(bits - prefix)).unwrap_or(0)))
                    .into(),
            ),
        };
        Some(Self(canonical, prefix))
    }

    pub(super) fn contains(self, ip: IpAddr) -> bool {
        if self.0.is_ipv4() != ip.is_ipv4() {
            return false;
        }
        match (self.0, ip) {
            (IpAddr::V4(a), IpAddr::V4(b)) => {
                (u32::from(a) ^ u32::from(b))
                    .checked_shr(u32::from(32 - self.1))
                    .unwrap_or(0)
                    == 0
            }
            (IpAddr::V6(a), IpAddr::V6(b)) => {
                (u128::from(a) ^ u128::from(b))
                    .checked_shr(u32::from(128 - self.1))
                    .unwrap_or(0)
                    == 0
            }
            _ => false,
        }
    }

    pub(super) fn canonical(self) -> String {
        format!("{}/{}", self.0, self.1)
    }
}

pub(super) fn hex_hash(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

#[derive(Clone, Default)]
pub struct AdminConfig {
    token_hash: Option<[u8; 32]>,
    allowlist: Vec<Cidr>,
    trusted_proxies: Vec<Cidr>,
}

impl AdminConfig {
    pub fn parse(
        hash: Option<&str>,
        allowlist: Option<&str>,
        proxies: Option<&str>,
    ) -> Result<Self, String> {
        fn cidrs(value: Option<&str>, name: &str) -> Result<Vec<Cidr>, String> {
            value
                .into_iter()
                .flat_map(|s| s.split(','))
                .map(|s| {
                    Cidr::parse(s.trim()).ok_or_else(|| format!("{name} must contain valid CIDRs"))
                })
                .collect()
        }
        let token_hash = hash
            .map(|s| hex_hash(s).ok_or("ADMIN_TOKEN_HASH must be 64 hexadecimal characters"))
            .transpose()?;
        Ok(Self {
            token_hash,
            allowlist: cidrs(allowlist, "ADMIN_IP_ALLOWLIST")?,
            trusted_proxies: cidrs(proxies, "ADMIN_TRUSTED_PROXIES")?,
        })
    }

    pub(crate) fn client_ip(&self, req: &Request) -> Option<IpAddr> {
        let canonical = |ip: IpAddr| match ip {
            IpAddr::V6(addr) => addr.to_ipv4_mapped().map_or(ip, IpAddr::V4),
            _ => ip,
        };
        let mut ip = canonical(req.extensions().get::<ConnectInfo<SocketAddr>>()?.0.ip());
        let trusted = |addr| self.trusted_proxies.iter().any(|cidr| cidr.contains(addr));
        if !trusted(ip) {
            return Some(ip);
        }
        // A configured proxy appends the real previous hop. Never trust a prefix
        // supplied by a caller beyond the first untrusted hop on the right.
        let chain: Vec<_> = req.headers().get_all("x-forwarded-for").iter().collect();
        for header in chain.into_iter().rev() {
            for entry in header.to_str().ok()?.rsplit(',') {
                ip = canonical(entry.trim().parse().ok()?);
                if !trusted(ip) {
                    return Some(ip);
                }
            }
        }
        // A peer configured as a proxy must supply a valid chain. An all-trusted
        // chain is ambiguous and cannot authorize a request.
        None
    }

    pub(super) fn authorized(&self, headers: &HeaderMap, ip: Option<IpAddr>) -> bool {
        let token = bearer(headers);
        let actual = Sha256::digest(token.unwrap_or_default());
        let expected = self.token_hash.unwrap_or_default();
        let matched = ct_eq(actual.as_ref(), &expected);
        let allowed = ip.is_some_and(|addr| self.allowlist.iter().any(|cidr| cidr.contains(addr)));
        self.token_hash.is_some() & token.is_some() & matched & allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cidr_matches_v4_and_v6_prefixes() {
        for (cidr, within, outside) in [
            ("192.0.2.99/24", "192.0.2.255", "192.0.3.0"),
            (
                "2001:db8:0:12::1/64",
                "2001:db8:0:12::abcd",
                "2001:db8:0:13::",
            ),
            ("192.0.2.3", "192.0.2.3", "192.0.2.4"),
            ("1.2.3.4/0", "8.7.6.5", "::1"),
        ] {
            let subnet = Cidr::parse(cidr).unwrap();
            assert!(subnet.contains(within.parse().unwrap()));
            assert!(!subnet.contains(outside.parse().unwrap()));
        }
        assert!(Cidr::parse("192.0.2.1/33").is_none());
    }
}
