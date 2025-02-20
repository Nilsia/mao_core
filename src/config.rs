use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    path::PathBuf,
    str::FromStr,
};

use crate::{
    card::{card_type::CardType, card_value::CardValue},
    error::Error,
    mao::mao_core::PlayerTurnChange,
};

use serde::{de, Deserialize};

#[derive(Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct CardEffectsStruct(HashMap<CardEffectsKey, CardEffects>);

impl CardEffectsStruct {
    pub fn add_rule_name(&mut self, rule_name: &str) -> anyhow::Result<()> {
        for effects in self.values_mut() {
            for effect in effects.effects_mut() {
                match &mut effect.rule_effect {
                    Some(v) => v.rule_name = rule_name.to_owned(),
                    None => {
                        effect.rule_effect = Some(RuleCardsEffect {
                            rule_name: rule_name.to_owned(),
                            error_message: None,
                        })
                    }
                }
            }
        }
        Ok(())
    }
    pub fn remove_card_effects(&mut self, card_effects: &CardEffectsStruct) -> anyhow::Result<()> {
        for (outer_key, outer_value) in card_effects.iter() {
            if let Some(value_self) = self.get_mut(outer_key) {
                value_self
                    .effects_mut()
                    .retain(|v| !outer_value.effects().contains(v));
                if value_self.effects().is_empty() {
                    self.remove(outer_key);
                }
            }
        }
        Ok(())
    }
    pub fn merge_card_effects(
        &mut self,
        mut card_effects: CardEffectsStruct,
    ) -> anyhow::Result<()> {
        for (card_key, card_effect_outer) in card_effects.iter_mut() {
            match self.get_mut(card_key) {
                Some(card_effect_self) => {
                    card_effect_self
                        .effects_mut()
                        .append(card_effect_outer.effects_mut());
                }
                None => {
                    self.insert(card_key.to_owned(), card_effect_outer.to_owned());
                }
            }
        }
        Ok(())
    }
}

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
            for s in effects.effects() {
                match_single_card_effect(&s.effect)
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
        self.sanitize();
        Ok(())
    }

    /// Removes unecessary values and empty values
    fn sanitize(&mut self) {
        for value in self.cards_effects.values_mut() {
            for single_card_effect in value.effects_mut() {
                single_card_effect.effect.sanitize()
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
pub struct OneOrMoreWords(pub Vec<String>);

impl Deref for OneOrMoreWords {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for OneOrMoreWords {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct SayContainerVisitor;
        impl<'dee> serde::de::Visitor<'dee> for SayContainerVisitor {
            type Value = OneOrMoreWords;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "deserializing SayContainer")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(OneOrMoreWords(vec![String::from(v)]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'dee>,
            {
                let mut v = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(c) = seq.next_element()? {
                    v.push(c);
                }
                Ok(OneOrMoreWords(v))
            }
        }
        deserializer.deserialize_any(SayContainerVisitor)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "values")]
pub enum CardPlayerAction {
    #[serde(alias = "say")]
    Say(Vec<OneOrMoreWords>),
    #[serde(alias = "physical")]
    Physical(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SingleCardEffect {
    PlayerTurnChange(PlayerTurnChange),
    CardPlayerAction(CardPlayerAction),
}

impl SingleCardEffect {
    pub fn sanitize(&mut self) {
        match self {
            SingleCardEffect::PlayerTurnChange(_) => (),
            SingleCardEffect::CardPlayerAction(a) => match a {
                CardPlayerAction::Say(c) => c.retain(|v| !v.0.is_empty()),
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
    effects: Vec<CardEffectsInner>,
}

impl CardEffects {
    pub fn effects(&self) -> &[CardEffectsInner] {
        &self.effects
    }
    pub fn effects_mut(&mut self) -> &mut Vec<CardEffectsInner> {
        &mut self.effects
    }
    pub fn new(effects: Vec<CardEffectsInner>) -> Self {
        Self { effects }
    }

    pub fn single(single: CardEffectsInner) -> Self {
        Self {
            effects: vec![single],
        }
    }

    pub fn multiple(multiple: Vec<CardEffectsInner>) -> Self {
        Self { effects: multiple }
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
        Ok(CardEffects { effects: data })
    }
}

impl FromStr for CardEffectsInner {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<SingleCardEffect>().map(CardEffectsInner::only)
    }
}
