use crate::stack::StackType;

#[derive(Debug)]
pub struct DmDescription(pub(crate) String);

#[derive(Debug)]
pub enum Error {
    RuleNotValid { desc: DmDescription },
    NotEnoughCardsForInitilization,
    InvalidRulesDirectory { desc: DmDescription },
    LibLoading { desc: DmDescription },
    IOError { error: std::io::Error },
    AnyhowError { error: anyhow::Error },
    RuleNotFound { desc: DmDescription },
    NoStackAvailable { stacks: Vec<StackType> },
    StackNotFound { stack_index: usize, len: usize },
    InvalidPlayerIndex { player_index: usize, len: usize },
    NotEnoughCards,
    GivenSliceEmpty,
}

impl From<Result<(), Error>> for Error {
    fn from(value: Result<(), Error>) -> Self {
        value.unwrap_err()
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RuleNotValid { ref desc } => write!(f, "{}", desc.0),
            Error::NotEnoughCardsForInitilization => {
                write!(f, "There is not enough cards in the stack")
            }
            Error::InvalidRulesDirectory { ref desc } => write!(f, "{}", desc.0),
            Error::LibLoading { desc } => write!(f, "{}", desc.0),
            Error::IOError { error } => write!(f, "{}", error),
            Error::RuleNotFound { desc } => write!(f, "{}", desc.0),
            Error::AnyhowError { error } => write!(f, "{}", error),
            Error::NoStackAvailable { stacks } => {
                write!(
                    f,
                    "For some reason, there is no available stacks ({})",
                    stacks
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            Error::StackNotFound { stack_index, len } => {
                write!(f, "index of stack is {} but len is {}", stack_index, len)
            }
            Error::InvalidPlayerIndex { player_index, len } => {
                write!(f, "index of players is {} but len is {}", player_index, len)
            }
            Error::NotEnoughCards => write!(f, "Not enough cards"),
            Error::GivenSliceEmpty => todo!(),
        }
    }
}

impl From<libloading::Error> for Error {
    fn from(value: libloading::Error) -> Self {
        Self::LibLoading {
            desc: DmDescription(value.to_string()),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IOError { error: value }
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::AnyhowError { error: value }
    }
}
