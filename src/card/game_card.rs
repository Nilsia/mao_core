use super::Card;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Default)]
pub enum GameCardVisibility {
    #[default]
    Owner,
    Other,
    All,
    Players(Vec<usize>),
}

#[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Default)]
pub enum GameCardDisplay {
    Hidden,
    #[default]
    Visible,
    RevealOnHover,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Default)]
pub struct GameCard {
    supposed_card: Option<Card>,
    real_card: Card,
    visibility: GameCardVisibility,
    rules: Vec<String>,
    /// Override visibility => if Hidden, cards should be hidden whatever happens
    display: GameCardDisplay,
    played_by: Option<usize>,
}

impl GameCard {
    pub fn set_as_normal(&mut self) {
        self.supposed_card = None;
        self.display = GameCardDisplay::default();
        self.visibility = GameCardVisibility::default();
    }
    pub fn new(
        supposed_card: Option<Card>,
        real_card: Card,
        visibility: GameCardVisibility,
        rules: &[&str],
        played_by: Option<usize>,
    ) -> Self {
        Self {
            supposed_card,
            real_card,
            visibility,
            rules: rules.iter().map(|s| String::from(*s)).collect(),
            display: GameCardDisplay::Visible,
            played_by,
        }
    }

    pub fn set_display(&mut self, display: GameCardDisplay) {
        self.display = display
    }

    pub fn display(&self) -> GameCardDisplay {
        self.display
    }

    pub fn normal_card(real_card: Card) -> Self {
        Self {
            supposed_card: None,
            real_card,
            visibility: GameCardVisibility::Owner,
            rules: Vec::new(),
            display: GameCardDisplay::Visible,
            played_by: None,
        }
    }

    pub fn played_card(&self) -> &Card {
        self.supposed_card.as_ref().unwrap_or(&self.real_card)
    }

    pub fn visibility(&self) -> &GameCardVisibility {
        &self.visibility
    }

    pub fn rules(&self) -> &[String] {
        self.rules.as_ref()
    }

    pub fn set_supposed_card(&mut self, card: Card) {
        self.supposed_card = Some(card)
    }

    pub fn real_card(&self) -> &Card {
        &self.real_card
    }

    pub fn set_played_by(&mut self, played_by: Option<usize>) {
        self.played_by = played_by;
    }

    pub fn played_by(&self) -> Option<usize> {
        self.played_by
    }

    pub fn clear_properties(&mut self) {
        *self = Self::normal_card(self.real_card.clone())
    }
}
