use crate::{Error, Result};
use notes_sync::{
    transfer::{ApplicationAcknowledgment, Publication},
    Revision,
};
use reqwest::{
    blocking::{Client, Response},
    Url,
};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    path::Path,
    time::Duration,
};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub origin: String,
    pub name: String,
    pub allow_private: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<notes_model::RelPath>,
}
impl Endpoint {
    pub fn validate(&self) -> Result<Url> {
        let url = Url::parse(&self.origin).map_err(|_| Error::Invalid)?;
        if url.username() != ""
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || url.host_str().is_none()
            || self
                .scope
                .as_ref()
                .is_some_and(|p| p.is_root() || p.as_str().split('/').any(|n| n.starts_with('.')))
            || self.name.is_empty()
            || self.name.len() > 64
            || !self
                .name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        {
            return Err(Error::Invalid);
        }
        let host = url.host_str().unwrap().trim_matches(['[', ']']);
        let literal = host.parse::<IpAddr>().ok();
        if url.scheme() != "https"
            && !(url.scheme() == "http"
                && self.allow_private
                && literal.is_some_and(|ip| ip.is_loopback()))
        {
            return Err(Error::Invalid);
        }
        Ok(url)
    }
}
fn allowed_ip(ip: IpAddr, private: bool) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let b = ip.octets();
            if ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || b[0] >= 224
                || b[0] == 0
                || ip.is_documentation()
            {
                return false;
            }
            private
                || !(ip.is_private()
                    || ip.is_loopback()
                    || (b[0] == 100 && (64..128).contains(&b[1])))
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return allowed_ip(IpAddr::V4(v4), private);
            }
            if ip.is_unspecified() || ip.is_multicast() || ip.is_unicast_link_local() {
                return false;
            }
            private || !(ip.is_loopback() || ip.is_unique_local())
        }
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub workspace: Uuid,
    pub revisions: Vec<Revision>,
    pub heads: std::collections::BTreeMap<notes_model::NoteId, Uuid>,
    pub next_cursor: usize,
    pub has_more: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    revision: Uuid,
    stored: bool,
    applied: bool,
}
/// Also implemented by fault-injecting tests. No filesystem methods belong here.
pub trait Transport {
    fn page(&mut self, cursor: usize) -> Result<Page>;
    fn fetch(&mut self, id: Uuid) -> Result<Publication>;
    fn publish(&mut self, publication: &Publication) -> Result<()>;
    fn acknowledge(&mut self, receipt: &ApplicationAcknowledgment) -> Result<()>;
}
pub struct Remote {
    client: Client,
    base: Url,
    bearer: String,
    scope: Option<notes_model::RelPath>,
}
impl Remote {
    pub fn connect(endpoint: &Endpoint, token_file: &Path, ca_file: Option<&Path>) -> Result<Self> {
        let url = endpoint.validate()?;
        let meta = std::fs::symlink_metadata(token_file).map_err(|_| Error::Denied)?;
        if !token_file.is_absolute() || !meta.is_file() || meta.len() > 200 {
            return Err(Error::Denied);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o077 != 0 {
                return Err(Error::Denied);
            }
        }
        let bearer = std::fs::read_to_string(token_file)
            .map_err(|_| Error::Denied)?
            .trim()
            .to_owned();
        if !bearer.starts_with("nt_")
            || bearer.len() > 199
            || bearer.chars().any(char::is_whitespace)
        {
            return Err(Error::Denied);
        }
        let host = url
            .host_str()
            .ok_or(Error::Invalid)?
            .trim_matches(['[', ']']);
        let port = url.port_or_known_default().ok_or(Error::Invalid)?;
        let addresses: Vec<_> = if let Ok(ip) = host.parse::<IpAddr>() {
            vec![SocketAddr::new(ip, port)]
        } else {
            (host, port)
                .to_socket_addrs()
                .map_err(|_| Error::Offline)?
                .take(17)
                .collect()
        };
        if addresses.is_empty()
            || addresses.len() > 16
            || addresses
                .iter()
                .any(|a| !allowed_ip(a.ip(), endpoint.allow_private))
        {
            return Err(Error::Invalid);
        }
        // rustls-no-provider requires explicit process initialization. A provider
        // already selected by the embedding process is left intact.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut builder = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .resolve_to_addrs(host, &addresses);
        if let Some(path) = ca_file {
            let meta = std::fs::metadata(path).map_err(|_| Error::Invalid)?;
            if !meta.is_file() || meta.len() > 64 * 1024 {
                return Err(Error::Invalid);
            }
            let bytes = std::fs::read(path).map_err(|_| Error::Invalid)?;
            let cert = reqwest::Certificate::from_pem(&bytes).map_err(|_| Error::Invalid)?;
            builder = builder.tls_certs_merge([cert]);
        }
        let client = builder.build().map_err(|_| Error::Invalid)?;
        // Pin the selected scope exactly; changing credentials must not silently
        // broaden or narrow the client namespace.
        let workspaces: serde_json::Value = decode(
            client
                .get(url.join("v1/workspaces").map_err(|_| Error::Invalid)?)
                .bearer_auth(&bearer)
                .send()
                .map_err(|_| Error::Offline)?,
        )?;
        let rows = workspaces["workspaces"].as_array().ok_or(Error::Protocol)?;
        if rows.len() != 1
            || rows[0]["name"] != endpoint.name
            || rows[0]["scope"] != endpoint.scope.as_ref().map(|p| p.as_str()).unwrap_or("")
            || rows[0]["review"] != false
        {
            return Err(Error::Denied);
        }
        let base = url
            .join(&format!("v1/workspaces/{}/sync/revisions", endpoint.name))
            .map_err(|_| Error::Invalid)?;
        Ok(Self {
            client,
            base,
            bearer,
            scope: endpoint.scope.clone(),
        })
    }
}
impl Remote {
    fn localize(&self, r: &mut Revision) -> Result<()> {
        if let Some(scope) = &self.scope {
            let path = r
                .path
                .as_str()
                .strip_prefix(&format!("{scope}/"))
                .ok_or(Error::Denied)?;
            r.path = notes_model::RelPath::parse(path).map_err(|_| Error::Protocol)?;
        }
        Ok(())
    }
    fn globalize(&self, r: &mut Revision) -> Result<()> {
        if let Some(scope) = &self.scope {
            r.path = notes_model::RelPath::parse(&format!("{scope}/{}", r.path))
                .map_err(|_| Error::Invalid)?;
        }
        Ok(())
    }
}
fn decode<T: serde::de::DeserializeOwned>(response: Response) -> Result<T> {
    match response.status().as_u16() {
        200 => {}
        401 | 403 => return Err(Error::Denied),
        409 => return Err(Error::Conflict),
        507 => return Err(Error::Limit),
        429 | 503 => return Err(Error::Busy),
        _ => return Err(Error::Protocol),
    }
    if response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(';').next())
        != Some("application/json")
    {
        return Err(Error::Protocol);
    }
    let mut bytes = vec![];
    response
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Offline)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(Error::Limit);
    }
    serde_json::from_slice(&bytes).map_err(|_| Error::Protocol)
}
impl Transport for Remote {
    fn acknowledge(&mut self, receipt: &ApplicationAcknowledgment) -> Result<()> {
        let response: ApplicationAcknowledgment = decode(
            self.client
                .post(
                    self.base
                        .join("acknowledgments")
                        .map_err(|_| Error::Invalid)?,
                )
                .bearer_auth(&self.bearer)
                .json(receipt)
                .send()
                .map_err(|_| Error::Offline)?,
        )?;
        if response != *receipt {
            return Err(Error::Protocol);
        }
        Ok(())
    }

