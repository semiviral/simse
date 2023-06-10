use crate::{config::SmtpTlsMode, AsyncBufReadWriteUnpin};
use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{
    de::{Error, Visitor},
    Deserialize,
};
use std::{
    borrow::Cow,
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufStream},
    net::TcpStream,
    time::timeout,
};
use tracing::info;

pub static SENDER_VALIDATOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<name>[\w]+) <(?P<local>[\w\-\.]+)@(?P<domain>(?:[\w-]+\.)+[\w-]{2,})>$")
        .expect("failed to compile email validator regex")
});
pub static EMAIL_VALIDATOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<local>[\w\-\.]+)@(?P<domain>(?:[\w-]+\.)+[\w-]{2,})$")
        .expect("failed to compile email validator regex")
});
pub static SUBJECT_REPLACER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?P<title>\{title\})").expect("failed to compile subject replacer regex")
});

pub fn format_subject_title<'a>(subject: &'a str, title: &str) -> Cow<'a, str> {
    let Some(captures) = SUBJECT_REPLACER.captures(subject)
    else {
        return Cow::Borrowed(subject)
    };

    let Some(title_match) = captures.name("title")
    else {
        return Cow::Borrowed(subject);
    };

    let mut new_subject = String::new();
    new_subject.push_str(&subject[..title_match.start()]);
    new_subject.push_str(title);
    new_subject.push_str(&subject[title_match.end()..]);

    Cow::Owned(new_subject)
}

// #[derive(Debug)]
// pub struct Identity {
//     name: String,
//     email: Email,
// }

// impl Identity {
//     pub fn name(&self) -> &str {
//         &self.name
//     }

//     pub const fn email(&self) -> &Email {
//         &self.email
//     }
// }

// impl std::fmt::Display for Identity {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_fmt(format_args!("{} <{}>", self.name(), self.email()))
//     }
// }

// impl<'de> Deserialize<'de> for Identity {
//     fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         deserializer.deserialize_string(IdentityVisitor)
//     }
// }

// struct IdentityVisitor;
// impl<'de> Visitor<'de> for IdentityVisitor {
//     type Value = Identity;

//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("a formatted sender: \"Sender <sender@example.com>\"")
//     }

//     fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
//     where
//         E: Error,
//     {
//         self.visit_str(&v)
//     }

//     fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
//     where
//         E: Error,
//     {
//         let captures = SENDER_VALIDATOR
//             .captures(v)
//             .ok_or(Error::custom("value is in incorrect format"))?;
//         let name = captures.name("name").unwrap();
//         let local = captures.name("local").unwrap();
//         let domain = captures.name("domain").unwrap();

//         Ok(Identity {
//             name: name.as_str().to_string(),
//             email: Email {
//                 local: local.as_str().to_string(),
//                 domain: domain.as_str().to_string(),
//             },
//         })
//     }
// }

// #[derive(Debug)]
// pub struct Email {
//     local: String,
//     domain: String,
// }

// impl Email {
//     pub fn new(email: &str) -> Result<Self> {
//         let captures = EMAIL_VALIDATOR.captures(email).ok_or(anyhow!(
//             "provided email could not be validated: {:?}",
//             email
//         ))?;
//         let local = captures.name("local").ok_or(anyhow!(
//             "provided email does not have local part: {:?}",
//             email
//         ))?;
//         let domain = captures.name("domain").ok_or(anyhow!(
//             "provided email does not have domain part: {:?}",
//             email
//         ))?;

//         Ok(Self {
//             local: local.as_str().to_owned(),
//             domain: domain.as_str().to_owned(),
//         })
//     }

//     #[inline]
//     pub fn local(&self) -> &str {
//         &self.local
//     }

//     #[inline]
//     pub fn domain(&self) -> &str {
//         &self.domain
//     }
// }

// impl std::fmt::Display for Email {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_fmt(format_args!("{}@{}", self.local(), self.domain()))
//     }
// }

// impl<'de> Deserialize<'de> for Email {
//     fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         deserializer.deserialize_string(EmailVisitor)
//     }
// }

// struct EmailVisitor;
// impl<'de> Visitor<'de> for EmailVisitor {
//     type Value = Email;

//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("a formatted email: \"sender@example.com\"")
//     }

//     fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
//     where
//         E: Error,
//     {
//         self.visit_str(&v)
//     }

