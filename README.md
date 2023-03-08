# backend-lishuuro
### ♟️ Backend code for lishuuro. ♟️

This small chess server is written in Rust language(Axum framework). :crab:


90% of messages from players goes through websockets. 💬 

Database is MongoDB, with collections for users, articles and shuuroGames. 🍀

For move generator server uses crate [`shuuro`](https://crates.io/crates/shuuro). ⚙️

Redis is used for storing sessions. 🔴 Unlogged players can play 2 days. After that new session is created.
