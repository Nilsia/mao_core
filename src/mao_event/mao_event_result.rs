use std::sync::Arc;

use crate::{
    card::Card,
    mao::mao_core::{MaoActionResult, MaoCore},
};

use super::MaoEvent;

/// Arguments: (mao, player_index)
pub type PenalityCallbackFunction = fn(&mut MaoCore, usize) -> anyhow::Result<()>;
/// Arguments: (mao, previous event, results_of_the_previous_event)
pub type OtherRulesCallbackFunction =
    fn(&mut MaoCore, &MaoEvent, &[&MaoEventResult]) -> anyhow::Result<MaoEventResult>;

#[derive(Clone, Debug)]
pub struct Disallow {
    pub rule: Arc<str>,
    pub msg: String,
    pub penality: Option<PenalityCallbackFunction>,
}

impl std::fmt::Display for Disallow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule, self.msg)
    }
}

#[derive(Clone, Debug)]
pub enum CardPlayerActionType {
    Say,
    Do,
    Other(String),
}

const FORGOT_SAY: &str = "You forget to say something";
const FORGOT_DO: &str = "You forget to do something";
impl AsRef<str> for CardPlayerActionType {
    fn as_ref(&self) -> &str {
        match &self {
            CardPlayerActionType::Say => FORGOT_SAY,
            CardPlayerActionType::Do => FORGOT_DO,
            CardPlayerActionType::Other(s) => s,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ForgotSomething {
    pub msg: Option<String>,
    pub forgot_type: CardPlayerActionType,
    pub rule: Option<Arc<str>>,
    pub penality: Option<PenalityCallbackFunction>,
    pub player_pseudo: Arc<str>,
}

impl std::fmt::Display for ForgotSomething {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let basic = "basic";
        write!(
            f,
            "{}: {} ({})",
            self.player_pseudo,
            self.forgot_type.as_ref(),
            self.rule.as_deref().unwrap_or(basic),
        )
    }
}

impl PartialEq for Disallow {
    fn eq(&self, other: &Self) -> bool {
        self.rule == other.rule && self.msg == other.msg
    }
}

impl Disallow {
    pub fn new(rule: Arc<str>, msg: String, penality: Option<PenalityCallbackFunction>) -> Self {
        Self {
            rule,
            msg,
            penality,
        }
    }
}
pub type CallbackFunction = fn(&mut MaoCore, usize) -> anyhow::Result<MaoActionResult>;

/// This structure is affilied to a [`MaoEventResult`]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum Necessary {
    /// This means that the event is necessary by the basic rules of the Mao
    BasicRule(bool),
    /// This means that the event is necessary by a rule
    ImportedRule {
        necessary: bool,
        rule_name: Arc<str>,
    },
}

pub struct MaoEventResult {
    /// The necessity of the event
    pub necessary: Necessary,
    /// The type of result
    pub res_type: MaoEventResultType,
    /// A callback function, this function is called when all rules with the same event have been called, all results not ignored results of all other rules will be passed as arguments
    pub other_rules_callback: Option<OtherRulesCallbackFunction>,
}

impl MaoEventResult {
    pub fn new(necessary: Necessary, res_type: MaoEventResultType) -> Self {
        Self {
            necessary,
            res_type,
            other_rules_callback: None,
        }
    }
}

pub enum PlayerInteractionResult {
    Request(PlayerInteractionRequest),
    Wrong(WrongPlayerInteraction),
}

#[derive(Clone, Debug)]
pub struct PlayerInteractionRequest {
    pub rule: Arc<str>,
    pub request_type: PlayerInteractionRequestType,
    /// (mao_core, prevous_request, request_response)
    pub rule_callback: fn(
        &mut MaoCore,
        PlayerInteractionRequest,
        PlayerInteractionRequest,
    ) -> anyhow::Result<Vec<PlayerInteractionResult>>,
    pub description: String,
    pub player_index: usize,
}

impl PlayerInteractionRequest {
    pub fn generate_response(&self, request_type_response: PlayerInteractionRequestType) -> Self {
        Self {
            rule: Arc::clone(&self.rule),
            request_type: request_type_response,
            rule_callback: self.rule_callback,
            description: String::new(),
            player_index: self.player_index,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PlayerInteractionRequestType {
    Custom(String),
    Card(Option<Card>),
    Confirmation(bool),
}

#[derive(Clone, Debug)]
pub enum WrongPlayerInteraction {
    /// The event has been disallowed by the rule
    Disallow(Disallow),
    /// The player forgot to do/say/... something
    ForgotSomething(ForgotSomething),
    /// When has to give a penality to a player
    GivePenality(GivePenality),
}

impl WrongPlayerInteraction {
    pub fn forgot_doing(
        msg: Option<String>,
        rule: Option<Arc<str>>,
        penality: Option<PenalityCallbackFunction>,
        player_pseudo: Arc<str>,
    ) -> Self {
        Self::ForgotSomething(ForgotSomething {
            forgot_type: CardPlayerActionType::Do,
            rule,
            penality,
            msg,
            player_pseudo,
        })
    }
    pub fn forgot_saying(
        msg: Option<String>,
        rule: Option<Arc<str>>,
        penality: Option<PenalityCallbackFunction>,
        player_pseudo: Arc<str>,
    ) -> Self {
        Self::ForgotSomething(ForgotSomething {
            forgot_type: CardPlayerActionType::Say,
            rule,
            penality,
            msg,
            player_pseudo,
        })
    }

    pub(crate) fn forgot_saying_basic(msg: Option<String>, player_pseudo: Arc<str>) -> Self {
        Self::forgot_saying(msg, None, None, player_pseudo)
    }
    pub(crate) fn forgot_doing_basic(msg: Option<String>, player_pseudo: Arc<str>) -> Self {
        Self::forgot_doing(msg, None, None, player_pseudo)
    }
    pub(crate) fn forgot_saying_ruled(rule: Arc<str>, player_pseudo: Arc<str>) -> Self {
        Self::forgot_saying(None, Some(rule), None, player_pseudo)
    }
    pub(crate) fn forgot_doing_ruled(rule: Arc<str>, player_pseudo: Arc<str>) -> Self {
        Self::forgot_doing(None, Some(rule), None, player_pseudo)
    }
}

impl std::fmt::Display for WrongPlayerInteraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WrongPlayerInteraction::Disallow(dis) => dis.to_string(),
                WrongPlayerInteraction::ForgotSomething(f) => f.to_string(),
                WrongPlayerInteraction::GivePenality(give_penality) => give_penality.to_string(),
            }
        )
    }
}

