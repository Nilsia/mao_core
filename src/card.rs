use self::{card_color::CardColor, card_type::CardType, card_value::CardValue};

pub mod card_color;
pub mod card_type;
pub mod card_value;
pub mod common_card_type;

pub const CARD_HEIGHT: u16 = 7;
pub const CARD_WIDTH: u16 = 9;

pub const RED: &str = "[31m";
pub const RESET: &str = "[0m";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Card {
    value: CardValue,
    sign: CardType,
    rule: Option<String>,
    owner_can_see_it: bool,
}

impl Card {
    pub fn new(value: CardValue, sign: CardType, rule: Option<String>) -> Self {
        Self {
            value,
            sign,
            rule,
            owner_can_see_it: true,
        }
    }

    pub fn get_value(&self) -> &CardValue {
        &self.value
    }

    pub fn get_sign(&self) -> &CardType {
        &self.sign
    }

    pub fn get_rule(&self) -> Option<&str> {
        self.rule.as_ref().map(|x| x.as_str())
    }

    pub fn owner_can_see_it(&self) -> bool {
        self.owner_can_see_it
    }

    pub fn set_owner_can_see_it(&mut self, value: bool) {
        self.owner_can_see_it = value
    }

    pub fn get_color(&self) -> CardColor {
        match self.sign {
            CardType::Common(ref c) => c.get_color(),
            CardType::Rule => CardColor::Undefined(String::new()),
            CardType::Jocker { ref color, .. } => color.to_owned(),
        }
    }

    /// Generates a word with a text inside
    fn card_with_word(word: &str) -> String {
        let mut result = String::new();
        result += "╭───────╮";
        result += &word
            .chars()
            .into_iter()
            .map(|c| format!("\n│{c}      │",))
            .collect::<Vec<String>>()
            .join("");
        result += "\n╰───────╯";
        result
    }
    /// Represents an empty [`Card`]
    pub fn hidden_card() -> String {
        Self::card_with_word("cache")
    }

    /// Represents an empty [`Stack`]
    pub fn empty_card() -> String {
        Self::card_with_word("empty")
    }

    pub fn to_string_light(&self) -> String {
        self.value.to_card_string() + " " + &self.sign.to_card_string()
    }
}

#[rustfmt::skip]
impl Card {
    pub fn to_string_custom(&self, visible: bool) -> String {
        let mut result = String::new();

        let add_side = |txt: String, right: bool| {
            if txt.len() == 2 {
                txt
            }
            else {
                if right {
                    txt + " "
                }
                else {
                    " ".to_owned() + &txt
                }
            }
        };
        
        if visible && self.owner_can_see_it {
            result +=            "╭───────╮";
            result += &format!("\n│{}     │", add_side(self.value.to_card_string(), true));
            result += &format!("\n│{}     │", add_side(self.sign.to_card_string(), true));
            result +=          "\n│       │";
            result += &format!("\n│     {}│", add_side(self.sign.to_card_string(), false));
            result += &format!("\n│     {}│", add_side(self.value.to_card_string(), false));
            result +=          "\n╰───────╯";
        } else {
            result += &Self::hidden_card();
        }
        return result;
    }
}

impl ToString for Card {
    fn to_string(&self) -> String {
        self.to_string_custom(true)
    }
}
