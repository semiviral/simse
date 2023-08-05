pub mod smtp;

use crate::config::{NotifierConfig, SmtpNotifierConfig};
use anyhow::Result;
use lettre::{
    message::{header::ContentType, Mailbox, MessageBuilder},
    transport::smtp::{authentication::Credentials, AsyncSmtpTransport},
    Message, Tokio1Executor,
};
use once_cell::sync::OnceCell;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
struct SmtpNotifier {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    message_template: MessageBuilder,
    subject_template: String,
    timeout: Duration,
}

impl SmtpNotifier {
    fn build_message(&mut self, notification: &Notification) -> Result<Message> {
        todo!("finish creating the build_message function");

        let message = self
            .message_template
            .clone()
            .to(Mailbox::new(loop {}, loop {}))
            .subject(smtp::format_subject_title(
                &self.subject_template,
                &notification.title,
            ))
            .body(notification.body.clone())?;

        Ok(message)
    }
}

#[derive(Debug)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

static NOTIFICATIONS: OnceCell<Sender<Notification>> = OnceCell::new();

pub async fn send_notification(notification: Notification, timeout: Duration) -> Result<()> {
    let notifications = NOTIFICATIONS
        .get()
        .ok_or(anyhow::anyhow!("notifier thread has not been spawned"))?;
    notifications.send_timeout(notification, timeout).await?;

    Ok(())
}

pub async fn spawn_notifier(notifiers: &NotifierConfig) -> Result<()> {
    let (sender, reciever) = tokio::sync::mpsc::channel(16);

    NOTIFICATIONS
        .set(sender)
        .expect("notification thread has already been spawned");

    let smtp_notifier = match notifiers.smtp.as_ref() {
        Some(smtp_config) => Some(build_smtp_notifier(smtp_config).await?),
        None => None,
    };

    tokio::spawn(async move {
        // Take mutable ownership of `receiver`.
        let mut reciever = reciever;
        let mut smtp_notifier = smtp_notifier;

        loop {
            let Some(notification) = reciever.recv().await else { break };

            if let Err(err) = try_send_smtp(smtp_notifier.as_mut(), &notification).await {
                warn!("SMTP notifier failed: {err:?}");
            }
        }

        tracing::info!("Notifier channels closed; task closing.");

        anyhow::Result::<(), anyhow::Error>::Ok(())
    });

    Ok(())
}

async fn try_send_smtp(
    smtp_notifier: Option<&mut SmtpNotifier>,
    notification: &Notification,
) -> Result<()> {
    use lettre::AsyncTransport;

    let Some(smtp_notifier) = smtp_notifier else { return Ok(()) };

    let message = smtp_notifier.build_message(notification)?;
    tokio::time::timeout(smtp_notifier.timeout, smtp_notifier.transport.send(message)).await??;

    trace!("Notification successfully delivered via SMTP.");

    Ok(())
}

async fn build_smtp_notifier(smtp_config: &SmtpNotifierConfig) -> anyhow::Result<SmtpNotifier> {
    debug!("Starting up SMTP notifier...");

    let passfile_path = &smtp_config.passfile;
    debug!("Reading password from: {}", passfile_path.to_string_lossy());
    let password = tokio::fs::read_to_string(passfile_path).await?;

    let smtp_notifier = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_config.host)?
        .port(smtp_config.port)
        .credentials(Credentials::new(smtp_config.username.clone(), password))
        .build();

    let message_template = Message::builder()
        .from(smtp_config.sender.clone())
        .header(ContentType::TEXT_PLAIN);

    let subject_template = smtp_config.subject.clone();
    let timeout = Duration::from_secs(smtp_config.timeout);

    Ok(SmtpNotifier {
        transport: smtp_notifier,
        message_template,
        subject_template,
        timeout,
    })
}
