use actix_web::{dev::Service as _, middleware::Logger, web, App, HttpServer};
#[cfg(debug_assertions)]
use horizon_api::openapi::ApiDoc;
use horizon_api::{
    auth, database_connection_options, events, health, logging, members, notifications, places,
    spaces, task_reminders, todos,
};
use sqlx::postgres::PgPoolOptions;
use std::{env, time::Duration};
#[cfg(debug_assertions)]
use utoipa::OpenApi;
#[cfg(debug_assertions)]
use utoipa_swagger_ui::SwaggerUi;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::init();
    log::info!(target: "horizon_api", "API process starting");

    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(database_connection_options());
    if env::args().nth(1).as_deref() == Some("migrate") {
        sqlx::migrate!("./migrations").run(&pool).await?;
        return Ok(());
    }
    if env::var("HORIZON_PROFILE_REMINDER_BACKLOG").as_deref() == Ok("1") {
        profile_reminder_backlog(pool.clone());
    }
    let auth_state = web::Data::new(
        auth::AuthApplication::from_env(pool.clone()).map_err(std::io::Error::other)?,
    );
    let spaces_state = web::Data::new(spaces::SpacesApplication::new(pool.clone()));
    let trusted_sign_in_proxy = match env::var("AUTH_TRUSTED_PROXY_IP") {
        Ok(value) if !value.trim().is_empty() => Some(
            value
                .trim()
                .parse::<std::net::IpAddr>()
                .map_err(|_| {
                    std::io::Error::other("AUTH_TRUSTED_PROXY_IP must be a single IP address")
                })?
                .to_canonical(),
        ),
        Ok(_) | Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let notifications_state =
        web::Data::new(notifications::NotificationsApplication::new(pool.clone()));
    let reminder_worker =
        task_reminders::TaskReminderWorker::new(pool.clone(), auth_state.email_sender());
    let calendar_reminders = events::CalendarReminderWorker::new(pool.clone());
    if let Some(push_worker) = notifications::PushWorker::from_env(pool.clone()) {
        actix_web::rt::spawn(async move {
            let mut interval = actix_web::rt::time::interval(Duration::from_secs(15));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                if let Err(error) = push_worker.run_once().await {
                    log::error!("Failed to process Web Push queue: {error}");
                }
            }
        });
    } else if env::var("WEB_PUSH_PUBLIC_KEY").is_ok_and(|key| !key.is_empty()) {
        log::warn!("Web Push is disabled: check the VAPID key pair and contact email");
    }
    let purge_spaces = spaces_state.clone();
    let purge_notifications = notifications_state.clone();
    let reminders = reminder_worker.clone();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(error) = purge_spaces.purge_expired_deleted_spaces().await {
                log::error!("Failed to purge expired deleted Spaces: {error}");
            }
            if let Err(error) = purge_notifications.purge_expired().await {
                log::error!("Failed to purge expired notifications: {error}");
            }
            if let Err(error) = reminders.purge_stuck_claims().await {
                log::error!("Failed to recover stuck Task reminder jobs: {error}");
            }
        }
    });
    let task_reconciliation = reminder_worker.clone();
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if let Err(error) = task_reconciliation.reconcile_pending().await {
                log::error!("Failed to reconcile Task reminders: {error}");
            }
        }
    });
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(15));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if let Err(error) = reminder_worker.deliver_due_at(chrono::Utc::now()).await {
                log::error!("Failed to deliver Task reminders: {error}");
            }
        }
    });
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if let Err(error) = calendar_reminders.run_once_at(chrono::Utc::now()).await {
                log::error!("Failed to process Calendar reminders: {error}");
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
    let server = HttpServer::new(move || {
        let app = App::new()
            .wrap_fn(|request, service| {
                let method = request.method().clone();
                let path = request.path().to_owned();
                let future = service.call(request);
                async move {
                    let response = future.await?;
                    let status = response.status();
                    if status.is_server_error() {
                        log::error!(
                            target: "horizon_api::http",
                            "{method} {path} returned {status}"
                        );
                    }
                    Ok(response)
                }
            })
            .wrap(Logger::new("%m %U %s %Dms").log_target("horizon_api::http"))
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
            .service(auth::redeem_sign_in_code)
            .service(auth::refresh_token)
            .service(auth::revoke_current_session)
            .service(auth::current_account)
            .service(notifications::list_notifications)
            .service(notifications::mark_notification_read)
            .service(notifications::mark_all_notifications_read)
            .service(notifications::notification_preferences)
            .service(notifications::update_notification_preferences)
            .service(notifications::register_push_subscription)
            .service(notifications::remove_push_subscription)
            .service(notifications::push_config)
            .service(notifications::push_subscription_status)
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
            .service(places::preview_place_import)
            .service(places::delete_place)
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
        let app = if let Some(proxy) = trusted_sign_in_proxy {
            app.app_data(auth::controllers::TrustedSignInProxy(proxy))
        } else {
            app
        };
        #[cfg(debug_assertions)]
        let app = app
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/openapi.json", ApiDoc::openapi()));
        app
    })
    .bind(("0.0.0.0", 8000))?;
    log::info!(target: "horizon_api", "API listening on 0.0.0.0:8000");
    server.run().await?;
    Ok(())
}

fn profile_reminder_backlog(pool: sqlx::PgPool) {
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_millis(250));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let started = std::time::Instant::now();
            let acquired = pool.acquire().await.is_ok();
            let acquire_us = started.elapsed().as_micros();
            let peak_rss_kib = std::fs::read_to_string("/proc/self/status")
                .ok()
                .and_then(|status| {
                    status.lines().find_map(|line| {
                        let value = line.strip_prefix("VmHWM:")?.split_whitespace().next()?;
                        value.parse::<u64>().ok()
                    })
                })
                .unwrap_or_default();
            log::info!(
                target: "horizon_api::reminder_backlog_profile",
                "epoch_ms={} acquire_us={acquire_us} acquired={} peak_rss_kib={peak_rss_kib}",
                chrono::Utc::now().timestamp_millis(),
                u8::from(acquired),
            );
        }
    });
}
