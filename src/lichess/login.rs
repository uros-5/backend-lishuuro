use reqwest::{Client, Url};

use super::{
    curr_url,
    login_helpers::{create_challenge, create_verifier},
    LoginData, PostLoginToken, Token,
};
use base64::encode;
use rand::Rng;

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
        ("redirect_uri", &r),
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
        encode(rand::thread_rng().gen::<[u8; 6]>())
            .replace("+", "")
            .replace("/", "")
            .replace("=", "")
    )
}

/// Generate random game id.
pub fn random_game_id() -> String {
    format!(
        "{}",
        encode(rand::thread_rng().gen::<[u8; 10]>())
            .replace("+", "")
            .replace("/", "")
            .replace("=", "")
    )
}

/// Getting lichess token.
pub async fn get_lichess_token(code: &String, code_verifier: &String, prod: bool) -> Token {
    let url = "https://lichess.org/api/token";
    let body = PostLoginToken::new(&code_verifier, code);
    let body = body.to_json(prod);
    let client = Client::default();
    let req = client.post(url).json(&body).send();
    if let Ok(i) = req.await {
        let json = i.json::<Token>().await;

        if let Ok(tok) = json {
            return tok;
        }
    }
    return Token::default();
}

/// If user exist then we have login data.
pub async fn get_lichess_user(token: String) -> String {
    let url = "https://lichess.org/api/account";
    let client = Client::default();
    let res = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;
    if let Ok(i) = res {
        let json = i.json::<LoginData>().await;
        if let Ok(data) = json {
            return String::from(data.username);
        }
    }
    String::from("")
}
