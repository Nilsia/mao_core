#[derive(Default, PartialEq, Eq)]
pub struct Disallow {
    pub rule: String,
    pub msg: String,
}

#[derive(PartialEq, Eq)]
pub enum MaoEventResult {
    Ignored,
    Disallow(Disallow),
    Handled,
    OverrideCommonRule,
}
