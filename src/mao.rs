pub mod mao_internal;

use crate::{config::Config, error::Error};
use std::{fmt::Debug, sync::Arc};

use self::mao_internal::{MaoInternal, RequestData, RequestResponse};

pub trait UiMaoTrait: Debug + Send + Sync {
    fn request_stack_choice(
        &self,
        mao: &mut MaoInternal,
        data: RequestData,
    ) -> anyhow::Result<RequestResponse>;

    fn request_card_choice(
        &self,
        mao: &mut MaoInternal,
        data: RequestData,
    ) -> anyhow::Result<RequestResponse>;

    fn show_information(&self, msg: &str) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct MaoCore {
    mao: MaoInternal,
    ui: Arc<dyn UiMaoTrait>,
    // data: dyn UiMaoTrait,
}

impl MaoCore {
    pub fn from_config(config: &Config, ui: Arc<dyn UiMaoTrait>) -> Result<Self, Error> {
        Ok(Self {
            mao: MaoInternal::from_config(config)?,
            ui,
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
    pub fn request_stack_choice(&mut self, data: RequestData) -> anyhow::Result<RequestResponse> {
        self.ui.request_stack_choice(&mut self.mao, data)
    }
}