//     fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
//     where
//         E: Error,
//     {
//         let captures = EMAIL_VALIDATOR
//             .captures(v)
//             .ok_or(Error::custom("value is in incorrect format"))?;

//         let local = captures.name("local").unwrap();
//         let domain = captures.name("domain").unwrap();

//         Ok(Email {
//             local: local.as_str().to_string(),
//             domain: domain.as_str().to_string(),
//         })
//     }
// }

// pub struct Smtp {
//     stream: BufStream<Box<dyn AsyncBufReadWriteUnpin>>,
//     response_buf: String,
//     net_timeout: Duration,
//     sender: Identity,
//     subject: String,
// }

// impl Smtp {
//     const SMTP_RDY: &str = "220";
//     const SMTP_OK: &str = "250";

    

//     async fn read_line_from<R: AsyncBufRead + Unpin>(
//         stream: &mut R,
//         response_buf: &mut String,
//         net_timeout: Duration,
//     ) -> Result<()> {
//         response_buf.clear();
//         timeout(net_timeout, stream.read_line(response_buf)).await??;
//         info!("SMTP IN: {:?}", response_buf);

//         Ok(())
//     }

//     async fn write_line_to<R: AsyncWrite + Unpin>(
//         stream: &mut R,
//         line: &str,
//         net_timeout: Duration,
//     ) -> Result<()> {
//         stream.write_all(line.as_bytes()).await?;

//         if !line.ends_with('\n') && !line.ends_with("\r\n") {
//             stream.write_all(b"\r\n").await?;
//         }

//         timeout(net_timeout, stream.flush()).await??;

//         info!("SMTP OUT: {:?}", line);

//         Ok(())
//     }

//     async fn do_ehlo<RW: AsyncBufRead + AsyncWrite + Unpin>(
//         stream: &mut RW,
//         response_buf: &mut String,
//         net_timeout: Duration,
//     ) -> Result<()> {
//         static EHLO_MSG: Lazy<String> =
//             Lazy::new(|| format!("EHLO {}", gethostname::gethostname().to_string_lossy()));

//         // Send EHLO to inform server we are ready
//         Self::write_line_to(stream, &EHLO_MSG, net_timeout).await?;
//         stream.flush().await?;

//         // Wait for reply hello
//         Self::read_line_from(stream, response_buf, net_timeout).await?;
//         if !response_buf.starts_with(Self::SMTP_OK) {
//             bail!(
//                 "expected SMTP Hello from server, instead got: {:?}",
//                 &response_buf
//             );
//         }

//         // Cycle through capabilities; we don't care, they're specified by the user in the config.
//         while Self::read_line_from(stream, response_buf, Duration::from_secs(1))
//             .await
//             .is_ok()
//         {}

//         Ok(())
//     }

//     pub async fn new(
//         host: &str,
//         port: u16,
//         tls: SmtpTlsMode,
//         net_timeout: Duration,
//         sender: Identity,
//         subject: String,
//     ) -> Result<Self> {
//         let address = format!("{}:{}", host, port);
//         let socket_addrs: Box<[SocketAddr]> = address
//             .to_socket_addrs()
//             .with_context(|| "DNS query returned no results")?
//             .collect();

//         info!("Attempting to connect to SMTP notifier: {}", address);
//         let mut stream = timeout(net_timeout, TcpStream::connect(&*socket_addrs)).await??;
//         let mut stream_buf = BufStream::new(&mut stream);
//         let mut response_buf = String::with_capacity(1024);

//         // Wait for ready
//         info!("Waiting for SMTP 220 Service Ready signal.");
//         Self::read_line_from(&mut stream_buf, &mut response_buf, net_timeout).await?;
//         if !response_buf.starts_with(Self::SMTP_RDY) {
//             bail!("Expected SMTP ready, got: {:?}", &response_buf);
//         }

//         Self::do_ehlo(&mut stream_buf, &mut response_buf, net_timeout).await?;

//         let stream: Box<dyn AsyncBufReadWriteUnpin> = {
//             match tls {
//                 SmtpTlsMode::StartTls => {
//                     use tokio_rustls::{
//                         rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName},
//                         TlsConnector,
//                     };

