use super::{
    models::AuthError,
    requests::{MagicLinkRequest, RedeemMagicLinkRequest},
    responses::{
        AccountResponse, AuthenticationResponse, MagicLinkAccepted, Problem, TokenPairResponse,
    },
    services::AuthApplication,
};
use actix_web::{
    delete, get,
    http::{header, StatusCode},
    post, web, HttpRequest, HttpResponse,
};
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

fn request_ip(request: &HttpRequest) -> String {
    let connection_info = request.connection_info();
    let candidate = connection_info
        .realip_remote_addr()
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim();
    SocketAddr::from_str(candidate)
        .map(|address| address.ip().to_string())
        .or_else(|_| IpAddr::from_str(candidate).map(|address| address.to_string()))
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn problem(status: StatusCode, title: &'static str, code: &'static str) -> HttpResponse {
    HttpResponse::build(status)
        .content_type("application/problem+json")
        .json(Problem {
            r#type: "about:blank",
            title,
            status: status.as_u16(),
            code,
        })
}

fn rate_limited(retry_after_seconds: i64) -> HttpResponse {
    HttpResponse::TooManyRequests()
        .insert_header((header::RETRY_AFTER, retry_after_seconds.to_string()))
        .content_type("application/problem+json")
        .json(Problem {
            r#type: "about:blank",
            title: "Too many requests",
            status: StatusCode::TOO_MANY_REQUESTS.as_u16(),
            code: "rate_limited",
        })
}

fn unauthorized() -> HttpResponse {
    problem(
        StatusCode::UNAUTHORIZED,
        "Authentication failed",
        "authentication_failed",
    )
}

fn bearer_token(request: &HttpRequest) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

#[utoipa::path(
    post,
    path = "/auth/magic-links",
    request_body = MagicLinkRequest,
    responses(
        (status = 202, description = "Request accepted without revealing account existence", body = MagicLinkAccepted),
        (status = 400, description = "Invalid email address", body = Problem, content_type = "application/problem+json"),
        (status = 429, description = "Magic-link request limit exceeded", body = Problem, content_type = "application/problem+json"),
        (status = 503, description = "Email delivery is temporarily unavailable", body = Problem, content_type = "application/problem+json")
    )
)]
#[post("/auth/magic-links")]
pub async fn request_magic_link(
    state: web::Data<AuthApplication>,
    request: HttpRequest,
    body: web::Json<MagicLinkRequest>,
) -> HttpResponse {
    let ip = request_ip(&request);
    match state
        .request_magic_link(&body.email, &ip, body.time_zone.as_deref())
        .await
    {
        Ok(()) => HttpResponse::Accepted().json(MagicLinkAccepted {
            message: "If the address can receive mail, a sign-in link will be sent.".to_owned(),
        }),
        Err(AuthError::InvalidRequest) => {
            problem(StatusCode::BAD_REQUEST, "Invalid request", "invalid_email")
        }
        Err(AuthError::RateLimited {
            retry_after_seconds,
        }) => rate_limited(retry_after_seconds),
        Err(AuthError::EmailDelivery) => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Email delivery unavailable",
            "email_delivery_failed",
        ),
        Err(_) => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    post,
    path = "/auth/magic-links/redeem",
    request_body = RedeemMagicLinkRequest,
    responses(
        (status = 200, description = "Magic link redeemed and session created", body = AuthenticationResponse),
        (status = 400, description = "Invalid request", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "Magic link is invalid, expired, or used", body = Problem, content_type = "application/problem+json"),
        (status = 503, description = "Service unavailable", body = Problem, content_type = "application/problem+json")
    )
)]
#[post("/auth/magic-links/redeem")]
pub async fn redeem_magic_link(
    state: web::Data<AuthApplication>,
    body: web::Json<RedeemMagicLinkRequest>,
) -> HttpResponse {
    match state.redeem_magic_link(&body.token).await {
        Ok(session) => HttpResponse::Ok().json(AuthenticationResponse::from(session)),
        Err(AuthError::Unauthorized) => unauthorized(),
        Err(AuthError::Internal) => problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error",
            "token_issue_failed",
        ),
        Err(_) => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    post,
    path = "/auth/token/refresh",
    security(("refreshBearer" = [])),
    responses(
        (status = 200, description = "Refresh token rotated", body = TokenPairResponse),
        (status = 401, description = "Refresh token is invalid, expired, revoked, or reused", body = Problem, content_type = "application/problem+json"),
        (status = 503, description = "Service unavailable", body = Problem, content_type = "application/problem+json")
    )
)]
#[post("/auth/token/refresh")]
pub async fn refresh_token(
    state: web::Data<AuthApplication>,
    request: HttpRequest,
) -> HttpResponse {
    let Some(token) = bearer_token(&request) else {
        return unauthorized();
    };
    match state.refresh_token(token).await {
        Ok(tokens) => HttpResponse::Ok().json(TokenPairResponse::from(tokens)),
        Err(AuthError::Unauthorized) => unauthorized(),
        Err(AuthError::Internal) => problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error",
            "token_issue_failed",
        ),
        Err(_) => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    delete,
    path = "/auth/session/current",
    security(("refreshBearer" = [])),
    responses(
        (status = 204, description = "Current refresh Session revoked"),
        (status = 401, description = "Refresh token is invalid", body = Problem, content_type = "application/problem+json"),
        (status = 503, description = "Service unavailable", body = Problem, content_type = "application/problem+json")
    )
)]
#[delete("/auth/session/current")]
pub async fn revoke_current_session(
    state: web::Data<AuthApplication>,
    request: HttpRequest,
) -> HttpResponse {
    let Some(token) = bearer_token(&request) else {
        return unauthorized();
    };
    match state.revoke_current_session(token).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(AuthError::Unauthorized) => unauthorized(),
        Err(_) => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}

#[utoipa::path(
    get,
    path = "/account/me",
    security(("accessBearer" = [])),
    responses(
        (status = 200, description = "Authenticated account", body = AccountResponse),
        (status = 401, description = "Access token is invalid or expired", body = Problem, content_type = "application/problem+json"),
        (status = 503, description = "Service unavailable", body = Problem, content_type = "application/problem+json")
    )
)]
#[get("/account/me")]
pub async fn current_account(
    state: web::Data<AuthApplication>,
    request: HttpRequest,
) -> HttpResponse {
    let Some(token) = bearer_token(&request) else {
        return unauthorized();
    };
    match state.current_account(token).await {
        Ok(account) => HttpResponse::Ok().json(AccountResponse::from(account)),
        Err(AuthError::Unauthorized) => unauthorized(),
        Err(_) => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service unavailable",
            "database_unavailable",
        ),
    }
}
