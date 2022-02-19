use awc::Client;

use actix_session::Session;
use base64::encode;
use rand::Rng;
use sha2::digest::generic_array::typenum::U32;
use sha2::{digest::generic_array::GenericArray, Digest, Sha256};
use url::Url;

use crate::lichess::model::{LoginData, PostLoginToken, Token};

const LOGIN_STATE: &str = "_aXV20V_";

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

pub fn login_url() -> (Url, String) {
    let url = "https://lichess.org/oauth?";
    let verifier: String = create_verifier();
    let challenge: String = create_challenge(&verifier);
    let mut final_url = Url::parse(url).unwrap();
    let queries = [
        ("state", LOGIN_STATE),
        ("response_type", "code"),
        ("client_id", "abc"),
        (
            "redirect_uri",
            &format!("http://localhost:8080/callback")[..],
        ),
        ("code_challenge", &challenge[..]),
        ("code_challenge_method", "S256"),
    ];
    for i in queries {
        final_url.query_pairs_mut().append_pair(i.0, i.1);
    }

    (final_url, verifier)
}
pub fn random_username() -> String {
    format!("Anon-{}", encode(rand::thread_rng().gen::<[u8; 6]>()))
}

pub async fn get_lichess_token(session: &Session, code: &str) -> Token {
    let url = "https://lichess.org/api/token";
    let code_verifier = session.get::<String>("codeVerifier").ok().unwrap().unwrap();
    let body = PostLoginToken::new(code_verifier, code);
    let client = Client::default();
    let res = client.post(url).send_json(&body.to_json()).await;
    match res {
        Ok(mut i) => {
            let json = i.json::<Token>().await;
            match json {
                Ok(tok) => {
                    println!("{:?}", tok);
                    return tok;
                }
                Err(_) => {
                    return Token::default();
                }
            }
        }
        Err(_) => {
            return Token::default();
        }
    }
}

pub async fn get_lichess_user(token: String) -> String {
    let url = "https://lichess.org/api/account";
    let client = Client::default();
    let res = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;
    match res {
        Ok(mut i) => {
            let json = i.json::<LoginData>().await;
            match json {
                Ok(data) => {
                    return String::from(data.username);
                }
                Err(_) => {
                    println!("error");
                }
            }
        }
        Err(err) => {
            println!("{}", err);
        }
    }
    String::from("")
}
