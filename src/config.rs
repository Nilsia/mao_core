use std::{
    collections::{HashMap, HashSet},
    fmt,
    ops::{Deref, DerefMut},
    path::PathBuf,
    str::FromStr,
};

use crate::{
    card::{card_type::CardType, card_value::CardValue},
    error::Error,
    mao::mao_core::PlayerTurnChange,
};

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

// pub trait TomlExtention<'de, K, V>
// where
//     K: Deserialize<'de>,
//     V: Deserialize<'de>,
// {
//     fn key_value(input: &str) -> (K, V);
//     fn from_str(input: &str) -> Self
//     where
//         Self: Sized,
//     {
//         Self::from_iterator(input.lines())
//     }
//     fn from_iterator<T>(iterator: T) -> Self
//     where
//         T: IntoIterator,
//         T::Item: AsRef<str>;
// }

#[derive(Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct CardEffectsStruct(HashMap<CardEffectsKey, CardEffects>);

impl Deref for CardEffectsStruct {
    type Target = HashMap<CardEffectsKey, CardEffects>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CardEffectsStruct {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// impl<'de> TomlExtention<'de, CardEffectsKey, CardEffects> for CardEffectsStruct {
//     fn from_iterator<T>(iterator: T) -> Self
//     where
//         T: IntoIterator,
//         T::Item: AsRef<str>,
//     {
//         todo!()
//     }

//     fn key_value(input: &str) -> (CardEffectsKey, CardEffects) {
//         let splitted: Vec<&str> = input.split("=").collect();
//         if splitted.len() >= 2 {
//             return (
//                 CardEffectsKey::deserialize(splitted.first().unwrap()).unwrap(),
//                 CardEffects::deserialize(splitted.last().unwrap()).unwrap(),
//             );
//         }
//         // handle this case
//         panic!("")
//     }
// }

#[derive(Default, Clone, Deserialize, Debug)]
pub struct Config {
    pub dirname: String,
    #[serde(default)]
    pub cards_effects: CardEffectsStruct,
}

impl Config {
    pub fn get_all_physical_actions(&self) -> HashSet<String> {
        let mut actions = HashSet::new();
        let mut match_single_card_effect = |card_effect: &SingleCardEffect| match card_effect {
            SingleCardEffect::PlayerTurnChange(_) => (),
            SingleCardEffect::CardPlayerAction(cpa) => match cpa {
                CardPlayerAction::Say(_) => (),
                CardPlayerAction::Physical(p) => {
                    actions.insert(p.to_owned());
                }
            },
        };
        for effects in self.cards_effects.values() {
            match &effects.effects {
                SingOrMult::Single(s) => match_single_card_effect(&s.effect),
                SingOrMult::Multiple(v_s) => {
                    for s in v_s {
                        match_single_card_effect(&s.effect)
                    }
                }
            }
        }
        actions.into_iter().collect()
    }
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
        for value in self.cards_effects.values_mut() {
            match &mut value.effects {
                SingOrMult::Single(single) => single.effect.clear(),
                SingOrMult::Multiple(v) => {
                    for single_card_effect in v {
                        single_card_effect.effect.clear()
                    }
                }
            }
        }
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct CardEffectsKey {
    pub c_type: Option<CardType>,
    pub value: Option<CardValue>,
}

impl CardEffectsKey {
    pub fn new(c_type: Option<CardType>, value: Option<CardValue>) -> Self {
        Self { c_type, value }
    }
}

impl FromStr for CardEffectsKey {
    type Err = anyhow::Error;

    /// Values_Type
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split('_').collect();
        match splitted.len() {
            0 => Err(anyhow::anyhow!("Invalid key for a card effect")),
            1 => {
                let card_effect =
                    CardEffectsKey::new(s.parse::<CardType>().ok(), s.parse::<CardValue>().ok());
                if card_effect.value.is_none() == card_effect.c_type.is_none() {
                    s.parse::<CardType>()?;
                    s.parse::<CardValue>()?;
                }
                Ok(card_effect)
            }
            2 => Ok(CardEffectsKey::new(
                Some(splitted.last().unwrap().parse::<CardType>()?),
                Some(splitted.first().unwrap().parse::<CardValue>()?),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SingOrMult<T>
where
    T: std::fmt::Debug + Clone + Eq + PartialEq,
{
    Multiple(Vec<T>),
    Single(T),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "values")]
pub enum CardPlayerAction {
    #[serde(alias = "say")]
    Say(Vec<SingOrMult<String>>),
    #[serde(alias = "physical")]
    Physical(String),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SingleCardEffect {
    PlayerTurnChange(PlayerTurnChange),
    CardPlayerAction(CardPlayerAction),
}

impl SingleCardEffect {
    pub fn clear(&mut self) {
        match self {
            SingleCardEffect::PlayerTurnChange(_) => (),
            SingleCardEffect::CardPlayerAction(a) => match a {
                CardPlayerAction::Say(c) => c.retain(|v| match v {
                    SingOrMult::Single(_) => true,
                    SingOrMult::Multiple(v) => !v.is_empty(),
                }),
                CardPlayerAction::Physical(_) => (),
            },
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleCardsEffect {
    #[serde(alias = "rule_name")]
    pub rule_name: String,
    #[serde(alias = "rule_error_message")]
    pub error_message: Option<String>,
}

#[derive(Eq, PartialEq, Debug, Clone, Deserialize)]
pub struct CardEffectsInner {
    #[serde(alias = "effects")]
    pub effect: SingleCardEffect,
    #[serde(alias = "rule_info")]
    pub rule_effect: Option<RuleCardsEffect>,
}

impl CardEffectsInner {
    pub fn only(effect: SingleCardEffect) -> Self {
        Self {
            effect,
            rule_effect: None,
        }
    }

    pub fn new(effect: SingleCardEffect, rule_effect: RuleCardsEffect) -> Self {
        Self {
            effect,
            rule_effect: Some(rule_effect),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct CardEffects {
    pub effects: SingOrMult<CardEffectsInner>,
}

impl CardEffects {
    pub fn new(effects: SingOrMult<CardEffectsInner>) -> Self {
        Self { effects }
    }

    pub fn single(single: CardEffectsInner) -> Self {
        Self {
            effects: SingOrMult::Single(single),
        }
    }

    pub fn multiple(multiple: Vec<CardEffectsInner>) -> Self {
        Self {
            effects: SingOrMult::Multiple(multiple),
        }
    }
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

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        CardEffectsInner::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            .map(CardEffects::single)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<CardEffectsInner>()
            .map_err(serde::de::Error::custom)
            .map(CardEffects::single)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut data: Vec<CardEffectsInner> = match seq.size_hint() {
            Some(s) => Vec::with_capacity(s),
            None => vec![],
        };
        while let Some(value) = seq.next_element::<CardEffectsInner>()? {
            data.push(value);
        }
        Ok(CardEffects {
            effects: SingOrMult::Multiple(data),
        })
    }
}

// struct CardsEffectInnerVisitor;

impl FromStr for CardEffectsInner {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<SingleCardEffect>().map(CardEffectsInner::only)
    }
}

// impl<'de> Deserialize<'de> for CardEffectsInner {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_any(CardsEffectInnerVisitor)
//     }
// }

// impl<'de> Visitor<'de> for CardsEffectInnerVisitor {
//     type Value = CardEffectsInner;

//     fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//         write!(formatter, "a card effet is wrong inside inner card effect")
//     }

//     fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
//     where
//         E: serde::de::Error,
//     {
//         v.parse::<CardEffectsInner>()
//             .map_err(serde::de::Error::custom)
//     }

//     fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
//     where
//         A: serde::de::MapAccess<'de>,
//     {
//         for i in map.next_key()
//         let effect =
//             CardPlayerAction::deserialize(serde::de::value::MapAccessDeserializer::new(map))
//                 .map(SingleCardEffect::CardPlayerAction)?;
//         // let mut rule_effect = None;
//         // let a = RuleCardsEffect::deserialize(serde::de::value::MapAccessDeserializer::new(map))
//         Ok(CardEffectsInner {
//             effect,
//             rule_effect: None,
//         })
//     }
// }
