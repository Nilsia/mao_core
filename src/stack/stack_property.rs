use crate::{card::game_card::GameCard, error::Error};

pub trait StackProperty: std::fmt::Debug {
    fn get_cards(&self) -> &[GameCard];
    fn get_cards_mut(&mut self) -> &mut Vec<GameCard>;
    fn remove_card(&mut self, card_index: usize) -> Result<GameCard, Error> {
        if card_index >= self.get_cards_mut().len() {
            Err(Error::InvalidCardIndex {
                card_index,
                len: self.get_cards_mut().len(),
            })
        } else {
            Ok(self.get_cards_mut().remove(card_index))
        }
    }
    fn add_card(&mut self, card: GameCard) {
        self.get_cards_mut().push(card)
    }
    fn get_card(&self, card_index: usize) -> Result<&GameCard, Error> {
        self.get_cards()
            .get(card_index)
            .ok_or(Error::InvalidCardIndex {
                card_index,
                len: self.get_cards().len(),
            })
    }
    fn get_card_mut(&mut self, card_index: usize) -> Result<&mut GameCard, Error> {
        let len = self.get_cards().len();
        self.get_cards_mut()
            .get_mut(card_index)
            .ok_or(Error::InvalidCardIndex { card_index, len })
    }
}
