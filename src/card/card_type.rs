use super::{card_color::CardColor, common_card_type::CommonCardType};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardType {
    Common(CommonCardType),
    Rule,
    Jocker { desc: String, color: CardColor },
}

impl CardType {
    pub fn to_card_string(&self) -> String {
        // returns a one or two character string for the card graphics
        match self {
            CardType::Common(color_type) => color_type.to_card_string(),
            CardType::Rule => "♯".to_string(), // trouver un caracter de rêgle (l'outil pour mesurer) à mettre à la place
            CardType::Jocker { color, .. } => {
                if color == &CardColor::Red {
                    "\x1b[31mJ\x1b[0m".to_string()
                } else {
                    "J".to_string()
                }
            }
        }
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
