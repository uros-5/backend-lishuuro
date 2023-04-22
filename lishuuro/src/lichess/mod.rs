use std::env;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::database::redis::CookieValue;

pub mod login;
pub mod login_helpers;

#[derive(Debug)]
/// Token returned by lichess server.
pub struct PostLoginToken {
    pub code: String,
    pub code_verifier: String,
}

impl PostLoginToken {
    /// Creating new lichess user token.
    pub fn new(code_verifier: &String, code: &String) -> Self {
        PostLoginToken {
            code: code.to_string(),
            code_verifier: code_verifier.to_string(),
        }
    }

    /// Function used to post.
    pub fn to_json(&self, prod: bool) -> Value {
        let uri = curr_url(prod);
        let uri = format!("{}/callback", uri.0);

        serde_json::json!({
            "grant_type": "authorization_code",
            "redirect_uri": uri.as_str(),
            "client_id": "lishuuro",
            "code": self.code,
            "code_verifier": self.code_verifier
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// Lichess user token.
pub struct Token {
    pub access_token: String,
    pub expires_in: i32,
}

#[derive(Debug, Serialize, Deserialize)]
/// Lichess user login data.
pub struct LoginData {
    id: String,
    pub username: String,
}

impl Default for Token {
    fn default() -> Self {
        Token {
            access_token: String::from(""),
            expires_in: 0,
        }
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct MyKey {
    pub prod: bool,
    pub login_state: String,
}

impl Default for MyKey {
    fn default() -> Self {
        if let Ok(login_state) = env::var("LOGIN_STATE") {
            if let Ok(prod) = env::var("PROD") {
                if let Ok(prod) = prod.parse::<bool>() {
                    return MyKey { prod, login_state };
                }
            }
        }
        MyKey {
            prod: false,
            login_state: String::from("JFvbcx4bg"),
        }
    }
}

/// My server url.
pub fn curr_url(prod: bool) -> (&'static str, &'static str) {
    if prod {
        ("https://lishuuro.org/w", "https://lishuuro.org")
    } else {
        ("http://localhost:8080", "http://localhost:5173")
    }
}

pub fn cookies(prod: bool) -> CookieValue {
    if prod {
        CookieValue::new("None", "true", "true")
    } else {
        CookieValue::new("Lax", "/", "/")
    }
}
