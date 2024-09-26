pub mod card_color;
pub mod card_type;
pub mod card_value;
pub mod common_card_type;

use self::{card_color::CardColor, card_type::CardType, card_value::CardValue};

pub const RED: &str = "[31m";
pub const RESET: &str = "[0m";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Card {
    value: CardValue,
    sign: CardType,
    rule: Option<String>,
    owner_can_see_it: bool,
    other_can_see_it: bool,
}

impl Card {
    pub fn new(value: CardValue, sign: CardType, rule: Option<String>) -> Self {
        Self {
            value,
            sign,
            rule,
            owner_can_see_it: true,
            other_can_see_it: false,
        }
    }

    pub fn get_value(&self) -> &CardValue {
        &self.value
    }

    pub fn get_sign(&self) -> &CardType {
        &self.sign
    }

    pub fn get_rule(&self) -> Option<&str> {
        self.rule.as_deref()
    }

    pub fn owner_can_see_it(&self) -> bool {
        self.owner_can_see_it
    }

    pub fn set_owner_can_see_it(&mut self, value: bool) {
        self.owner_can_see_it = value
    }

    pub fn other_can_see_it(&self) -> bool {
        self.other_can_see_it
    }

    pub fn set_other_can_see_it(&mut self, value: bool) {
        self.other_can_see_it = value
    }

    pub fn get_color(&self) -> CardColor {
        match self.sign {
            CardType::Common(ref c) => c.get_color(),
            CardType::Rule => CardColor::Undefined(String::new()),
            CardType::Jocker { ref color, .. } => color.to_owned(),
        }
    }

    /// Returns a card descriptor just valud and sign in one line
    pub fn to_string_light(&self) -> String {
        self.value.to_card_string() + " " + &self.sign.to_card_string()
    }
}
