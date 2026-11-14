use crate::{auth, events, health, members, notifications, places, spaces, todos};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    info(title = "Horizon API", version = "0.1.0"),
    paths(
        health::liveness,
        health::readiness,
        auth::controllers::request_magic_link,
        auth::controllers::redeem_magic_link,
        auth::controllers::refresh_token,
        auth::controllers::revoke_current_session,
        auth::controllers::current_account,
        notifications::controllers::list_notifications,
        notifications::controllers::mark_notification_read,
        notifications::controllers::mark_all_notifications_read,
        notifications::controllers::notification_preferences,
        notifications::controllers::update_notification_preferences,
        spaces::controllers::list_spaces,
        spaces::controllers::create_shared_space,
        spaces::controllers::transfer_space_ownership,
        spaces::controllers::delete_space,
        spaces::controllers::list_deleted_spaces,
        spaces::controllers::restore_space,
        events::controllers::list_events,
        events::controllers::list_occurrences,
        events::controllers::create_event,
        events::controllers::update_event,
        events::controllers::update_occurrence,
        events::controllers::cancel_occurrence,
        events::controllers::delete_event,
        todos::controllers::list_tasks,
        todos::controllers::create_task_list,
        todos::controllers::update_task_list,
        todos::controllers::delete_task_list,
        todos::controllers::create_task,
        todos::controllers::update_task,
        todos::controllers::delete_task,
        todos::controllers::update_task_completion,
        todos::controllers::update_my_task_reminders,
        places::controllers::list_places,
        places::controllers::create_place,
        members::list_members,
        members::update_member_role,
        members::remove_member,
        members::list_space_invitations,
        members::create_invitation,
        members::revoke_invitation,
        members::list_my_invitations,
        members::accept_invitation,
        members::join_links::create_join_link,
        members::join_links::list_join_links,
        members::join_links::revoke_join_link,
        members::join_links::request_join_access,
        members::join_links::list_my_join_requests,
        members::join_links::list_pending_join_requests,
        members::join_links::decide_join_request
    ),
    components(schemas(
        auth::AccountResponse,
        auth::TokenPairResponse,
        auth::AuthenticationResponse,
        auth::MagicLinkRequest,
        auth::MagicLinkAccepted,
        auth::RedeemMagicLinkRequest,
        auth::Problem,
        notifications::NotificationResponse,
        notifications::NotificationPageResponse,
        notifications::NotificationPreferenceResponse,
        notifications::NotificationSettingsResponse,
        notifications::UpdateNotificationReadRequest,
        notifications::UpdateNotificationPreferencesRequest,
        spaces::CreateSpaceRequest,
        spaces::SpaceResponse,
        spaces::TransferOwnershipRequest,
        spaces::TransferOwnershipResponse,
        spaces::DeletedSpaceResponse,
        events::CreateEventRequest,
        events::EventResponse,
        events::EventPageResponse,
        todos::CreateTaskListRequest,
        todos::CreateTaskRequest,
        todos::UpdateTaskCompletionRequest,
        todos::UpdateTaskListRequest,
        todos::UpdateTaskRequest,
        todos::TaskListResponse,
        todos::TaskResponse,
        todos::TaskBoardResponse,
        places::CreatePlaceRequest,
        places::PlaceResponse,
        places::PlacePageResponse,
        members::SpaceMemberResponse,
        members::SpaceInvitationResponse,
        members::CreateInvitationRequest,
        members::UpdateMemberRoleRequest,
        members::CreateJoinLinkRequest,
        members::JoinLinkResponse,
        members::JoinRequestResponse,
        members::JoinRequestDecision
    )),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::new);
        components.add_security_scheme(
            "accessBearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
        components.add_security_scheme(
            "refreshBearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("Opaque refresh token")
                    .build(),
            ),
        );
    }
}
