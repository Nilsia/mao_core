#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    fn to_card_string(&self) -> String {
        // returns a one or two character string for the card graphics
        match self {
            CommonCardType::Spade => "♤".to_string(),
            CommonCardType::Diamond => "\x1b[31m♦\x1b[0m".to_string(),
            CommonCardType::Club => "♧".to_string(),
            CommonCardType::Heart => "\x1b[31m♥\x1b[0m".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardType {
    Common(CommonCardType),
    Rule,
    Jocker { desc: String, color: CardColor },
}

impl CardType {
    fn to_card_string(&self) -> String {
        // returns a one or two character string for the card graphics
        match self {
            CardType::Common(color_type) => color_type.to_card_string(),
            CardType::Rule => "♯".to_string(), // trouver un caracter de rêgle (l'outil pour mesurer) à mettre à la place
            CardType::Jocker { desc, color } => {
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardValue {
    Number(isize),
    MinusInfinity,
    PlusInfinity,
}

impl CardValue {
    fn to_card_string(&self) -> String {
        // returns a one or two character string for the card graphics
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Card {
    value: CardValue,
    sign: CardType,
    rule: Option<String>,
}

impl Card {
    pub fn new(value: CardValue, sign: CardType, rule: Option<String>) -> Self {
        Self { value, sign, rule }
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

    pub fn get_color(&self) -> CardColor {
        match self.sign {
            CardType::Common(ref c) => c.get_color(),
            CardType::Rule => CardColor::Undefined(String::new()),
            CardType::Jocker { ref color, .. } => color.to_owned(),
        }
    }
}

impl ToString for Card {
    #[rustfmt::skip]
    fn to_string(&self) -> String {
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
        
        result +=            "╭───────╮";
        result += &format!("\n│{}     │", add_side(self.value.to_card_string(), true));
        result += &format!("\n│{}     │", add_side(self.sign.to_card_string(), true));
        result +=          "\n│       │";
        result += &format!("\n│     {}│", add_side(self.sign.to_card_string(), false));
        result += &format!("\n│     {}│", add_side(self.value.to_card_string(), false));
        result +=          "\n╰───────╯";
        return result;
    }
}
