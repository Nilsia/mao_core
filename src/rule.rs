use std::path::PathBuf;

use dlopen2::wrapper::{Container, WrapperApi};

use crate::{
    error::Error,
    mao::{automaton::NodeState, mao_internal::MaoInternal},
    mao_event::{mao_event_result::MaoEventResult, MaoEvent},
    VERSION,
};

type OnEventFunctionSignature =
    fn(event: &MaoEvent, mao: &mut MaoInternal) -> anyhow::Result<MaoEventResult>;

#[derive(WrapperApi)]
pub struct Library {
    on_event: fn(event: &MaoEvent, mao: &mut MaoInternal) -> anyhow::Result<MaoEventResult>,
    get_version: fn() -> String,
    get_actions: Option<fn() -> Vec<Vec<NodeState>>>,
    name: fn() -> &'static str,
    description: Option<fn() -> &'static str>,
    author: fn() -> Option<&'static str>,
}

pub struct Rule {
    lib: Container<Library>,
    light_filename: String,
    path: PathBuf,
}

impl Rule {
    pub fn new(lib: Container<Library>, name: String) -> Self {
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
                .get(0)
                .unwrap()
                .to_owned(),
        }
    }

    pub fn get_on_event_func(&self) -> OnEventFunctionSignature {
        self.lib.on_event
    }

    pub(crate) fn get_version(&self) -> String {
        (self.lib.get_version)()
    }

    pub(crate) fn get_actions(&self) -> Option<Vec<Vec<NodeState>>> {
        self.lib.get_actions()
    }

    pub fn is_valid_rule(&self, mao: &mut MaoInternal) -> Result<(), Error> {
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

    pub fn description(&self) -> Option<&'static str> {
        self.lib.description()
    }

    pub fn name(&self) -> &'static str {
        self.lib.name()
    }

    pub fn author(&self) -> Option<&'static str> {
        self.lib.author()
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

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        unsafe {
            Ok(Self::new(
                Container::load("./".to_string() + value)?,
                value.to_owned(),
            ))
        }
    }
}
