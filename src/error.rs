use crate::stack::stack_type::StackType;

#[derive(Debug)]
pub struct DmDescription(pub(crate) String);

#[derive(Debug)]
pub enum Error {
    RuleNotValid {
        desc: DmDescription,
    },
    NotEnoughCardsForInitilization,
    InvalidConfig {
        desc: String,
    },
    LibLoading {
        desc: DmDescription,
    },
    IOError {
        error: std::io::Error,
    },
    AnyhowError {
        error: anyhow::Error,
    },
    RuleNotFound {
        desc: DmDescription,
    },
    NoStackAvailable {
        stacks: Vec<StackType>,
    },
    NotEnoughCards,
    GivenSliceEmpty,
    InvalidCardIndex {
        card_index: usize,
        len: usize,
    },
    InvalidStackIndex {
        stack_index: usize,
        len: usize,
    },
    InvalidPlayerIndex {
        player_index: usize,
        len: usize,
    },
    FunctionNotFound {
        rule_name: String,
        func_name: String,
    },
}

impl Error {
    fn invalid_index(&self, msg: &str, index: usize, len: usize) -> String {
        format!("Index out of range : {index} (len: {len}, {msg}) ")
    }
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
            Error::InvalidConfig { ref desc } => write!(f, "{}", desc),
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
            Error::InvalidPlayerIndex { player_index, len } => {
                write!(f, "{}", self.invalid_index("player", *player_index, *len))
            }
            Error::NotEnoughCards => write!(f, "Not enough cards"),
            Error::GivenSliceEmpty => write!(f, "Given slice empty"),
            Error::InvalidCardIndex { card_index, len } => {
                write!(f, "{}", self.invalid_index("card", *card_index, *len))
            }
            Error::InvalidStackIndex { stack_index, len } => {
                write!(f, "{}", self.invalid_index("stack", *stack_index, *len))
            }
            Error::FunctionNotFound {
                rule_name,
                func_name,
            } => write!(
                f,
                "The function {} has not been found inside the rule {}",
                func_name, rule_name
            ),
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
