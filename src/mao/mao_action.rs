use crate::error::{Error, Result};

use super::automaton::PlayerAction;

#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum IdString {
    String(String),
    Index(usize),
}

impl IdString {
    pub fn index_expecting(&self) -> Result<usize> {
        match self {
            IdString::String(_) => Err(Error::InvalidExpectingValue(
                "Expecting index found String".to_owned(),
            )),
            IdString::Index(i) => Ok(*i),
        }
    }

    pub fn string_expecting(&self) -> Result<&str> {
        match self {
            IdString::String(s) => Ok(s),
            IdString::Index(_) => Err(Error::InvalidExpectingValue(
                "Expecting string found index".to_owned(),
            )),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default, PartialOrd, Ord)]
pub struct MaoInteraction {
    pub data: Option<IdString>,
    pub action: PlayerAction,
}

impl MaoInteraction {
    pub fn new(data: Option<IdString>, action: PlayerAction) -> Self {
        Self { data, action }
    }
}
impl std::fmt::Display for MaoInteraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({})",
            match self.action {
                PlayerAction::SelectCard => "Card",
                PlayerAction::SelectPlayer => "Player",
                PlayerAction::SelectPlayableStack => "PlayableStack",
                PlayerAction::SelectDrawableStack => "DrawableStack",
                PlayerAction::SelectDiscardableStack => "DiscardableStack",
                PlayerAction::SelectRule => "Rule",
                PlayerAction::DoAction => "DoAction",
            },
            self.data.as_ref().map_or("new".to_string(), |v| match v {
                IdString::String(s) => s.to_owned(),
                IdString::Index(v) => (v + 1).to_string(),
            }
            .to_string())
        )
    }
}
