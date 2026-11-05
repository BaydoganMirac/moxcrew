//! Public read-only sources with pinned DNS and bounded redirects.
use crate::{Error, Result};
use reqwest::Url;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

pub(crate) fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192
                    && (b == 168 || (b == 0 && (c == 0 || c == 2)) || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return public_ip(v4.into());
            }
            let s = ip.segments();
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && s[0] != 0x2002
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}

pub(crate) fn public_url(raw: &str) -> Result<Url> {
    let mut url = Url::parse(raw).map_err(|_| Error("WEB_URL_INVALID".into()))?;
    let host = url
        .host_str()
        .ok_or_else(|| Error("WEB_URL_INVALID".into()))?;
    if raw.len() > 4096
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(if url.scheme() == "https" { 443 } else { 80 })
        || host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.ends_with(".local")
    {
        return Err(Error("WEB_URL_INVALID".into()));
    }
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        if !public_ip(ip) {
            return Err(Error("WEB_PRIVATE_ADDRESS_DENIED".into()));
        }
    } else if !host.contains('.') {
        return Err(Error("WEB_PRIVATE_ADDRESS_DENIED".into()));
    }
    url.set_fragment(None);
    Ok(url)
}

pub(crate) async fn read_public(raw: &str, limit: usize) -> Result<(String, Vec<u8>)> {
    if limit == 0 || limit > 4 * 1024 * 1024 {
        return Err(Error("WEB_SOURCE_LIMIT".into()));
    }
    tokio::time::timeout(Duration::from_secs(12), async {
        let mut url = public_url(raw)?;
        for hop in 0..=3 {
            let host = url
                .host_str()
                .ok_or_else(|| Error("WEB_URL_INVALID".into()))?
                .trim_matches(['[', ']'])
                .to_owned();
            let port = url.port_or_known_default().unwrap();
            let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
                .await
                .map_err(|_| Error("WEB_DNS_FAILED".into()))?
                .take(17)
                .collect();
            if addresses.is_empty()
                || addresses.len() > 16
                || addresses.iter().any(|a| !public_ip(a.ip()))
            {
                return Err(Error("WEB_PRIVATE_ADDRESS_DENIED".into()));
            }
            let client = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .resolve_to_addrs(&host, &addresses)
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(|_| Error("WEB_SOURCE_UNAVAILABLE".into()))?;
            let mut response = client
                .get(url.clone())
                .header("User-Agent", "Moxcrew/0.12 read-only source")
                .header("Accept-Encoding", "identity")
                .send()
                .await
                .map_err(|_| Error("WEB_SOURCE_UNAVAILABLE".into()))?;
            if response.status().is_redirection() {
                if hop == 3 {
                    return Err(Error("WEB_REDIRECT_LIMIT".into()));
                }
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|h| h.to_str().ok())
                    .ok_or_else(|| Error("WEB_REDIRECT_INVALID".into()))?;
                url = public_url(
                    url.join(location)
                        .map_err(|_| Error("WEB_REDIRECT_INVALID".into()))?
                        .as_str(),
                )?;
                continue;
            }
            if !response.status().is_success() {
                return Err(Error("WEB_SOURCE_UNAVAILABLE".into()));
            }
            if response.content_length().is_some_and(|n| n > limit as u64) {
                return Err(Error("WEB_SOURCE_LIMIT".into()));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| Error("WEB_SOURCE_UNAVAILABLE".into()))?
            {
                if bytes.len() + chunk.len() > limit {
                    return Err(Error("WEB_SOURCE_LIMIT".into()));
                }
                bytes.extend_from_slice(&chunk);
            }
            return Ok((url.to_string(), bytes));
        }
        Err(Error("WEB_REDIRECT_LIMIT".into()))
    })
    .await
    .map_err(|_| Error("WEB_SOURCE_TIMEOUT".into()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_reads_reject_private_mapped_reserved_and_credential_urls() {
        for ip in [
            "127.0.0.1",
            "10.2.3.4",
            "100.64.0.1",
            "169.254.169.254",
            "172.31.0.1",
            "192.168.0.2",
            "198.18.1.2",
            "203.0.113.1",
            "::1",
            "::ffff:127.0.0.1",
            "2001:db8::1",
            "2002:7f00:1::",
            "fc00::1",
            "fe80::1",
            "3fff::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(public_ip(ip.parse().unwrap()));
        }
        for url in [
            "file:///etc/passwd",
            "http://localhost/",
            "https://internal/",
            "http://127.1/",
            "http://2130706433/",
            "https://[::ffff:127.0.0.1]/",
            "https://user:pass@example.com/",
            "http://example.com:8080/",
        ] {
            assert!(public_url(url).is_err(), "{url}");
        }
        assert_eq!(
            public_url("https://example.com/a?q=1#fragment")
                .unwrap()
                .as_str(),
            "https://example.com/a?q=1"
        );
    }
}
