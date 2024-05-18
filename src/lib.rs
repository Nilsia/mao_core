pub mod action_msg_range;
pub mod card {
    pub mod card;
    pub mod card_color;
    pub mod card_type;
    pub mod card_value;
    pub mod common_card_type;
}
pub mod config;
pub mod error;
pub mod mao_event {
    pub mod card_event;
    pub mod mao_event;
}
pub mod mao_event_result;
pub mod mao_struct;
pub mod player;
pub mod rule;
pub mod stack {
    pub mod stack;
    pub mod stack_property;
    pub mod stack_type;
}

pub const VERSION: &str = "1.0";
