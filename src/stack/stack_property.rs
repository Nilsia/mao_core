use crate::{card::Card, error::Error};

pub trait StackProperty: std::fmt::Debug {
    fn get_cards(&self) -> &[Card];
    fn get_cards_mut(&mut self) -> &mut Vec<Card>;
    fn remove_card(&mut self, card_index: usize) -> Result<Card, Error> {
        if card_index >= self.get_cards_mut().len() {
            Err(Error::InvalidCardIndex {
                card_index,
                len: self.get_cards_mut().len(),
            })
        } else {
            Ok(self.get_cards_mut().remove(card_index))
        }
    }
    fn add_card(&mut self, card: Card) {
        self.get_cards_mut().push(card)
    }
}
