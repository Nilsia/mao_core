use crate::card::card::Card;

use super::card_event::CardEvent;

#[derive(Clone, Debug)]
pub enum StackTarget {
    Player(usize),
    Stack(usize),
}

#[derive(Clone, Debug)]
pub enum MaoEvent {
    PlayedCardEvent(CardEvent),
    DiscardCardEvent(CardEvent),
    DrawedCardEvent(CardEvent),
    GiveCardEvent {
        card: Card,
        from_player_index: usize,
        target: StackTarget,
    },
    StackRunsOut {
        empty_stack_index: usize,
        removed_cards_number: usize,
    },
    GameStart,
    EndPlayerTurn {
        events: Vec<MaoEvent>,
    },
}

impl MaoEvent {
    pub fn get_card(&self) -> Option<&Card> {
        match self {
            MaoEvent::PlayedCardEvent(ref e) => Some(&e.card),
            MaoEvent::DiscardCardEvent(ref e) => Some(&e.card),
            MaoEvent::DrawedCardEvent(ref e) => Some(&e.card),
            MaoEvent::GiveCardEvent { card, .. } => Some(&card),
            MaoEvent::StackRunsOut { .. } => None,
            MaoEvent::GameStart => None,
            MaoEvent::EndPlayerTurn { .. } => None,
        }
    }
}
