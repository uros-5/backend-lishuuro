use mongodb::Collection;

use crate::lichess::login::random_username;

use super::Player;

pub async fn create_player(db: &Collection<Player>) -> String {
    loop {
        let username = random_username();
        let player = Player {
            _id: String::from(&username),
            reg: false,
            created_at: bson::DateTime::now(),
        };
        let res = db.insert_one(&player, None).await;
        if let Ok(_) = res {
            return format!("{}", &username);
        }
    }
}