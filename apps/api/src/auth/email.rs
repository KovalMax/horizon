use async_trait::async_trait;
use lettre::{
    message::Mailbox,
    transport::smtp::{authentication::Credentials, AsyncSmtpTransport},
    AsyncTransport, Message, Tokio1Executor,
};
use std::env;

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_magic_link(&self, recipient: &str, link: &str) -> Result<(), String>;

    async fn send_task_reminder(
        &self,
        recipient: &str,
        task_title: &str,
        space_name: &str,
        due: &str,
    ) -> Result<(), String> {
        let _ = (recipient, task_title, space_name, due);
        Err("task reminders are not supported by this email sender".to_owned())
    }

    async fn send_space_invitation(
        &self,
        recipient: &str,
        space_name: &str,
        link: &str,
    ) -> Result<(), String> {
        let _ = (recipient, space_name, link);
        Err("space invitations are not supported by this email sender".to_owned())
    }
}

pub struct SmtpEmailSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpEmailSender {
    pub fn from_env() -> Result<Self, String> {
        let host = env::var("SMTP_HOST").map_err(|_| "SMTP_HOST must be configured".to_owned())?;
        let port = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "1025".to_owned())
            .parse::<u16>()
            .map_err(|_| "SMTP_PORT must be a valid port".to_owned())?;
        let from = env::var("SMTP_FROM")
            .unwrap_or_else(|_| "Horizon <no-reply@horizon.localhost>".to_owned())
            .parse::<Mailbox>()
            .map_err(|_| "SMTP_FROM must be a valid email mailbox".to_owned())?;
        let use_tls = env::var("SMTP_TLS")
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let builder = if use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
                .map_err(|_| "SMTP_HOST is invalid".to_owned())?
                .port(port)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host).port(port)
        };

        let username = env::var("SMTP_USERNAME")
            .ok()
            .filter(|value| !value.is_empty());
        let password = env::var("SMTP_PASSWORD")
            .ok()
            .filter(|value| !value.is_empty());
        let builder = match (username, password) {
            (Some(username), Some(password)) => {
                builder.credentials(Credentials::new(username, password))
            }
            (None, None) => builder,
            _ => return Err("SMTP_USERNAME and SMTP_PASSWORD must be set together".to_owned()),
        };

        Ok(Self {
            transport: builder.build(),
            from,
        })
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send_magic_link(&self, recipient: &str, link: &str) -> Result<(), String> {
        let recipient = recipient
            .parse::<Mailbox>()
            .map_err(|_| "recipient email is invalid".to_owned())?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject("Your Horizon sign-in link")
            .body(format!(
                "Use this link to sign in to Horizon. It expires in 30 minutes.\n\n{link}\n\nIf you did not request this link, you can ignore this email."
            ))
            .map_err(|_| "could not create sign-in email".to_owned())?;

        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|_| "email delivery failed".to_owned())
    }

    async fn send_task_reminder(
        &self,
        recipient: &str,
        task_title: &str,
        space_name: &str,
        due: &str,
    ) -> Result<(), String> {
        let recipient = recipient
            .parse::<Mailbox>()
            .map_err(|_| "recipient email is invalid".to_owned())?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject(format!("Reminder: {task_title}"))
            .body(format!(
                "A Task in {space_name} is due {due}.\n\n{task_title}\n\nSign in to Horizon to view the Task."
            ))
            .map_err(|_| "could not create task reminder email".to_owned())?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|_| "email delivery failed".to_owned())
    }

    async fn send_space_invitation(
        &self,
        recipient: &str,
        space_name: &str,
        link: &str,
    ) -> Result<(), String> {
        let recipient = recipient
            .parse::<Mailbox>()
            .map_err(|_| "recipient email is invalid".to_owned())?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject(format!("You're invited to {space_name} on Horizon"))
            .body(format!(
                "You have been invited to join {space_name} on Horizon as a Viewer.\n\nSign in with this email address to accept the invitation: {link}\n\nThis invitation expires in 7 days."
            ))
            .map_err(|_| "could not create invitation email".to_owned())?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|_| "email delivery failed".to_owned())
    }
}