    fn page(&mut self, cursor: usize) -> Result<Page> {
        let mut url = self.base.clone();
        url.set_query(Some(&format!("cursor={cursor}&limit=20")));
        let mut page: Page = decode(
            self.client
                .get(url)
                .bearer_auth(&self.bearer)
                .send()
                .map_err(|_| Error::Offline)?,
        )?;
        for revision in &mut page.revisions {
            self.localize(revision)?;
        }
        Ok(page)
    }
    fn fetch(&mut self, id: Uuid) -> Result<Publication> {
        let url = Url::parse(&format!("{}/{id}", self.base)).map_err(|_| Error::Invalid)?;
        let mut p: Publication = decode(
            self.client
                .get(url)
                .bearer_auth(&self.bearer)
                .send()
                .map_err(|_| Error::Offline)?,
        )?;
        self.localize(&mut p.revision)?;
        for branch in &mut p.branches {
            self.localize(&mut branch.revision)?;
        }
        Ok(p)
    }
    fn publish(&mut self, p: &Publication) -> Result<()> {
        let mut wire = p.clone();
        self.globalize(&mut wire.revision)?;
        for branch in &mut wire.branches {
            self.globalize(&mut branch.revision)?;
        }
        let receipt: Receipt = decode(
            self.client
                .post(self.base.clone())
                .bearer_auth(&self.bearer)
                .json(&wire)
                .send()
                .map_err(|_| Error::Offline)?,
        )?;
        if receipt.revision != p.revision.id || !receipt.stored || receipt.applied {
            return Err(Error::Protocol);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn address_policy_never_allows_metadata_or_redirect_credentials() {
        for ip in [
            "169.254.169.254",
            "fe80::1",
            "::ffff:169.254.169.254",
            "0.0.0.0",
            "224.0.0.1",
        ] {
            assert!(!allowed_ip(ip.parse().unwrap(), true));
        }
        assert!(!allowed_ip("127.0.0.1".parse().unwrap(), false));
        assert!(allowed_ip("127.0.0.1".parse().unwrap(), true));
        for origin in [
            "https://token@example.org/",
            "https://example.org/path",
            "https://example.org/?token=x",
            "http://example.org/",
            "http://localhost/",
        ] {
            assert!(Endpoint {
                scope: None,
                origin: origin.into(),
                name: "home".into(),
                allow_private: true
            }
            .validate()
            .is_err());
        }
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    #[test]
    fn client_builds_tls_backend_and_refuses_redirects() {
        for redirect in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut received = vec![];
                while !received.ends_with(b"\r\n\r\n") {
                    let mut b = [0u8; 1];
                    stream.read_exact(&mut b).unwrap();
                    received.push(b[0]);
                    assert!(received.len() < 4096);
                }
                let body = r#"{"workspaces":[{"name":"home","scope":"","review":false}]}"#;
                let response = if redirect {
                    "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
                } else {
                    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len())
                };
                stream.write_all(response.as_bytes()).unwrap();
            });
            let dir = tempfile::tempdir().unwrap();
            let token = dir.path().join("token");
            std::fs::write(&token, "nt_test").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
            let result = Remote::connect(
                &Endpoint {
                    scope: None,
                    origin: format!("http://{address}"),
                    name: "home".into(),
                    allow_private: true,
                },
                &token,
                None,
            );
            if redirect {
                assert!(matches!(result, Err(Error::Protocol)));
            } else {
                assert!(result.is_ok());
            }
            server.join().unwrap();
        }
    }
}
