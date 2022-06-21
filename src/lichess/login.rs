
use base64::encode;
use rand::Rng;
use sha2::digest::generic_array::typenum::U32;
use sha2::{digest::generic_array::GenericArray, Digest, Sha256};
use url::Url;
use reqwest;

use crate::lichess::model::{LoginData, PostLoginToken, Token};
use crate::models::redis::UserSession;

use super::model::curr_url;

fn sha256(buffer: String) -> GenericArray<u8, U32> {
    let mut hasher = Sha256::new();
    hasher.update(buffer.as_bytes());
    let result = hasher.finalize();
    result
}

pub fn base64_encode<T: AsRef<[u8]>>(s: T) -> String {
    encode(s)
        .replace("+", "-")
        .replace("/", "_")
        .replace("=", "")
}

pub fn create_verifier() -> String {
    let random_bytes = rand::thread_rng().gen::<[u8; 32]>();
    base64_encode(random_bytes)
}

pub fn create_challenge(verifier: &String) -> String {
    base64_encode(sha256(verifier.clone()))
}

/// Start of login process.
pub fn login_url(login_state: &String, prod: bool) -> (Url, String) {
    let url = "https://lichess.org/oauth?";
    let verifier: String = create_verifier();
    let challenge: String = create_challenge(&verifier);
    let mut final_url = Url::parse(url).unwrap();
    let r = format!("{}/callback", curr_url(prod).0);

    let queries = [
        ("state", login_state.as_str()),
        ("response_type", "code"),
        ("client_id", "lishuuro"),
        (
            "redirect_uri",
            &r
        ),
        ("code_challenge", &challenge[..]),
        ("code_challenge_method", "S256"),
    ];
    for i in queries {
        final_url.query_pairs_mut().append_pair(i.0, i.1);
    }

    (final_url, verifier)
}

/// If anon then generate random username.
pub fn random_username() -> String {
    format!(
        "Anon-{}",
        encode(rand::thread_rng().gen::<[u8; 7]>())
            .replace("+", "")
            .replace("/", "")
            .replace("=", "")
    )
}

/// Getting lichess token.
pub async fn get_lichess_token(session: &UserSession, code: &str, prod: bool) -> Token {
    let url = "https://lichess.org/api/token";
    let code_verifier = String::from(&session.code_verifier);
    let body = PostLoginToken::new(code_verifier, code);
    let client = reqwest::Client::new(); 
    
    let res = client.post(url).json(&body.to_json(prod)).send().await;
    if let Ok(mut i) = res {
        let json = i.json::<Token>().await;

        if let Ok(tok) = json {
            return tok;
        }
    }
    else {
    }
    return Token::default();
}

/// If user exist then we have login data.
pub async fn get_lichess_user(token: String) -> String {
    let url = "https://lichess.org/api/account";
    let client = reqwest::Client::new(); 
    let res = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;
    if let Ok(mut i) = res {
        let json = i.json::<LoginData>().await;
        if let Ok(data) = json {
            return String::from(data.username);
        }
    }
    String::from("")
}