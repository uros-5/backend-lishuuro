use crate::models::model::ShuuroGame;

#[derive(Clone)]
pub struct LiveGames {
    pub shuuro_games: Vec<ShuuroLive>,
}

impl Default for LiveGames {
    fn default() -> Self {
        LiveGames {
            shuuro_games: vec![],
        }
    }
}

#[derive(Clone)]
pub struct ShuuroLive {
    id: String,
    game: ShuuroGame,
}
