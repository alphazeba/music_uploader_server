use std::collections::HashMap;

use rocket::{
    http,
    request::{self, FromRequest},
    Request,
};
use rocket_basicauth::BasicAuth;
use thiserror::Error;

use crate::config::secrets_config::load_users;

pub struct Authenticated {
    pub username: String,
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("something went wrong")]
    FailedToAuthorize,
    #[error("server config issue")]
    FailedToGetConfig,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = AuthError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match Self::from_request_inner(req).await {
            Ok(a) => request::Outcome::Success(a),
            Err(e) => request::Outcome::Error((http::Status::Unauthorized, e)),
        }
    }
}

impl<'r> Authenticated {
    async fn from_request_inner(req: &'r Request<'_>) -> Result<Self, AuthError> {
        let authenticator = Self::get_authenticator(req)?;
        let user_auth = Self::get_incoming_basic_auth(req).await?;
        match authenticator.is_authenticated(&user_auth) {
            true => Ok(Authenticated {username: user_auth.username}),
            false => Err(AuthError::FailedToAuthorize),
        }
    }

    async fn get_incoming_basic_auth(req: &'r Request<'_>) -> Result<BasicAuth, AuthError> {
        match req.guard::<BasicAuth>().await {
            request::Outcome::Success(a) => Ok(a),
            _ => Err(AuthError::FailedToAuthorize),
        }
    }

    fn get_authenticator(req: &'r Request<'_>) -> Result<&'r Authenticator, AuthError> {
        match req.rocket().state::<Authenticator>() {
            Some(x) => Ok(x),
            _ => Err(AuthError::FailedToGetConfig),
        }
    }
}

pub struct Authenticator {
    users: HashMap<String, String>,
}

impl Authenticator {
    pub fn new() -> Result<Self, AuthError> {
        let users = load_users(&"./Secrets.toml".to_string());
        Ok(Authenticator {
            users: users.users.into_iter()
                .map(|user| (user.username, user.password))
                .collect(),
        })
    }

    fn is_authenticated(&self, auth: &BasicAuth) -> bool {
        let fake_password= "fake password".to_string();
        let testing_password = self.users
            .get(&auth.username)
            .unwrap_or(&fake_password);
        let user_was_found = self.users.contains_key(&auth.username);
        let password_is_correct = Self::compare_str(
            testing_password, &auth.password);
        user_was_found && password_is_correct
    }

    fn compare_str(a: &String, b: &String) -> bool {
        a.trim() == b.trim()
    }
}


