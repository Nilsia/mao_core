use std::{collections::HashMap, fmt, path::PathBuf, str::FromStr};

use crate::{
    card::{card_type::CardType, card_value::CardValue},
    error::Error,
    mao::mao_core::PlayerTurnChange,
};

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

#[derive(Default, Clone, Deserialize, Debug)]
pub struct Config {
    pub dirname: String,
    pub cards_effects: Option<HashMap<CardEffectsKey, CardEffects>>,
}

impl Config {
    pub fn verify(&mut self) -> Result<(), Error> {
        let path = PathBuf::from(&self.dirname);
        if !path.is_dir() {
            return Err(Error::InvalidConfig {
                desc: String::from("Provided path is not a directory"),
            });
        }
        self.clear();
        Ok(())
    }

    /// Removes unecessary values
    fn clear(&mut self) {
        if let Some(values) = self.cards_effects.as_mut().map(|hash| hash.values_mut()) {
            for value in values {
                match value {
                    CardEffects::SingleEffect(single) => single.clear(),
                    CardEffects::MultipleEffects(v) => {
                        for single_card_effect in v {
                            single_card_effect.clear()
                        }
                    }
                }
            }
        }
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
            0 => Err(anyhow::anyhow!("Invalid key for a card effect")),
            1 => Ok(CardEffectsKey::new(None, s.parse::<CardValue>()?)),
            2 => Ok(CardEffectsKey::new(
                Some(splitted.first().unwrap().parse::<CardType>()?),
                splitted.last().unwrap().parse::<CardValue>()?,
            )),
            _ => Err(anyhow::anyhow!("Too many objects for key of card effect")),
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

#[derive(Clone, Debug)]
pub enum SingOrMult<T>
where
    T: std::fmt::Debug + Clone,
{
    Single(T),
    Multiple(Vec<T>),
}

#[derive(Clone, Debug, Deserialize)]
pub struct CardPlayerAction {
    #[serde(rename = "values")]
    pub has_to_contains: Vec<SingOrMult<String>>,
}

impl<'de> Deserialize<'de> for SingOrMult<String> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SingOrMultVisitor;
        impl<'dee> Visitor<'dee> for SingOrMultVisitor {
            type Value = SingOrMult<String>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "deserializing SingOrMult<String>")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(SingOrMult::Single(v.to_owned()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'dee>,
            {
                let mut data = match seq.size_hint() {
                    Some(sisze) => Vec::with_capacity(sisze),
                    None => vec![],
                };
                while let Some(value) = seq.next_element::<String>()? {
                    data.push(value);
                }
                if data.len() == 1 {
                    Ok(SingOrMult::Single(data.pop().unwrap()))
                } else {
                    Ok(SingOrMult::Multiple(data))
                }
            }
        }

        deserializer.deserialize_any(SingOrMultVisitor)
    }
}

#[derive(Clone, Debug)]
pub enum SingleCardEffect {
    PlayerTurnChange(PlayerTurnChange),
    CardPlayerAction(CardPlayerAction),
}

impl SingleCardEffect {
    pub fn clear(&mut self) {
        match self {
            SingleCardEffect::PlayerTurnChange(_) => (),
            SingleCardEffect::CardPlayerAction(a) => {
                println!("found SingOrMult");
                a.has_to_contains.retain(|v| match v {
                    SingOrMult::Single(_) => true,
                    SingOrMult::Multiple(v) => !v.is_empty(),
                })
            }
        }
    }
}

impl<'de> Deserialize<'de> for SingleCardEffect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SingleCardEffectVisitor;
        impl<'dee> serde::de::Visitor<'dee> for SingleCardEffectVisitor {
            type Value = SingleCardEffect;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "deserializing SingleCardEffect")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                SingleCardEffect::from_str(v).map_err(serde::de::Error::custom)
            }
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'dee>,
            {
                CardPlayerAction::deserialize(serde::de::value::MapAccessDeserializer::new(map))
                    .map(SingleCardEffect::CardPlayerAction)
            }
        }
        deserializer.deserialize_any(SingleCardEffectVisitor)
    }
}

impl FromStr for SingleCardEffect {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<PlayerTurnChange>().map(Self::PlayerTurnChange)
    }
}

#[derive(Clone, Debug)]
pub enum CardEffects {
    SingleEffect(SingleCardEffect),
    MultipleEffects(Vec<SingleCardEffect>),
}

struct CardEffectsVisitor;

impl<'de> Deserialize<'de> for CardEffects {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(CardEffectsVisitor)
    }
}

impl<'de> serde::de::Visitor<'de> for CardEffectsVisitor {
    type Value = CardEffects;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a card effect is wrong")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse::<SingleCardEffect>()
            .map_err(serde::de::Error::custom)
            .map(CardEffects::SingleEffect)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        CardPlayerAction::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            .map(|v| CardEffects::SingleEffect(SingleCardEffect::CardPlayerAction(v)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut data: Vec<SingleCardEffect> = match seq.size_hint() {
            Some(s) => Vec::with_capacity(s),
            None => vec![],
        };
        while let Some(value) = seq.next_element::<SingleCardEffect>()? {
            data.push(value);
        }
        Ok(CardEffects::MultipleEffects(data))
    }
}
