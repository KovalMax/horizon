pub mod controllers;
mod email;
mod factories;
mod jwt_tokens;
mod models;
mod repositories;
mod requests;
mod responses;
mod services;
mod token_service;

pub use controllers::{
    current_account, redeem_magic_link, refresh_token, request_magic_link, revoke_current_session,
};
pub use email::{EmailSender, SmtpEmailSender};
pub use factories::AuthSettings;
pub use models::{AccessPrincipal, Account, AuthError, AuthenticatedSession, TokenSet};
pub use repositories::{AuthRepository, PostgresAuthRepository};
pub use requests::{MagicLinkRequest, RedeemMagicLinkRequest};
pub use responses::{
    AccountResponse, AuthenticationResponse, MagicLinkAccepted, Problem, TokenPairResponse,
};
// Compatibility names preserve the public response vocabulary used by existing consumers.
pub type AuthenticationResult = AuthenticationResponse;
pub type TokenPair = TokenPairResponse;
pub use services::AuthApplication;
pub use token_service::TokenService;
pub type AuthState = AuthApplication;
