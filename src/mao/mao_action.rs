use super::automaton::PlayerAction;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MaoInteraction {
    pub index: Option<usize>,
    pub action: PlayerAction,
}

impl MaoInteraction {
    pub fn new(index: Option<usize>, action: PlayerAction) -> Self {
        Self { index, action }
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
            },
            self.index
                .map_or("new".to_string(), |v| (v + 1).to_string())
        )
    }
}
