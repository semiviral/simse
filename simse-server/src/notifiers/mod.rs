use anyhow::Result;
use once_cell::sync::OnceCell;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{trace, warn};

pub mod smtp;

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

pub fn spawn_notifier() -> Result<()> {
    let (sender, reciever) = tokio::sync::mpsc::channel(16);

    if NOTIFICATIONS.set(sender).is_err() {
        anyhow::bail!("notification thread has already been spawned");
    }

    tokio::spawn(notifier_loop(reciever));

    Ok(())
}

async fn notifier_loop(mut reciever: Receiver<Notification>) -> Result<()> {
    let smtp = OnceCell::new();
    if let Some(smtp_config) = &crate::get_config().notifiers.smtp {
        use lettre::{
            message::header::ContentType,
            transport::smtp::{authentication::Credentials, AsyncSmtpTransport},
            Message, Tokio1Executor,
        };

        let smtp_notifier: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_config.host)
                .unwrap()
                .port(smtp_config.port)
                .credentials(Credentials::new(
                    smtp_config.username.clone(),
                    smtp_config.password.as_ref().unwrap().clone(),
                ))
                .build();

        let message_template = Message::builder()
            .from(smtp_config.from.clone())
            .to(smtp_config.to.clone())
            .header(ContentType::TEXT_PLAIN);

        let subject_template = smtp_config.subject.clone();

        let timeout = Duration::from_secs(smtp_config.timeout);

        smtp.set((smtp_notifier, message_template, subject_template, timeout))
            .map_err(|_| ())
            .unwrap();
    }

    loop {
        let Some(notification) = reciever.recv().await else { break };

        if let Some((smtp_notifier, message_template, subject_tempalate, timeout)) = smtp.get() {
            use lettre::AsyncTransport;

            let message = message_template
                .clone()
                .subject(crate::notifiers::smtp::format_subject_title(
                    subject_tempalate,
                    &notification.title,
                ))
                .body(notification.body)
                .unwrap();

            match tokio::time::timeout(*timeout, smtp_notifier.send(message)).await {
                Ok(Err(err)) => warn!("Failed to send SMTP notification: {:?}", err),
                Err(_) => warn!("Failed to send SMTP notification: timeout elapsed"),
                _ => trace!("Notification successfully delivered via SMTP."),
            }
        }
    }

    tracing::info!("Notifier channels closed; task closing.");

    Ok(())
}
