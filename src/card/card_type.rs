use std::str::FromStr;

use crate::card::{RED, RESET};

use super::{card_color::CardColor, common_card_type::CommonCardType};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardType {
    Common(CommonCardType),
    Rule,
    Jocker { desc: String, color: CardColor },
}
impl CardType {
    // Returns a one or two character string for the card graphics
    pub fn to_card_string(&self) -> String {
        match self {
            CardType::Common(color_type) => color_type.to_card_string(),
            CardType::Rule => "♯".to_string(), // trouver un caracter de rêgle (l'outil pour mesurer) à mettre à la place
            CardType::Jocker { color, .. } => {
                if color == &CardColor::Red {
                    format!("\x1b{}\x1b{}", RED, RESET)
                } else {
                    "J".to_string()
                }
            }
        }
    }
}

impl Default for CardType {
    fn default() -> Self {
        Self::Common(CommonCardType::Spade)
    }
}

impl FromStr for CardType {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "rule" => Self::Rule,
            "jred" => Self::Jocker {
                desc: String::new(),
                color: CardColor::Red,
            },
            "jvlack" => Self::Jocker {
                desc: String::new(),
                color: CardColor::Black,
            },
            _ => Self::Common(CommonCardType::from_str(value)?),
        })
    }
}

impl std::fmt::Display for CardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Common(c) => write!(f, "{c}"),
            Self::Rule => write!(f, "Rule"),
            Self::Jocker { color, desc } => write!(f, "Jocker({desc},{color})"),
        }
    }
}
