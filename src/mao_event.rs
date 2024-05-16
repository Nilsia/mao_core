use crate::{card::Card, player::Player, stack::StackProperty};

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

pub enum MaoEvent<'g> {
    PlayedCardEvent(CardEvent),
    DiscardCardEvent(CardEvent),
    DrawedCardEvent(CardEvent),
    GiveCardEvent {
        card: Card,
        from_player: &'g mut Player,
        target: &'g (dyn StackProperty + 'g),
    },
    StackRunsOut {
        empty_stack_index: usize,
        removed_cards_number: usize,
    },
    GameStart,
}
