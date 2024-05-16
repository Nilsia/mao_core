use std::io::Write;

use crate::{card::Card, error::Error};

pub trait StackProperty {
    fn get_cards(&self) -> &[Card];
    fn get_cards_mut(&mut self) -> &mut Vec<Card>;
    fn print_cards(&self) -> Result<(), Error> {
        for card in self.get_cards() {
            print!("{} ||", card.to_string());
        }
        print!("\n");
        std::io::stdout().flush()?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Stack {
    cards: Vec<Card>,
    visible: bool,
    stack_type: Vec<StackType>,
    in_fron_of: Option<String>, // player pseudo
}

impl Stack {
    pub fn new(cards: Vec<Card>, visible: bool, stack_type: Vec<StackType>) -> Self {
        Self {
            cards,
            visible,
            stack_type,
            in_fron_of: None,
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn then_some<T>(&self, t: T) -> Option<T> {
        self.visible.then_some(t)
    }

    pub fn then<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce() -> T,
    {
        self.visible.then(f)
    }

    pub fn get_stack_types(&self) -> &[StackType] {
        &self.stack_type
    }

    pub fn get_stack_types_mut(&mut self) -> &mut Vec<StackType> {
        &mut self.stack_type
    }

    pub fn top(&self) -> Option<&Card> {
        self.cards.last()
    }
    pub fn top_mut(&mut self) -> Option<&mut Card> {
        self.cards.last_mut()
    }
}

impl std::ops::Deref for Stack {
    type Target = Vec<Card>;

    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}
impl std::ops::DerefMut for Stack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cards
    }
}

impl StackProperty for Stack {
    fn get_cards(&self) -> &[Card] {
        &self.cards
    }

    fn get_cards_mut(&mut self) -> &mut Vec<Card> {
        &mut self.cards
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum StackType {
    PlayedStack, // where we play
    Draw,        //pioche
    Discard,     // defausse
}

impl std::fmt::Display for StackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                StackType::PlayedStack => "PlayedStack",
                StackType::Draw => "Draw",
                StackType::Discard => "Discard",
            }
        )
    }
}
