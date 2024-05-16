use crate::{card::Card, error::Error, stack::StackProperty};

#[derive(Debug, Clone)]
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

    pub fn print_self_cards(&self) -> Result<(), Error> {
        println!("Your hand ({}) : ", self.pseudo);
        self.print_cards()?;
        Ok(())
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
