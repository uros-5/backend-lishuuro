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
pub struct Token {
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
            access_token: String::from(""),
            expires_in: 0,
        }
    }
}

pub fn curr_url(prod: bool) -> (&'static str, &'static str)  {
    if prod {
        ("https://lishuuro.org/w", "https://lishuuro.org")
    } else {
        ("http://localhost:8080", "http://localhost:3000")
    }
}
