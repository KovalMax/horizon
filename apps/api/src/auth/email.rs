use async_trait::async_trait;
use lettre::{
    message::Mailbox,
    transport::smtp::{authentication::Credentials, AsyncSmtpTransport},
    AsyncTransport, Message, Tokio1Executor,
};
use std::env;

use super::email_domain::{self, EmailDomainError};

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn validate_recipient_domain(&self, _recipient: &str) -> Result<(), EmailDomainError> {
        Ok(())
    }

    async fn send_magic_link(&self, recipient: &str, link: &str) -> Result<(), String>;

    async fn send_sign_in(&self, recipient: &str, link: &str, _code: &str) -> Result<(), String> {
        self.send_magic_link(recipient, link).await
    }

    async fn send_task_reminder(&self, _recipient: &str) -> Result<(), String> {
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
    resolver: Option<hickory_resolver::TokioResolver>,
    allow_test_domain: bool,
}

fn log_smtp_delivery_failure(kind: &str, recipient: &str, error: &lettre::transport::smtp::Error) {
    let details = error.to_string().replace(recipient, "<redacted-recipient>");
    eprintln!("SMTP {kind} delivery failed: {details}");
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
            let relay = if port == 465 {
                AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            } else {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            };
            relay
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
            resolver: hickory_resolver::TokioResolver::builder_tokio()
                .and_then(|builder| builder.build())
                .ok(),
            allow_test_domain: host == "mailpit",
        })
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn validate_recipient_domain(&self, recipient: &str) -> Result<(), EmailDomainError> {
        if self.allow_test_domain && recipient.ends_with("@example.test") {
            return Ok(());
        }
        let resolver = self
            .resolver
            .as_ref()
            .ok_or(EmailDomainError::Unavailable)?;
        email_domain::check(resolver, recipient).await
    }

    async fn send_magic_link(&self, recipient: &str, link: &str) -> Result<(), String> {
        self.send_sign_in(recipient, link, "").await
    }

    async fn send_sign_in(&self, recipient: &str, link: &str, code: &str) -> Result<(), String> {
        let recipient = recipient
            .parse::<Mailbox>()
            .map_err(|_| "recipient email is invalid".to_owned())?;
        let recipient_address = recipient.email.to_string();
        let code_instructions = if code.is_empty() {
            String::new()
        } else {
            format!("\n\nUsing the Horizon Home Screen app? Return to the app and enter this sign-in code instead:\n\n{code}\n\nUse either the link or the code, not both.")
        };
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject("Your Horizon sign-in link")
            .body(format!(
                "Use this link to sign in to Horizon. It expires in 30 minutes.\n\n{link}{code_instructions}\n\nIf you did not request this sign-in email, you can ignore it."
            ))
            .map_err(|_| "could not create sign-in email".to_owned())?;

        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|error| {
                log_smtp_delivery_failure("magic-link", &recipient_address, &error);
                "email delivery failed".to_owned()
            })
    }

    async fn send_task_reminder(&self, recipient: &str) -> Result<(), String> {
        let recipient = recipient
            .parse::<Mailbox>()
            .map_err(|_| "recipient email is invalid".to_owned())?;
        let recipient_address = recipient.email.to_string();
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject("Horizon Task reminder")
            // Queued mail cannot be recalled if Space membership changes after sending.
            .body("A Task is due soon. Sign in to Horizon to view your Tasks.".to_owned())
            .map_err(|_| "could not create task reminder email".to_owned())?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|error| {
                log_smtp_delivery_failure("task-reminder", &recipient_address, &error);
                "email delivery failed".to_owned()
            })
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
        let recipient_address = recipient.email.to_string();
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
            .map_err(|error| {
                log_smtp_delivery_failure("space-invitation", &recipient_address, &error);
                "email delivery failed".to_owned()
            })
    }
}
