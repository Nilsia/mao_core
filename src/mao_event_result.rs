use crate::mao_struct::Mao;

pub type PenalityCallbackFunction = fn(&mut Mao, player_index: usize) -> anyhow::Result<()>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct Disallow {
    pub rule: String,
    pub msg: String,
    pub penality: Option<PenalityCallbackFunction>,
}
impl Disallow {
    pub fn new(rule: String, msg: String, penality: Option<PenalityCallbackFunction>) -> Self {
        Self {
            rule,
            msg,
            penality,
        }
    }

    pub fn print_warning(&self) {
        println!(
            "You are not allowed to do this :{} ({})",
            self.msg, self.rule
        );
    }
}

pub type CallbackFuntion = fn(mao: &mut Mao) -> anyhow::Result<()>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct MaoEventResult {
    pub necessary: bool,
    pub res_type: MaoEventResultType,
}

impl MaoEventResult {
    pub fn new(necessary: bool, res_type: MaoEventResultType) -> Self {
        Self {
            necessary,
            res_type,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum MaoEventResultType {
    Ignored,
    Disallow(Disallow),
    OverrideBasicRule(CallbackFuntion),
    ExecuteBeforeTurnChange(CallbackFuntion),
    ExecuteAfterTurnChange(CallbackFuntion),
    Necessary,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct LightMaoEventResult {
    pub necessary: bool,
    pub res_type: MaoEventResultType,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum LightMaoEventResultType {
    Ignored,
    Disallow(Disallow),
    OverrideBasicRule,
    ExecuteBeforeTurnChange,
    ExecuteAfterTurnChange,
    Necessary,
}
