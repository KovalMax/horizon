use async_trait::async_trait;
use lettre::{
    message::{Mailbox, MultiPart},
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

fn escape_email_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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
        let plain = format!(
            "Use this link to sign in to Horizon. It expires in 30 minutes.\n\n{link}{code_instructions}\n\nIf you did not request this sign-in email, you can ignore it."
        );
        let html_code_instructions = if code.is_empty() {
            String::new()
        } else {
            // Keep the visual gap out of the text so both groups can be copied together.
            let display_code = match code.split_at_checked(4) {
                Some((first, last)) if first.len() == last.len() => format!(
                    r#"<span style="padding-right:0.45em">{}</span><span>{}</span>"#,
                    escape_email_html(first),
                    escape_email_html(last),
                ),
                _ => escape_email_html(code),
            };
            format!(
                r##"
                <tr><td style="padding:24px 24px 0">
                  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" bgcolor="#edf4ee">
                    <tr><td style="padding:16px">
                      <strong>Sign-in code</strong><br>
                      Using the Horizon Home Screen app? Return to the app and enter this code instead.
                    </td></tr>
                    <tr><td style="padding:0 16px 12px">
                      <code style="font-family:Consolas,'Courier New',monospace;font-size:28px"><strong>{display_code}</strong></code>
                    </td></tr>
                    <tr><td style="padding:0 16px 16px;color:#455b52;font-size:14px">Use either the link or the code, not both.</td></tr>
                  </table>
                </td></tr>"##
            )
        };
        let html_link = escape_email_html(link);
        let logo = url::Url::parse(link)
            .ok()
            .filter(|url| matches!(url.scheme(), "http" | "https"))
            .and_then(|url| url.join("/horizon-touch-180.png").ok())
            .map(|url| {
                format!(
                    r#"<td style="padding-right:8px"><img src="{}" width="30" height="30" alt=""></td>"#,
                    escape_email_html(url.as_str()),
                )
            })
            .unwrap_or_default();
        let html = format!(
            r##"<!doctype html>
            <html lang="en">
              <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="format-detection" content="telephone=no">
                <title>Your Horizon sign-in link</title>
              </head>
              <body>
                <table role="presentation" width="100%" cellpadding="0" cellspacing="0" bgcolor="#f7f8f4">
                  <tr><td align="center" style="padding:24px 12px">
                    <table role="presentation" align="center" width="100%" cellpadding="0" cellspacing="0" bgcolor="#ffffff" style="max-width:520px;border:1px solid #dbe5dc;color:#24352f;font-family:Arial,sans-serif;font-size:16px">
                      <tr><td style="padding:24px 24px 0">
                        <table role="presentation" cellpadding="0" cellspacing="0">
                          <tr>
                            {logo}
                            <td><strong>Horizon</strong></td>
                          </tr>
                        </table>
                        <h1 style="font-size:26px">Sign in to Horizon</h1>
                        <p>Use this link to sign in. It expires in 30 minutes.</p>
                        <table role="presentation" cellpadding="0" cellspacing="0">
                          <tr><td bgcolor="#365d50" style="padding:12px 20px">
                            <a href="{html_link}" style="color:#ffffff"><strong>Sign in to Horizon</strong></a>
                          </td></tr>
                        </table>
                      </td></tr>
                      {html_code_instructions}
                      <tr><td style="padding:24px;color:#455b52;font-size:14px">If you did not request this sign-in email, you can ignore it.</td></tr>
                    </table>
                  </td></tr>
                </table>
              </body>
            </html>"##
        );
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject("Your Horizon sign-in link")
            .multipart(MultiPart::alternative_plain_html(plain, html))
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
