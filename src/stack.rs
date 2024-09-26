use crate::card::Card;

use self::{stack_property::StackProperty, stack_type::StackType};

pub mod stack_property;
pub mod stack_type;

#[derive(Debug)]
pub struct Stack {
    cards: Vec<Card>,
    visible: bool,
    stack_type: Vec<StackType>,
    #[allow(dead_code)]
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

    pub fn visible(&self) -> bool {
        self.visible
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

    pub fn draw_card(&mut self) -> Option<Card> {
        self.cards.pop()
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
