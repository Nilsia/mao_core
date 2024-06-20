use crate::mao::mao_internal::{MaoActionResult, MaoInternal};

use super::MaoEvent;

/// Arguments: (mao, player_index)
pub type PenalityCallbackFunction = fn(&mut MaoInternal, usize) -> anyhow::Result<()>;
/// Arguments: (mao, previous event, results_of_the_previous_event)
pub type OtherRulesCallbackFunction =
    fn(&mut MaoInternal, &MaoEvent, &[&MaoEventResult]) -> anyhow::Result<MaoEventResult>;

#[derive(Clone, Debug)]
pub struct Disallow {
    pub rule: String,
    pub msg: String,
    pub penality: Option<PenalityCallbackFunction>,
}

impl PartialEq for Disallow {
    fn eq(&self, other: &Self) -> bool {
        self.rule == other.rule && self.msg == other.msg
    }
}

impl Disallow {
    pub fn new(rule: String, msg: String, penality: Option<PenalityCallbackFunction>) -> Self {
        Self {
            rule,
            msg,
            penality,
        }
    }

    // pub fn print_warning(&self, ui: Arc<dyn UiMaoTrait>) -> Result<(), Error> {
    //     Ok(ui.show_information(&format!(
    //         "You are not allowed to do this :{} ({})",
    //         self.msg, self.rule,
    //     ))?)
    // }
}
pub type CallbackFunction = fn(&mut MaoInternal, usize) -> anyhow::Result<MaoActionResult>;

/// This structure is affilied to a [`MaoEventResult`]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum Necessary {
    /// This means that the event is necessary by the basic rules of the Mao
    BasicRule(bool),
    /// This means that the event is necessary by a rule
    ImportedRule { necessary: bool, rule_name: String },
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

/// The type of the [`MaoEventResult`]
pub enum MaoEventResultType {
    /// The event has been ignored
    Ignored,
    /// The event has been disallowed by the rule
    Disallow(Disallow),
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