//                     let mut root_store = RootCertStore::empty();
//                     root_store.add_server_trust_anchors(
//                         webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
//                             OwnedTrustAnchor::from_subject_spki_name_constraints(
//                                 ta.subject,
//                                 ta.spki,
//                                 ta.name_constraints,
//                             )
//                         }),
//                     );
//                     let config = ClientConfig::builder()
//                         .with_safe_defaults()
//                         .with_root_certificates(root_store)
//                         .with_no_client_auth();
//                     let server_name = ServerName::try_from(host)?;

//                     Self::write_line_to(&mut stream_buf, "STARTTLS", net_timeout).await?;
//                     stream_buf.flush().await?;
//                     // Wait for STARTTLS ready from server
//                     Self::read_line_from(&mut stream_buf, &mut response_buf, net_timeout).await?;
//                     if !response_buf.starts_with(Self::SMTP_RDY) {
//                         bail!("Expected SMTP ready, got: {:?}", &response_buf);
//                     }

//                     let connector = TlsConnector::from(std::sync::Arc::new(config));
//                     drop(stream_buf);
//                     let mut stream = connector.connect(server_name, stream).await?;
//                     info!("SMTP TLS connection established.");

//                     let mut stream_buf = BufStream::new(&mut stream);
//                     Self::do_ehlo(&mut stream_buf, &mut response_buf, net_timeout).await?;
//                     drop(stream_buf);

//                     Box::new(stream)
//                 }

//                 SmtpTlsMode::ForceTls => todo!(),

//                 SmtpTlsMode::Off => Box::new(stream),
//             }
//         };

//         Ok(Self {
//             stream: BufStream::new(stream),
//             response_buf,
//             net_timeout,
//             sender,
//             subject,
//         })
//     }

//     pub async fn read_line(&mut self) -> Result<()> {
//         Self::read_line_from(&mut self.stream, &mut self.response_buf, self.net_timeout).await?;

//         Ok(())
//     }

//     pub async fn write_line(&mut self, s: &str) -> Result<()> {
//         Self::write_line_to(&mut self.stream, s, self.net_timeout).await?;

//         Ok(())
//     }

//     async fn wait_code(&mut self, code: usize) -> Result<()> {
//         self.read_line().await?;

//         let code_str = code.to_string();
//         if self.response_buf.starts_with(&code_str) {
//             Ok(())
//         } else {
//             Err(anyhow!("unexpected SMTP code: {:?}", self.response_buf))
//         }
//     }

//     pub async fn authenticate(&mut self, username: Option<&str>, password: &str) -> Result<()> {
//         use base64::engine::general_purpose;

//         let auth_data = general_purpose::STANDARD.encode(if let Some(username) = username {
//             format!("\0{}\0{}", username, password)
//         } else {
//             password.to_string()
//         });

//         self.write_line(&format!("AUTH PLAIN {}", auth_data))
//             .await?;

//         self.wait_code(235).await?;

//         Ok(())
//     }

//     pub async fn send_mail(
//         &mut self,
//         to: Identity,
//         cc: Option<&[Email]>,
//         title: &str,
//     ) -> Result<()> {
//         let cc = cc.unwrap_or_default();

//         self.write_line(&format!("MAIL FROM:<{}>", self.sender.email()))
//             .await?;
//         self.wait_code(250).await?;

//         self.write_line(&format!("RCPT TO:<{}>", to)).await?;
//         self.wait_code(250).await?;

//         for email in cc {
//             self.write_line(&format!("RCPT TO:<{}>", email)).await?;
//             self.wait_code(250).await?;
//         }

//         self.write_line("DATA").await?;
//         self.wait_code(354).await?;

//         self.write_line(&format!("From: {}", self.sender,)).await?;
//         self.write_line(&format!("To: {}", to)).await?;

//         for email in cc {
//             self.write_line(&format!("Cc: {}", email)).await?;
//         }

//         self.write_line(&format!("Date: {}", chrono::Utc::now()))
//             .await?;
//         self.write_line(&format!(
//             "Subject: {}",
//             Self::format_subject_title(&self.subject, title)
//         ))
//         .await?;

//         self.write_line("").await?;
//         self.write_line("This is some test data.").await?;
//         self.write_line("This is some more test data.").await?;
//         self.write_line("").await?;
//         self.write_line(".").await?;
//         self.wait_code(250).await?;

//         self.write_line("QUIT").await?;

//         Ok(())
//     }
// }
