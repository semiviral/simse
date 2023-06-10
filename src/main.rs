mod config;
mod notifiers;

use anyhow::{Context, Result};
use config::Config;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::info;

trait AsyncReadWrite: AsyncRead + AsyncWrite {}
impl<RW: AsyncRead + AsyncWrite> AsyncReadWrite for RW {}

trait AsyncBufReadWriteUnpin: AsyncRead + AsyncWrite + Unpin {}
impl<RWU: AsyncRead + AsyncWrite + Unpin> AsyncBufReadWriteUnpin for RWU {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = read_config().await.unwrap();
    info!("Config: {:#?}", config);

    if let Some(smtp_notifier) = config.notifier.smtp {
        let _smtp = notifiers::smtp::Smtp::new(
            &smtp_notifier.host,
            smtp_notifier.port,
            std::time::Duration::from_secs(smtp_notifier.timeout),
            smtp_notifier.sender,
        )
        .await
        .unwrap();
}

    // let mut watcher = notify::recommended_watcher(|res| match res {
    //     Ok(event) => trace!("FS event fired: {:?}", event),
    //     Err(err) => todo!(),
    // })
    // .unwrap();

    // let watch_path = std::path::Path::new("blast");
    // watcher
    //     .watch(watch_path, notify::RecursiveMode::Recursive)
    //     .unwrap();
    // info!("Watching path for changes: {:?}", watch_path);

    // let app = Router::new().route("/", get(root));

    // let addr = SocketAddr::from(([127, 0, 0, 1], 3006));

    // info!("listening on {}", addr);
    // axum::Server::bind(&addr)
    //     .serve(app.into_make_service())
    //     .await
    //     .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn read_config() -> Result<Config> {
    let config_path = option_env!("SIMSE_CONFIG_PATH").unwrap_or("config.toml");
    let config_file = tokio::fs::read_to_string(config_path)
        .await
        .with_context(|| format!("failed to read config from path: {}", config_path))?;

    toml::from_str(&config_file).with_context(|| "failed to parse valid config TOML from file")
}

trait AsyncReadWrite: AsyncRead + AsyncWrite {}
impl<RW: AsyncRead + AsyncWrite> AsyncReadWrite for RW {}

struct Smtp {
    stream: Box<dyn AsyncReadWrite>,
    response_buf: String,
    mime: bool,
    auth: bool,
    chunking: bool,
    dsn: bool,
    pipelining: bool,
    size: Option<NonZeroUsize>,
    starttls: bool,
    smtputf8: bool,
}

impl Smtp {
    const SMTP_RDY: &str = "220";
    const SMTP_SUCCESS: &str = "250";

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

        if !line.ends_with('\n')&&!line.ends_with("\r\n") {
            stream.write_all(b"\r\n").await?;
        }

        stream.flush().await?;
        info!("SMTP OUT: {:?}", line);

        Ok(())
    }

    pub async fn new(address: &str, host_fqdn: &str) -> Result<Self> {
        let socket_addrs: Box<[SocketAddr]> = address
            .to_socket_addrs()
            .with_context(|| "DNS query returned no results")?
            .collect();

        info!("Attempting to connect to SMTP notifier: {}", address);
        let mut stream =
            timeout(Duration::from_secs(5), TcpStream::connect(&*socket_addrs)).await??;
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
        Self::write_line(&mut stream_buf, &format!("EHLO {}", host_fqdn)).await?;
        // Wait for reply hello
        Self::read_line(&mut stream_buf, &mut response_buf).await?;
        if !response_buf.starts_with(Self::SMTP_SUCCESS) {
            bail!(
                "expected SMTP Hello from server, instead got: {:?}",
                &response_buf
            );
        }

        const SMTP_250_TIMEOUT: Duration = Duration::from_secs(3);

        loop {
            let timeout_result = timeout(
                SMTP_250_TIMEOUT,
                Smtp::read_line(&mut stream_buf, &mut response_buf),
            )
            .await;
            let Ok(Ok(_)) = timeout_result else { break };

            let Some((code, value)) = response_buf.split_once(|c| c == ' ' || c == '-') else {
                warn!("Unrecognized SMTP response: {:?}", &response_buf);
                break;
            };

            if code != Self::SMTP_SUCCESS {
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

        let stream: Box<dyn AsyncReadWrite> = if starttls {
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
            let server_addresss = address.split_once(':').map(|(s, _)| s).unwrap_or(address);
            let server_name = ServerName::try_from(server_addresss)?;

            Self::write_line(&mut stream_buf, "STARTTLS").await?;
            // Wait for STARTTLS ready from server
            Self::read_line(&mut stream_buf, &mut response_buf).await?;
            if !response_buf.starts_with(Self::SMTP_RDY) {
                bail!("Expected SMTP ready, got: {:?}", &response_buf);
            }

            let connector = TlsConnector::from(Arc::new(config));
            drop(stream_buf);
            let stream = connector.connect(server_name, stream).await?;
            info!("SMTP TLS connection established.");

            Box::new(stream)
        } else {
            drop(stream_buf);
            Box::new(stream)
        };

        Ok(Self {
            stream,
            response_buf,
            mime,
            auth,
            chunking,
            dsn,
            pipelining,
            size: NonZeroUsize::new(size),
            starttls,
            smtputf8,
        })
    }
}