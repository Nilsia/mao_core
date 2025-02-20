use super::Card;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Default)]
pub enum GameCardVisibility {
    #[default]
    Owner,
    Other,
    All,
    Players(Vec<usize>),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Default)]
pub struct GameCard {
    supposed_card: Option<Card>,
    real_card: Card,
    visibility: GameCardVisibility,
    rule: Option<String>,
}

impl GameCard {
    pub fn new(
        supposed_card: Option<Card>,
        real_card: Card,
        visibility: GameCardVisibility,
        rule: Option<&str>,
    ) -> Self {
        Self {
            supposed_card,
            real_card,
            visibility,
            rule: rule.map(String::from),
        }
    }

    pub fn normal_card(real_card: Card) -> Self {
        Self {
            supposed_card: None,
            real_card,
            visibility: GameCardVisibility::Owner,
            rule: None,
        }
    }

    pub fn played_card(&self) -> &Card {
        self.supposed_card.as_ref().unwrap_or(&self.real_card)
    }

    pub fn visibility(&self) -> &GameCardVisibility {
        &self.visibility
    }

    pub fn rule(&self) -> Option<&str> {
        self.rule.as_ref().map(|v| v.as_str())
    }
}
