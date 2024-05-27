pub mod mao_internal;

use crate::{config::Config, error::Error};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use self::mao_internal::{MaoInternal, RequestData, RequestResponse};

pub trait UiMaoTrait: Debug {
    fn request_stack_choice(
        &mut self,
        mao: &mut MaoInternal,
        data: RequestData,
    ) -> anyhow::Result<RequestResponse>;

    fn request_card_choice(
        &mut self,
        mao: &mut MaoInternal,
        data: RequestData,
    ) -> anyhow::Result<RequestResponse>;

    fn show_information(&mut self, msg: &str) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct MaoCore {
    mao: MaoInternal,
    ui: Arc<Mutex<dyn UiMaoTrait>>,
}

impl MaoCore {
    pub fn from_config(config: &Config, ui: Arc<Mutex<dyn UiMaoTrait>>) -> Result<Self, Error> {
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

    pub fn ui(&self) -> Arc<Mutex<dyn UiMaoTrait>> {
        self.ui.clone()
    }
}

impl MaoCore {
    pub fn request_stack_choice(&mut self, data: RequestData) -> anyhow::Result<RequestResponse> {
        self.ui
            .lock()
            .unwrap()
            .request_stack_choice(&mut self.mao, data)
    }
}
