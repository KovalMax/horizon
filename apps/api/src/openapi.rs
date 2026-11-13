use crate::{auth, events, health, members, places, spaces, todos};
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
        spaces::controllers::list_spaces,
        spaces::controllers::create_shared_space,
        events::controllers::list_events,
        events::controllers::create_event,
        events::controllers::update_event,
        events::controllers::delete_event,
        todos::controllers::list_tasks,
        todos::controllers::create_task_list,
        todos::controllers::update_task_list,
        todos::controllers::delete_task_list,
        todos::controllers::create_task,
        todos::controllers::update_task,
        todos::controllers::delete_task,
        todos::controllers::update_task_completion,
        places::controllers::list_places,
        places::controllers::create_place,
        members::list_members,
        members::list_space_invitations,
        members::create_invitation,
        members::list_my_invitations,
        members::accept_invitation
    ),
    components(schemas(
        auth::AccountResponse,
        auth::TokenPairResponse,
        auth::AuthenticationResponse,
        auth::MagicLinkRequest,
        auth::MagicLinkAccepted,
        auth::RedeemMagicLinkRequest,
        auth::Problem,
        spaces::CreateSpaceRequest,
        spaces::SpaceResponse,
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
        members::CreateInvitationRequest
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