#[derive(Clone, Debug)]
pub struct GivePenality {
    pub msg: Option<String>,
    pub rule: Option<Arc<str>>,
    pub penality: Option<PenalityCallbackFunction>,
    pub player_index: usize,
}

impl std::fmt::Display for GivePenality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = self.msg.as_ref() {
            f.write_str(msg)
        } else {
            let basic = "basic";
            write!(
                f,
                "Player {} you got a penality (rule: {})",
                self.player_index,
                self.rule.as_deref().unwrap_or(basic)
            )
        }
    }
}

/// The type of the [`MaoEventResult`]
pub enum MaoEventResultType {
    /// The event has been ignored
    Ignored,
    /// The event has been disallowed by the rule
    Disallow(Disallow),
    /// The player forgot to do/say/... something
    ForgetSomething(ForgotSomething),
    /// The event will override basic rules, the function should modify itself the player turn, the basics rules won't be modified besides
    OverrideBasicRule(CallbackFunction),
    /// Contains a function which will modified the player turn but not override the basic change turn, this function will be executed before the basic change turn
    ExecuteBeforeTurnChange(CallbackFunction),
    /// Contains a function which will modified the player turn but not override the basic change turn, this function will be executed before the basic change turn
    ExecuteAfterTurnChange(CallbackFunction),
}

pub struct LightMaoEventResult {
    pub necessary: bool,
    pub res_type: MaoEventResultType,
}

#[derive(PartialEq)]
pub enum LightMaoEventResultType {
    Ignored,
    Disallow(Disallow),
    OverrideBasicRule,
    ExecuteBeforeTurnChange,
    ExecuteAfterTurnChange,
    Necessary,
}
