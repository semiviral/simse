use crate::AsyncBufReadWriteUnpin;
use anyhow::{anyhow, bail, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{
    de::{Error, Visitor},
    Deserialize,
};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    num::NonZeroUsize,
    time::Duration,
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufStream},
    net::TcpStream,
    time::timeout,
};
use tracing::{info, warn};

pub static SENDER_VALIDATOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<name>[\w]+) <(?P<local>[\w\-\.]+)@(?P<domain>(?:[\w-]+\.)+[\w-]{2,})>$")
        .expect("failed to compile email validator regex")
});
pub static EMAIL_VALIDATOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<local>[\w\-\.]+)@(?P<domain>(?:[\w-]+\.)+[\w-]{2,})$")
        .expect("failed to compile email validator regex")
});

#[derive(Debug)]
pub struct Sender {
    name: String,
    email: Email,
}

impl Sender {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn email(&self) -> &Email {
        &self.email
    }
}

impl std::fmt::Display for Sender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} <{}>", self.name(), self.email()))
    }
}

impl<'de> Deserialize<'de> for Sender {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(SenderVisitor)
    }
}

struct SenderVisitor;
impl<'de> Visitor<'de> for SenderVisitor {
    type Value = Sender;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a formatted sender: \"Sender <sender@example.com>\"")
    }

    fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: Error,
    {
        let captures = SENDER_VALIDATOR
            .captures(v)
            .ok_or(Error::custom("value is in incorrect format"))?;
        let name = captures.name("name").unwrap();
        let local = captures.name("local").unwrap();
        let domain = captures.name("domain").unwrap();

        Ok(Sender {
            name: name.as_str().to_string(),
            email: Email {
                local: local.as_str().to_string(),
                domain: domain.as_str().to_string(),
            },
        })
    }
}

#[derive(Debug)]
pub struct Email {
    local: String,
    domain: String,
}

impl Email {
    pub fn new(email: &str) -> Result<Self> {
        let captures = EMAIL_VALIDATOR.captures(email).ok_or(anyhow!(
            "provided email could not be validated: {:?}",
            email
        ))?;
        let local = captures.name("local").ok_or(anyhow!(
            "provided email does not have local part: {:?}",
            email
        ))?;
        let domain = captures.name("domain").ok_or(anyhow!(
            "provided email does not have domain part: {:?}",
            email
        ))?;

        Ok(Self {
            local: local.as_str().to_owned(),
            domain: domain.as_str().to_owned(),
        })
    }

    #[inline]
    pub fn local(&self) -> &str {
        &self.local
    }

    #[inline]
    pub fn domain(&self) -> &str {
        &self.domain
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}@{}", self.local(), self.domain()))
    }
}

impl<'de> Deserialize<'de> for Email {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(EmailVisitor)
    }
}

struct EmailVisitor;
impl<'de> Visitor<'de> for EmailVisitor {
    type Value = Email;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a formatted email: \"sender@example.com\"")
    }

    fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: Error,
    {
        let captures = EMAIL_VALIDATOR
            .captures(v)
            .ok_or(Error::custom("value is in incorrect format"))?;

        let local = captures.name("local").unwrap();
        let domain = captures.name("domain").unwrap();

        Ok(Email {
            local: local.as_str().to_string(),
            domain: domain.as_str().to_string(),
        })
    }
}

pub struct Smtp {
    stream: BufStream<Box<dyn AsyncBufReadWriteUnpin>>,
    response_buf: String,
    mime: bool,
    auth: bool,
    chunking: bool,
    dsn: bool,
    pipelining: bool,
    starttls: bool,
    smtputf8: bool,
    size: Option<NonZeroUsize>,
    net_timeout: Duration,
    sender: Sender,
}

impl Smtp {
    const SMTP_RDY: &str = "220";
    const SMTP_OK: &str = "250";

    async fn read_line<R: AsyncBufRead + Unpin>(
        stream: &mut R,
        response_buf: &mut String,
    ) -> Result<()> {
        response_buf.clear();
        stream.read_line(response_buf).await?;
        info!("SMTP IN: {:?}", response_buf);

        Ok(())
    }

    async fn write_line<R: AsyncWrite + Unpin>(stream: &mut R, line: &str) -> Result<()> {
        stream.write_all(line.as_bytes()).await?;

        if !line.ends_with('\n') && !line.ends_with("\r\n") {
            stream.write_all(b"\r\n").await?;
        }

        stream.flush().await?;
        info!("SMTP OUT: {:?}", line);

        Ok(())
    }

