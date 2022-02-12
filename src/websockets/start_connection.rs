use crate::websockets::lobby::Lobby;
use crate::{models::redis_session::*, websockets::WsConn,models::model::{AppState, User},};
use actix::Addr;
use actix_session::Session;
use actix_web::{get, web, web::Data, web::Payload, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::sync::Mutex;
use crate::login_lichess::random_username;
use crate::login_lichess::create_verifier;
#[get("/ws/")]
pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    srv: Data<Addr<Lobby>>,
    session: Session,
    app_data: web::Data<Mutex<AppState>>
) -> Result<HttpResponse, Error> {
    let (mut logged, mut username) = is_logged(&session).await; let app_data = app_data.lock().unwrap();
    // username == '' create add to session add to mongo
    if username == "" {
        username = random_username();
        let anon = User::new(random_username());
        let verifier = create_verifier();
        set_session(&session, verifier).await;
        set_username(&session, &anon.username).await;
        set_reg(&session, &false).await;
        let mongo_result = app_data.users.insert_one(&anon, None).await;
        match mongo_result {
            Ok(_) => (),
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }
    let ws = WsConn::new(username, logged, srv.get_ref().clone());
    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
