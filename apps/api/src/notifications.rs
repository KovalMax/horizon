pub mod controllers;
mod cursor;
mod push;
pub(crate) mod push_delivery;
mod repositories;
mod requests;
mod responses;
mod services;

pub use controllers::{
    list_notifications, mark_all_notifications_read, mark_notification_read,
    notification_preferences, push_config, push_subscription_status, register_push_subscription,
    remove_push_subscription, update_notification_preferences,
};
pub use push::{PushConfig, PushPreferences, PutPushSubscription, RemovePushSubscription};
pub use push_delivery::PushWorker;
pub use requests::{
    NotificationPreferenceInput, UpdateNotificationPreferencesRequest,
    UpdateNotificationReadRequest,
};
pub use responses::{
    NotificationPageResponse, NotificationPreferenceResponse, NotificationResponse,
    NotificationSettingsResponse,
};
pub use services::{NotificationError, NotificationsApplication};

use sqlx::PgConnection;
use uuid::Uuid;

pub(crate) struct NotificationContent<'a> {
    pub event_type: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub destination: &'a str,
}

pub(crate) async fn enqueue_notification(
    connection: &mut PgConnection,
    space_id: Uuid,
    actor_id: Uuid,
    recipient_id: Uuid,
    content: &NotificationContent<'_>,
) -> Result<(), sqlx::Error> {
    if recipient_id == actor_id {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO notifications \
         (id, recipient_id, space_id, event_type, title, body, destination) \
         SELECT $1, $2, $3, $4, $5, $6, $7 \
         WHERE COALESCE((SELECT enabled FROM notification_preferences \
                         WHERE user_id = $2 AND event_type = $4 AND channel = 'in_app'), TRUE) \
           AND NOT EXISTS (SELECT 1 FROM space_notification_preferences \
                           WHERE user_id = $2 AND space_id = $3 AND muted)",
    )
    .bind(Uuid::new_v4())
    .bind(recipient_id)
    .bind(space_id)
    .bind(content.event_type)
    .bind(content.title)
    .bind(content.body)
    .bind(content.destination)
    .execute(connection)
    .await?;
    Ok(())
}

pub(crate) async fn enqueue_space_members(
    connection: &mut PgConnection,
    space_id: Uuid,
    actor_id: Uuid,
    content: &NotificationContent<'_>,
) -> Result<(), sqlx::Error> {
    let members = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM space_memberships WHERE space_id = $1 AND user_id <> $2",
    )
    .bind(space_id)
    .bind(actor_id)
    .fetch_all(&mut *connection)
    .await?;
    for member_id in members {
        enqueue_notification(connection, space_id, actor_id, member_id, content).await?;
    }
    Ok(())
}

pub(crate) async fn enqueue_space_owners(
    connection: &mut PgConnection,
    space_id: Uuid,
    actor_id: Uuid,
    content: &NotificationContent<'_>,
) -> Result<(), sqlx::Error> {
    let owners = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM space_memberships WHERE space_id = $1 AND role = 'owner' AND user_id <> $2",
    )
    .bind(space_id)
    .bind(actor_id)
    .fetch_all(&mut *connection)
    .await?;
    for owner_id in owners {
        enqueue_notification(connection, space_id, actor_id, owner_id, content).await?;
    }
    Ok(())
}