    pub async fn new(host: &str, port: u16, net_timeout: Duration, sender: Sender) -> Result<Self> {
        let address = format!("{}:{}", host, port);
        let socket_addrs: Box<[SocketAddr]> = address
            .to_socket_addrs()
            .with_context(|| "DNS query returned no results")?
            .collect();

        info!("Attempting to connect to SMTP notifier: {}", address);
        let mut stream = timeout(net_timeout, TcpStream::connect(&*socket_addrs)).await??;
        let mut stream_buf = BufStream::new(&mut stream);
        let mut response_buf = String::with_capacity(1024);

        let mut mime = false;
        let mut auth = false;
        let mut chunking = false;
        let mut dsn = false;
        let mut pipelining = false;
        let mut starttls = false;
        let mut smtputf8 = false;
        let mut size = 0;

        // Wait for ready
        info!("Waiting for SMTP 220 Service Ready signal.");
        Self::read_line(&mut stream_buf, &mut response_buf).await?;
        if !response_buf.starts_with(Self::SMTP_RDY) {
            bail!("Expected SMTP ready, got: {:?}", &response_buf);
        }

        // Send EHLO to inform server we are ready
        Self::write_line(
            &mut stream_buf,
            &format!("EHLO {}", gethostname::gethostname().to_string_lossy()),
        )
        .await?;
        // Wait for reply hello
        Self::read_line(&mut stream_buf, &mut response_buf).await?;
        if !response_buf.starts_with(Self::SMTP_OK) {
            bail!(
                "expected SMTP Hello from server, instead got: {:?}",
                &response_buf
            );
        }

        loop {
            let timeout_result = timeout(
                net_timeout,
                Smtp::read_line(&mut stream_buf, &mut response_buf),
            )
            .await;
            let Ok(Ok(_)) = timeout_result else { break };

            let Some((code, value)) = response_buf.split_once(|c| c == ' ' || c == '-') else {
                warn!("Unrecognized SMTP response: {:?}", &response_buf);
                break;
            };

            if code != Self::SMTP_OK {
                bail!("Unrecognized SMTP response: {:?}", &response_buf);
            }

            if value.starts_with("AUTH") {
                auth = true;
            } else if value.starts_with("8BITMIME") {
                mime = true;
            } else if value.starts_with("PIPELINING") {
                pipelining = true;
            } else if value.starts_with("CHUNKING") {
                chunking = true;
            } else if value.starts_with("DSN") {
                dsn = true;
            } else if value.starts_with("STARTTLS") {
                starttls = true;
            } else if value.starts_with("SMTPUTF8") {
                smtputf8 = true;
            } else if value.starts_with("SIZE") {
                if let Some((_, num_str)) = value.split_once(' ') {
                    let Ok(size_value) = num_str.parse()
                    else {
                        warn!("SMTP 'SIZE' capability provided invalid size: {:?}", num_str);
                        continue
                    };

                    size = size_value
                }
            }
        }

        let stream: Box<dyn AsyncBufReadWriteUnpin> = if starttls {
            use tokio_rustls::{
                rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName},
                TlsConnector,
            };

            let mut root_store = RootCertStore::empty();
            root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
                |ta| {
                    OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                },
            ));
            let config = ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            let server_name = ServerName::try_from(host)?;

            Self::write_line(&mut stream_buf, "STARTTLS").await?;
            // Wait for STARTTLS ready from server
            Self::read_line(&mut stream_buf, &mut response_buf).await?;
            if !response_buf.starts_with(Self::SMTP_RDY) {
                bail!("Expected SMTP ready, got: {:?}", &response_buf);
            }

            let connector = TlsConnector::from(std::sync::Arc::new(config));
            drop(stream_buf);
            let stream = connector.connect(server_name, stream).await?;
            info!("SMTP TLS connection established.");

            Box::new(stream)
        } else {
            drop(stream_buf);
            Box::new(stream)
        };

        Ok(Self {
            stream: BufStream::new(stream),
            response_buf,
            mime,
            auth,
            chunking,
            dsn,
            pipelining,
            starttls,
            smtputf8,
            size: NonZeroUsize::new(size),
            net_timeout,
            sender,
        })
    }

    #[inline]
    pub const fn has_8bitmime(&self) -> bool {
        self.mime
    }

    #[inline]
    pub const fn has_auth(&self) -> bool {
        self.auth
    }

    #[inline]
    pub const fn has_chunking(&self) -> bool {
        self.chunking
    }

    #[inline]
    pub const fn has_dsn(&self) -> bool {
        self.dsn
    }

    #[inline]
    pub const fn has_pipelining(&self) -> bool {
        self.pipelining
    }

    #[inline]
    pub const fn has_size(&self) -> Option<NonZeroUsize> {
        self.size
    }

    #[inline]
    pub const fn has_starttls(&self) -> bool {
        self.starttls
    }

    #[inline]
    pub const fn has_smtputf8(&self) -> bool {
        self.smtputf8
    }

    async fn wait_ok(&mut self) -> Result<()> {
        timeout(
            self.net_timeout,
            Self::read_line(&mut self.stream, &mut self.response_buf),
        )
        .await??;

        if self.response_buf.starts_with(Self::SMTP_OK) {
            Ok(())
        } else {
            Err(anyhow!("did not recieve OK from SMTP server"))
        }
    }

    pub async fn send_mail() {}
}
