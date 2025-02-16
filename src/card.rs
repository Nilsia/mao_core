pub mod card_color;
pub mod card_type;
pub mod card_value;
pub mod common_card_type;
pub mod game_card;

use self::{card_color::CardColor, card_type::CardType, card_value::CardValue};

pub const RED: &str = "[31m";
pub const RESET: &str = "[0m";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Card {
    value: CardValue,
    sign: CardType,
}

impl Card {
    pub fn new(value: CardValue, sign: CardType) -> Self {
        Self { value, sign }
    }

    pub fn get_value(&self) -> &CardValue {
        &self.value
    }

    pub fn get_sign(&self) -> &CardType {
        &self.sign
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
