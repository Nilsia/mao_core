#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardValue {
    Number(isize),
    MinusInfinity,
    PlusInfinity,
}

impl CardValue {
    /// Returns a one or two character string for the card graphics
    pub fn to_card_string(&self) -> String {
        match self {
            CardValue::Number(i) => format!("{i}"),
            CardValue::MinusInfinity => "-∞".to_string(),
            CardValue::PlusInfinity => "+∞".to_string(),
        }
    }
}

impl std::fmt::Display for CardValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Number(i) => write!(f, "{i}"),
            Self::MinusInfinity => write!(f, "MinusInfinity"),
            Self::PlusInfinity => write!(f, "PlusInfinity"),
        }
    }
}
