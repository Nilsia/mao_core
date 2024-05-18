use std::path::PathBuf;

use libloading::{Library, Symbol};

use crate::{
    error::Error, mao_event::mao_event::MaoEvent, mao_event_result::MaoEventResult,
    mao_struct::Mao, VERSION,
};

type OnEventFunctionSignature = fn(&MaoEvent, &mut Mao) -> anyhow::Result<MaoEventResult>;
type VersionGetterFunction = fn() -> String;

#[derive(Debug)]
pub struct Rule {
    lib: Library,
    name: String,
    path: PathBuf,
}

impl Rule {
    pub fn new(lib: Library, name: String) -> Self {
        Self {
            lib,
            path: PathBuf::from(&name),
            name: PathBuf::from(name)
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

    pub unsafe fn get_on_event_func(&self) -> Result<Symbol<OnEventFunctionSignature>, Error> {
        unsafe { Ok(self.lib.get::<OnEventFunctionSignature>(b"on_event\0")?) }
    }

    pub(crate) unsafe fn get_version(&self) -> Result<String, Error> {
        unsafe { Ok(self.lib.get::<VersionGetterFunction>(b"get_version\0")?()) }
    }

    pub fn is_valid_rule(&self, mao: &mut Mao) -> Result<(), Error> {
        unsafe {
            let event = MaoEvent::VerifyEvent;
            match self.get_on_event_func() {
                Ok(func) => {
                    func(&event, mao).unwrap();
                    let version = &self.get_version()?;
                    match version == VERSION {
                        true => Ok(()),
                        false => Err(Error::RuleNotValid { desc:  crate::error::DmDescription(format!("versions are incompatible, please consider recompiling your rule (mao_library: {}, rule: {})", VERSION, version)) }),
                    }
                }
                Err(e) => Err(Error::LibLoading {
                    desc: crate::error::DmDescription(e.to_string()),
                }),
            }
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
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
        unsafe { Ok(Self::new(Library::new(value)?, value.to_owned())) }
    }
}
