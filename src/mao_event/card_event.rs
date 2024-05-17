use crate::card::card::Card;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct CardEvent {
    pub card: Card,
    pub player_index: usize,
    pub stack_index: Option<usize>,
}

impl CardEvent {
    pub fn new(card: Card, player_index: usize, stack_index: Option<usize>) -> Self {
        Self {
            card,
            player_index,
            stack_index,
        }
    }
}
