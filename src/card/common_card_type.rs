use super::{card_color::CardColor, RED, RESET};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommonCardType {
    Spade,   // Pique
    Diamond, //Carreau
    Club,    // Trefle
    Heart,   // coeur
}

impl std::fmt::Display for CommonCardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Spade => "Spade",
                Self::Diamond => "Diamond",
                Self::Club => "Club",
                Self::Heart => "Heart",
            }
        )
    }
}

impl CommonCardType {
    pub fn get_color(&self) -> CardColor {
        match self {
            CommonCardType::Spade | CommonCardType::Club => CardColor::Black,
            CommonCardType::Diamond | CommonCardType::Heart => CardColor::Red,
        }
    }
    pub fn to_card_string(&self) -> String {
        // returns a one or two character string for the card graphics
        match self {
            CommonCardType::Spade => "♤".to_string(),
            CommonCardType::Diamond => format!("\x1b{}♦\x1b{}", RED, RESET),
            CommonCardType::Club => "♧".to_string(),
            CommonCardType::Heart => format!("\x1b{}♥\x1b{}", RED, RESET),
        }
    }
}
