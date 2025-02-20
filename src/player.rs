use crate::{
    card::{game_card::GameCard, Card},
    mao::{Data, DataContainer},
    stack::stack_property::StackProperty,
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Player {
    pseudo: String,
    hand: Vec<GameCard>, // (rule name, card)
    player_data: Data,
}

impl Player {
    pub fn new(pseudo: String, hand: Vec<GameCard>) -> Self {
        Self {
            pseudo,
            hand,
            player_data: Data::default(),
        }
    }

    pub fn get_pseudo(&self) -> &str {
        &self.pseudo
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
