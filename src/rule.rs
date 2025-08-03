use std::{path::PathBuf, sync::Arc};

use dlopen2::wrapper::{Container, WrapperApi};

use crate::{
    config::CardEffectsStruct,
    error::{Error, Result},
    mao::{automaton::NodeState, mao_core::MaoCore},
    mao_event::{mao_event_result::MaoEventResult, MaoEvent},
    VERSION,
};

type OnEventFunctionSignature =
    fn(event: &MaoEvent, mao: &mut MaoCore) -> anyhow::Result<MaoEventResult>;

#[derive(Default)]
pub struct RuleData {
    pub name: Arc<str>,
    pub author: Option<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub actions: Option<Vec<Vec<NodeState>>>,
    pub cards_effects: Option<CardEffectsStruct>,
}

#[derive(WrapperApi)]
pub struct Library {
    on_event: fn(event: &MaoEvent, mao: &mut MaoCore) -> anyhow::Result<MaoEventResult>,
    get_version: fn() -> String,
    rule_data: fn() -> RuleData,
    remove_card_effects: Option<fn(mao: &mut MaoCore) -> anyhow::Result<()>>,
}

pub struct Rule {
    lib: Container<Library>,
    light_filename: String,
    path: PathBuf,
    data: RuleData,
}

impl Rule {
    pub fn new(lib: Container<Library>, name: String) -> Self {
        let data = lib.rule_data();
        Self {
            lib,
            path: PathBuf::from(&name),
            light_filename: PathBuf::from(name)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .split(".")
                .map(String::from)
                .collect::<Vec<String>>()
                .first()
                .unwrap()
                .to_owned(),
            data,
        }
    }

    pub fn data(&self) -> &RuleData {
        &self.data
    }

    pub fn get_on_event_func(&self) -> OnEventFunctionSignature {
        self.lib.on_event
    }

    pub(crate) fn get_version(&self) -> String {
        (self.lib.get_version)()
    }

    pub(crate) fn get_actions(&self) -> Option<&[Vec<NodeState>]> {
        self.data.actions.as_ref().map(|v| &**v)
    }

    pub(crate) fn get_actions_mut(&mut self) -> Option<&mut Vec<Vec<NodeState>>> {
        self.data.actions.as_mut()
    }

    pub fn is_valid_rule(&self, mao: &mut MaoCore) -> Result<()> {
        let event = MaoEvent::VerifyEvent;
        (self.get_on_event_func())(&event, mao).unwrap();
        let version = &self.get_version();
        match version == VERSION {
            true => Ok(()),
            false => Err(Error::RuleNotValid { desc:  crate::error::DmDescription(format!("versions are incompatible, please consider recompiling your rule (mao_library: {}, rule: {})", VERSION, version)) }),
        }
    }

    pub fn light_filename(&self) -> &str {
        &self.light_filename
    }

    pub fn description(&self) -> Option<&str> {
        self.data.description.as_deref()
    }

    pub fn name(&self) -> &str {
        &self.data.name
    }

    pub fn author(&self) -> Option<&str> {
        self.data.author.as_deref()
    }
}

impl ToOwned for Rule {
    fn clone_into(&self, target: &mut Self::Owned) {
        *target = self.to_owned();
    }

    type Owned = Self;

    fn to_owned(&self) -> Self::Owned {
        Self::try_from(self.path.to_str().unwrap()).unwrap()
    }
}

impl TryFrom<&str> for Rule {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        unsafe {
            Ok(Self::new(
                Container::load("./".to_string() + value)?,
                value.to_owned(),
            ))
        }
    }
}
