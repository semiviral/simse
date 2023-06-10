mod config;
mod notifiers;

use anyhow::{Context, Result};
use config::Config;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
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
        let message = Message::builder()
            .from(smtp_notifier.from)
            .to(smtp_notifier.to)
            .subject(notifiers::smtp::format_subject_title(
                &smtp_notifier.subject,
                "Test Email",
            ))
            .header(ContentType::TEXT_PLAIN)
            .body(String::from(
                "This is a test line
This is another test line
Goodbye",
            ))
            .unwrap();

        let smtp = SmtpTransport::starttls_relay(&smtp_notifier.host)
            .unwrap()
            .port(smtp_notifier.port)
            .credentials(Credentials::new(
                smtp_notifier.username,
                smtp_notifier.password.unwrap(),
            ))
            .build();

        smtp.send(&message).unwrap();

        // let mut smtp = notifiers::smtp::Smtp::new(
        //     &smtp_notifier.host,
        //     smtp_notifier.port,
        //     smtp_notifier.tls,
        //     std::time::Duration::from_secs(smtp_notifier.timeout),
        //     smtp_notifier.from,
        //     smtp_notifier.subject,
        // )
        // .await
        // .unwrap();

        // smtp.authenticate(
        //     smtp_notifier.username.as_deref(),
        //     &smtp_notifier.password.unwrap(),
        // )
        // .await
        // .unwrap();
        // smtp.send_mail(smtp_notifier.to, None, "No Title")
        //     .await
        //     .unwrap();
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
