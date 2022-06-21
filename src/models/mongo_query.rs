use crate::lichess::login::random_username;

use super::mongo::{Db, User};

pub async fn create_user(colls: &Db) -> User {
    let username = random_username();
    let user = User::new(username);
       loop {
        if let Ok(r) = colls.users.insert_one(&user, None).await {
            return user;
        }
    }
}