use serde::{Deserialize, Serialize};
use serde_json::Value;

// LICHESS MODEL

#[derive(Debug)]
pub struct PostLoginToken<'a> {
    pub code: &'a str,
    pub code_verifier: String,
}

impl<'a> PostLoginToken<'a> {
    pub fn new(code_verifier: String, code: &'a str) -> Self {
        PostLoginToken {
            code,
            code_verifier,
        }
    }

    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "grant_type": "authorization_code",
            "redirect_uri": "http://lishuuro.org/w/callback",
            "client_id": "lishuuro",
            "code": self.code,
            "code_verifier": self.code_verifier
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub token_type: String,
    pub access_token: String,
    pub expires_in: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginData {
    id: String,
    pub username: String,
}

impl Default for Token {
    fn default() -> Self {
        Token {
            token_type: String::from(""),
            access_token: String::from(""),
            expires_in: 0,
        }
    }
}
