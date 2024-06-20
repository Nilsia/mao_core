use crate::card::Card;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct CardEvent {
    pub played_card: Card,
    pub card_index: usize,
    pub player_index: usize,
    pub stack_index: Option<usize>,
}

impl CardEvent {
    pub fn new(
        card: Card,
        player_index: usize,
        stack_index: Option<usize>,
        card_index: usize,
    ) -> Self {
        Self {
            played_card: card,
            player_index,
            stack_index,
            card_index,
        }
    }
}
