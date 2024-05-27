use crate::{card::Card, stack::stack_property::StackProperty};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct Player {
    pseudo: String,
    hand: Vec<Card>, // (rule name, card)
}

impl Player {
    pub fn new(pseudo: String, hand: Vec<Card>) -> Self {
        Self { pseudo, hand }
    }

    pub fn get_pseudo(&self) -> &str {
        &self.pseudo
    }
}

impl StackProperty for Player {
    fn get_cards(&self) -> &[Card] {
        &self.hand
    }

    fn get_cards_mut(&mut self) -> &mut Vec<Card> {
        &mut self.hand
    }
}
