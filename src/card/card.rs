use super::{card_value::CardValue, card_type::CardType, card_color::CardColor};





#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Card {
    value: CardValue,
    sign: CardType,
    rule: Option<String>,
    owner_can_see_it: bool
}

impl Card {
    pub fn new(value: CardValue, sign: CardType, rule: Option<String>) -> Self {
        Self { value, sign, rule , owner_can_see_it: true}
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

    pub fn owner_can_see_it(&self) -> bool {self.owner_can_see_it}

    pub fn set_owner_can_see_it(&mut self , value: bool) {self.owner_can_see_it =value}

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
