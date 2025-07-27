use crate::card::game_card::GameCard;

use self::card_event::CardEvent;

pub mod card_event;
pub mod mao_event_result;

/// Represents either the index of a [`Player`] or a [`Stack`] (implements [`StackProporty`])
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StackTarget {
    Player(usize),
    Stack(usize),
}

/// A event which an occur when playing the mao board game
#[derive(Clone, Debug, PartialEq)]
pub enum MaoEvent {
    /// Released when a card is played
    PlayedCardEvent(CardEvent),
    /// Released when a card is discarded
    DiscardCardEvent(CardEvent),
    /// Released when a card is drawed
    DrawedCardEvent(CardEvent),
    /// Released when a player attempts to give a card to a target which implements [`StackPropoerty`]
    GiveCardEvent {
        card: GameCard,
        from_player_index: usize,
        target: StackTarget,
    },
    /// Released when a [`StackPropery`] runs out of [`Card`]s
    StackPropertyRunsOut { empty_stack_index: StackTarget },
    /// Released when game starts
    GameStart,
    /// Released when a player has finished its turn
    EndPlayerTurn { events: Vec<MaoEvent> },
    /// Released when a player is going to take a penality
    PlayerPenality { player_target: usize },
    /// Released when verifying the validity of the rules (only called at inialization)
    VerifyEvent,
    /// Released when a player says something
    SayEvent {
        message: String,
        player_index: usize,
    },
    /// Released when a player does a physical interaction
    PhysicalEvent {
        physical_name: String,
        player_index: usize,
    },
}

impl MaoEvent {
    pub fn is_recordable(&self) -> bool {
        !matches!(
            self,
            MaoEvent::GameStart
                | MaoEvent::VerifyEvent
                | MaoEvent::StackPropertyRunsOut { .. }
                | MaoEvent::EndPlayerTurn { .. }
                | MaoEvent::PlayerPenality { .. }
        )
    }

    pub fn can_change_turn(&self) -> bool {
        matches!(self, Self::PlayedCardEvent(_) | Self::DrawedCardEvent(_))
    }

    /// Returns the concerned [`Card`] of the event if the event is about a card event
    pub fn get_card(&self) -> Option<&GameCard> {
        match self {
            MaoEvent::PlayedCardEvent(ref e) => Some(&e.played_card),
            MaoEvent::DiscardCardEvent(ref e) => Some(&e.played_card),
            MaoEvent::DrawedCardEvent(ref e) => Some(&e.played_card),
            MaoEvent::GiveCardEvent { card, .. } => Some(&card),
            MaoEvent::StackPropertyRunsOut { .. } => None,
            MaoEvent::GameStart => None,
            MaoEvent::EndPlayerTurn { .. } => None,
            MaoEvent::VerifyEvent => unreachable!("verify event"),
            MaoEvent::PlayerPenality { .. } => None,
            MaoEvent::SayEvent { .. } => None,
            MaoEvent::PhysicalEvent { .. } => None,
        }
    }
}
