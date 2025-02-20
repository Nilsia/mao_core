use std::sync::Arc;

use crate::{
    card::game_card::GameCard,
    mao::{Data, DataContainer},
    stack::stack_property::StackProperty,
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Player {
    pseudo: Arc<str>,
    hand: Vec<GameCard>, // (rule name, card)
    player_data: Data,
}

impl Player {
    pub fn new(pseudo: String, hand: Vec<GameCard>) -> Self {
        Self {
            pseudo: pseudo.into(),
            hand,
            player_data: Data::default(),
        }
    }

    pub fn get_pseudo(&self) -> Arc<str> {
        Arc::clone(&self.pseudo)
    }
}

impl StackProperty for Player {
    fn get_cards(&self) -> &[GameCard] {
        &self.hand
    }

    fn get_cards_mut(&mut self) -> &mut Vec<GameCard> {
        &mut self.hand
    }
}

impl DataContainer for Player {
    fn data(&self) -> &Data {
        &self.player_data
    }

    fn data_mut(&mut self) -> &mut Data {
        &mut self.player_data
    }
}
