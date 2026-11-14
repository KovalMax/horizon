use actix_web::{web, App, HttpServer};
#[cfg(debug_assertions)]
use horizon_api::openapi::ApiDoc;
use horizon_api::{
    auth, database_connection_options, events, health, members, notifications, places, spaces,
    task_reminders, todos,
};
use sqlx::postgres::PgPoolOptions;
use std::{env, time::Duration};
#[cfg(debug_assertions)]
use utoipa::OpenApi;
#[cfg(debug_assertions)]
use utoipa_swagger_ui::SwaggerUi;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(database_connection_options());
    if env::args().nth(1).as_deref() == Some("migrate") {
        sqlx::migrate!("./migrations").run(&pool).await?;
        return Ok(());
    }
    let auth_state = web::Data::new(
        auth::AuthApplication::from_env(pool.clone()).map_err(std::io::Error::other)?,
    );
    let spaces_state = web::Data::new(spaces::SpacesApplication::new(pool.clone()));
    let notifications_state =
        web::Data::new(notifications::NotificationsApplication::new(pool.clone()));
    let reminder_worker =
        task_reminders::TaskReminderWorker::new(pool.clone(), auth_state.email_sender());
    let purge_spaces = spaces_state.clone();
    let purge_notifications = notifications_state.clone();
    let reminders = reminder_worker.clone();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(error) = purge_spaces.purge_expired_deleted_spaces().await {
                eprintln!("Failed to purge expired deleted Spaces: {error}");
            }
            if let Err(error) = purge_notifications.purge_expired().await {
                eprintln!("Failed to purge expired notifications: {error}");
            }
            if let Err(error) = reminders.purge_stuck_claims().await {
                eprintln!("Failed to recover stuck Task reminder jobs: {error}");
            }
        }
    });
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(error) = reminder_worker.run_once().await {
                eprintln!("Failed to process Task reminders: {error}");
            }
        }
    });
    let events_state = web::Data::new(events::EventsApplication::new(pool.clone()));
    let todos_state = web::Data::new(todos::TodosApplication::new(pool.clone()));
    let places_state = web::Data::new(places::PlacesApplication::new(pool.clone()));
    let members_state = web::Data::new(members::MembersApplication::new(
        pool.clone(),
        auth_state.email_sender(),
        auth_state.public_url(),
    ));
    let join_links_state = web::Data::new(members::JoinLinksApplication::new(
        pool.clone(),
        auth_state.public_url(),
    ));
    HttpServer::new(move || {
        let app = App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(auth_state.clone())
            .app_data(spaces_state.clone())
            .app_data(notifications_state.clone())
            .app_data(events_state.clone())
            .app_data(todos_state.clone())
            .app_data(places_state.clone())
            .app_data(members_state.clone())
            .app_data(join_links_state.clone())
            .service(health::liveness)
            .service(health::readiness)
            .service(auth::request_magic_link)
            .service(auth::redeem_magic_link)
            .service(auth::refresh_token)
            .service(auth::revoke_current_session)
            .service(auth::current_account)
            .service(notifications::list_notifications)
            .service(notifications::mark_notification_read)
            .service(notifications::mark_all_notifications_read)
            .service(notifications::notification_preferences)
            .service(notifications::update_notification_preferences)
            .service(spaces::list_spaces)
            .service(spaces::create_shared_space)
            .service(spaces::transfer_space_ownership)
            .service(spaces::delete_space)
            .service(spaces::list_deleted_spaces)
            .service(spaces::restore_space)
            .service(events::list_events)
            .service(events::list_occurrences)
            .service(events::create_event)
            .service(events::update_event)
            .service(events::update_occurrence)
            .service(events::cancel_occurrence)
            .service(events::delete_event)
            .service(todos::list_tasks)
            .service(todos::create_task_list)
            .service(todos::update_task_list)
            .service(todos::delete_task_list)
            .service(todos::create_task)
            .service(todos::update_task)
            .service(todos::delete_task)
            .service(todos::update_task_completion)
            .service(todos::update_my_task_reminders)
            .service(places::list_places)
            .service(places::create_place)
            .service(members::list_members)
            .service(members::update_member_role)
            .service(members::remove_member)
            .service(members::list_space_invitations)
            .service(members::create_invitation)
            .service(members::revoke_invitation)
            .service(members::list_my_invitations)
            .service(members::accept_invitation);
        let app = app
            .service(members::create_join_link)
            .service(members::list_join_links)
            .service(members::revoke_join_link)
            .service(members::request_join_access)
            .service(members::list_my_join_requests)
            .service(members::list_pending_join_requests)
            .service(members::decide_join_request);
        #[cfg(debug_assertions)]
        let app = app
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/openapi.json", ApiDoc::openapi()));
        app
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await?;
    Ok(())
}
