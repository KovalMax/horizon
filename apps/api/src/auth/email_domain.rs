use hickory_resolver::{
    net::{DnsError, NetError, NoRecords},
    proto::{
        op::ResponseCode,
        rr::{Name, RData},
    },
    TokioResolver,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmailDomainError {
    NoMail,
    Unavailable,
}

pub fn normalize_email(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 254 || value.chars().any(char::is_whitespace) {
        return None;
    }
    let email = value.parse::<lettre::Address>().ok()?;
    let domain = Name::from_utf8(email.domain()).ok()?;
    if email.user().len() > 64
        || domain.num_labels() < 2
        || domain.iter().any(|label| {
            label.first() == Some(&b'-')
                || label.last() == Some(&b'-')
                || !label
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        })
    {
        return None;
    }
    Some(email.to_string().to_ascii_lowercase())
}

fn no_records_code(error: &NetError) -> Option<ResponseCode> {
    match error {
        NetError::Dns(DnsError::NoRecordsFound(NoRecords { response_code, .. })) => {
            Some(*response_code)
        }
        _ => None,
    }
}

pub(super) async fn check(
    resolver: &TokioResolver,
    recipient: &str,
) -> Result<(), EmailDomainError> {
    let (_, domain) = recipient.rsplit_once('@').ok_or(EmailDomainError::NoMail)?;
    // Use an absolute name so the system's DNS search suffixes cannot change the domain.
    let domain = Name::from_utf8(format!("{}.", domain.trim_end_matches('.')))
        .map_err(|_| EmailDomainError::NoMail)?;
    tokio::time::timeout(Duration::from_secs(3), async {
        match resolver.mx_lookup(domain.clone()).await {
            Ok(records) => {
                return if records
                    .answers()
                    .iter()
                    .any(|record| matches!(&record.data, RData::MX(mx) if !mx.exchange.is_root()))
                {
                    Ok(())
                } else {
                    // Null MX explicitly opts out of email, even if A/AAAA exist.
                    Err(EmailDomainError::NoMail)
                };
            }
            Err(error) => match no_records_code(&error) {
                Some(ResponseCode::NXDomain) => return Err(EmailDomainError::NoMail),
                Some(ResponseCode::NoError) => {}
                _ => return Err(EmailDomainError::Unavailable),
            },
        }

        // SMTP allows an implicit MX at the domain itself if no MX exists.
        async fn address_exists<T>(
            lookup: impl std::future::Future<Output = Result<T, NetError>>,
        ) -> Result<bool, EmailDomainError> {
            match lookup.await {
                Ok(_) => Ok(true),
                Err(error)
                    if matches!(
                        no_records_code(&error),
                        Some(ResponseCode::NoError | ResponseCode::NXDomain)
                    ) =>
                {
                    Ok(false)
                }
                Err(_) => Err(EmailDomainError::Unavailable),
            }
        }
        let v4 = address_exists(resolver.ipv4_lookup(domain.clone()));
        let v6 = address_exists(resolver.ipv6_lookup(domain));
        tokio::pin!(v4, v6);
        let (first, remaining) = tokio::select! {
            result = &mut v4 => {
                if result == Ok(true) { return Ok(()); }
                (result, v6.await)
            }
            result = &mut v6 => {
                if result == Ok(true) { return Ok(()); }
                (result, v4.await)
            }
        };
        match (first, remaining) {
            (Ok(true), _) | (_, Ok(true)) => Ok(()),
            (Ok(false), Ok(false)) => Err(EmailDomainError::NoMail),
            _ => Err(EmailDomainError::Unavailable),
        }
    })
    .await
    .unwrap_or(Err(EmailDomainError::Unavailable))
}
