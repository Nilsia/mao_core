use libloading::{Library, Symbol};

use crate::{error::Error, mao_event::MaoEvent, mao_event_result::MaoEventResult, mao_struct::Mao};

type OnEventFunctionSignature = fn(&MaoEvent, &mut Mao) -> anyhow::Result<MaoEventResult>;

#[derive(Debug)]
pub struct Rule {
    lib: Library,
    name: String,
}

impl Rule {
    pub fn new(lib: Library, name: String) -> Self {
        Self { lib, name }
    }

    pub unsafe fn get_func(&self) -> Result<Symbol<OnEventFunctionSignature>, Error> {
        unsafe { Ok(self.lib.get::<OnEventFunctionSignature>(b"on_event")?) }
    }

    pub fn is_valid_rule(&self) -> bool {
        unsafe { self.get_func().is_ok() }
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
        Self::try_from(self.name.as_str()).unwrap()
    }
}

impl TryFrom<&str> for Rule {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        unsafe { Ok(Self::new(Library::new(value)?, value.to_owned())) }
    }
}
