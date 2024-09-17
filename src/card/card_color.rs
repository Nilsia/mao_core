#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardColor {
    Red,
    Black,
    Undefined(String),
}

impl std::fmt::Display for CardColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CardColor::Red => "red",
                CardColor::Black => "black",
                CardColor::Undefined(s) => s,
            }
        )
    }
}
