use crate::{mao::node_state::PlayerAction, stack::stack_type::StackType};

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
    DlOpen2 {
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
    InvalidRuleIndex {
        rule_index: usize,
        len: usize,
    },
    FunctionNotFound {
        rule_name: String,
        func_name: String,
    },
    MissingRequestCallbacks(Vec<&'static str>),
    InvalidRequestResponse,
    InvalidMaoInteraction {
        expected: Vec<PlayerAction>,
        received: Vec<PlayerAction>,
    },
    OnMaoInteraction(String),
    RuleAlreadyActivated {
        rule_name: String,
    },
    RuleNotActivated {
        rule_name: String,
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
            Error::DlOpen2 { desc } => write!(f, "{}", desc.0),
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
            Error::MissingRequestCallbacks(keys) => write!(
                f,
                "The following request callback functions are missing : {}",
                keys.join(", ")
            ),
            Error::InvalidRequestResponse => write!(f, "Invalid Request Reponse"),
            Error::InvalidMaoInteraction { expected, received } => write!(
                f,
                "Invalid mao interactions: \nexpected: {expected:?}\nreceived: {received:?}"
            ),
            Error::OnMaoInteraction(s) => write!(f, "OnMaoInteraction: {}", s),
            Error::InvalidRuleIndex { rule_index, len } => write!(
                f,
                "Invalid Rule index given : {} out of {}",
                rule_index, len
            ),
            Error::RuleAlreadyActivated { rule_name } => {
                write!(f, "The {} has already been activated !", rule_name)
            }
            Error::RuleNotActivated { rule_name } => {
                write!(
                    f,
                    "The {} is not activated and it as been requested to be deactivated !",
                    rule_name
                )
            }
        }
    }
}

impl From<dlopen2::Error> for Error {
    fn from(value: dlopen2::Error) -> Self {
        Self::DlOpen2 {
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
