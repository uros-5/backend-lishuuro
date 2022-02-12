use crate::models::model::ShuuroGame;
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

pub struct ShuuroLive {
    id: String,
    game: ShuuroGame,
}
