use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Datelike, Utc};
use uuid::Uuid;

pub(super) struct NotificationCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

impl NotificationCursor {
    pub(super) fn decode(value: &str) -> Option<Self> {
        let mut bytes = [0; 24];
        if value.len() != 32 || URL_SAFE_NO_PAD.decode_slice(value, &mut bytes).ok()? != 24 {
            return None;
        }
        let created_at =
            DateTime::from_timestamp_micros(i64::from_be_bytes(bytes[..8].try_into().ok()?))?;
        if !(1..=9999).contains(&created_at.year()) {
            return None;
        }
        Some(Self {
            created_at,
            id: Uuid::from_slice(&bytes[8..]).ok()?,
        })
    }

    pub(super) fn encode(created_at: DateTime<Utc>, id: Uuid) -> String {
        let mut bytes = [0; 24];
        bytes[..8].copy_from_slice(&created_at.timestamp_micros().to_be_bytes());
        bytes[8..].copy_from_slice(id.as_bytes());
        URL_SAFE_NO_PAD.encode(bytes)
    }
}
