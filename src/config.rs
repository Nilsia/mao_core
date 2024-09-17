use std::{collections::HashMap, path::PathBuf, str::FromStr};

use crate::{
    card::{card_type::CardType, card_value::CardValue},
    error::Error,
    mao::mao_core::PlayerTurnChange,
};

use serde::Deserialize;

#[derive(Default, Clone, Deserialize, Debug)]
pub struct Config {
    pub dirname: String,
    pub cards_effects: Option<HashMap<CardEffectsKey, PlayerTurnChange>>,
}

impl Config {
    pub fn verify(&self) -> Result<(), Error> {
        let path = PathBuf::from(&self.dirname);
        if !path.is_dir() {
            return Err(Error::InvalidConfig {
                desc: String::from("Provided path is not a directory"),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct CardEffectsKey {
    pub c_type: Option<CardType>,
    pub value: CardValue,
}

impl CardEffectsKey {
    pub fn new(c_type: Option<CardType>, value: CardValue) -> Self {
        Self { c_type, value }
    }
}

impl FromStr for CardEffectsKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split('_').collect();
        match splitted.len() {
            0 => return Err(anyhow::anyhow!("Invalid key for a card effect")),
            1 => Ok(CardEffectsKey::new(None, s.parse::<CardValue>()?)),
            2 => Ok(CardEffectsKey::new(
                Some(splitted.first().unwrap().parse::<CardType>()?),
                splitted.last().unwrap().parse::<CardValue>()?,
            )),
            _ => return Err(anyhow::anyhow!("Too many objects for key of card effect")),
        }
    }
}

struct CardEffectsKeyVisitor;

impl<'de> Deserialize<'de> for CardEffectsKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(CardEffectsKeyVisitor)
    }
}

impl<'de> serde::de::Visitor<'de> for CardEffectsKeyVisitor {
    type Value = CardEffectsKey;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a key of the possible effects a card can have")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse::<CardEffectsKey>()
            .map_err(serde::de::Error::custom)
    }
}
