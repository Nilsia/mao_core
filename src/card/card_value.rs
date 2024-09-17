use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardValue {
    Number(isize),
    MinusInfinity,
    PlusInfinity,
}

impl FromStr for CardValue {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<isize>().map_or_else(
            |_| match s {
                "plusinfinity" => Ok(Self::PlusInfinity),
                "minusinfinity" => Ok(Self::MinusInfinity),
                _ => Err(anyhow::anyhow!("Invalid str for parsing CardValue")),
            },
            |v| Ok(Self::Number(v)),
        )
    }
}

impl Default for CardValue {
    fn default() -> Self {
        Self::Number(1)
    }
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
