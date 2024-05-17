#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum StackType {
    Playable,    // where we play
    Drawable,    //pioche
    Discardable, // defausse
}

impl std::fmt::Display for StackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                StackType::Playable => "Playable",
                StackType::Drawable => "Drawble",
                StackType::Discardable => "Discardable",
            }
        )
    }
}
