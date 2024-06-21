pub mod mao_action;
pub mod mao_internal;
pub mod node_state;

use crate::{config::Config, error::Error};

use self::mao_internal::MaoInternal;

pub struct MaoCore {
    mao: MaoInternal,
    // ui: Arc<dyn UiMaoTrait>,
    // data: dyn UiMaoTrait,
}

impl MaoCore {
    pub fn from_config(config: &Config) -> Result<Self, Error> {
        Ok(Self {
            mao: MaoInternal::from_config(config)?,
            // ui,
        })
    }
    pub fn mao(&self) -> &MaoInternal {
        &self.mao
    }
    pub fn mao_mut(&mut self) -> &mut MaoInternal {
        &mut self.mao
    }
}

impl MaoCore {
    // pub fn request_stack_choice(&mut self, data: RequestData) -> anyhow::Result<RequestResponse> {
    //     self.ui.request_stack_choice(&mut self.mao, data)
    // }
}
