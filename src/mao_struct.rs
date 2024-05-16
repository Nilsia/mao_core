use core::result::Result;
use rand::{seq::SliceRandom, thread_rng};
use std::{fs, ops::DerefMut, path::PathBuf};

use crate::{
    action_msg_range::ActionMsgRange,
    card::{Card, CardType, CardValue, CommonCardType},
    config::Config,
    error::{DmDescription, Error},
    mao_event::{CardEvent, MaoEvent},
    mao_event_result::MaoEventResult,
    player::Player,
    rule::Rule,
    stack::{Stack, StackProperty, StackType},
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum MaoActionResult {
    ChangedTurn,
    Nothing,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum PlayerTurnResult {
    CanPlay,
    WrongTurn,
    CannotPlaceThisCard {
        card_to_place: Card,
        placed_card: Card,
    },
    Other {
        desc: String,
    },
}

#[derive(Debug)]
pub struct Mao {
    available_rules: Vec<Rule>,
    activated_rules: Vec<Rule>,
    stacks: Vec<Stack>,
    players: Vec<Player>,
    player_turn: usize,
}

// getters and setters
impl Mao {
    pub fn new(available_libraries: Vec<Rule>, stacks: Vec<Stack>, players: Vec<Player>) -> Self {
        Self {
            available_rules: available_libraries,
            activated_rules: Vec::new(),
            stacks,
            players,
            player_turn: 0,
        }
    }

    pub fn light(libraries: Vec<Rule>, stacks: Vec<Stack>) -> Self {
        Self::new(libraries, stacks, Vec::new())
    }

    pub fn get_players(&self) -> &[Player] {
        &self.players
    }

    pub fn get_players_mut(&mut self) -> &mut Vec<Player> {
        &mut self.players
    }

    pub fn get_stacks(&self) -> &[Stack] {
        &self.stacks
    }

    pub fn get_stacks_mut(&mut self) -> &mut Vec<Stack> {
        &mut self.stacks
    }
    pub fn from_directory(config: &Config) -> Result<Self, Error> {
        let path = PathBuf::from(&config.dirname);
        if !path.is_dir() {
            return Err(Error::InvalidRulesDirectory {
                desc: DmDescription("Provided path is not a directory".to_string()),
            });
        }
        let rules: Vec<String> = fs::read_dir(&path)
            .map_err(|e| {
                Err(Error::InvalidRulesDirectory {
                    desc: DmDescription(e.to_string()),
                })
            })?
            .flatten()
            .filter_map(|f| {
                if f.path().is_file() {
                    Some(f.file_name().into_string())
                } else {
                    None
                }
            })
            .flatten()
            .map(|s| path.join(&s).into_os_string().into_string().unwrap())
            .collect();
        let mut libraries = Vec::with_capacity(rules.capacity());
        for rule in rules {
            libraries.push(Rule::try_from(rule.as_str())?);
        }

        // generate stacks
        let mut stacks = vec![Stack::new(
            generate_common_draw(),
            false,
            vec![StackType::Draw],
        )];
        stacks.push(Stack::new(Vec::new(), true, vec![StackType::PlayedStack]));

        let s = Self::light(libraries, stacks);
        if !s.rules_valid() {
            return Err(Error::RuleNotValid {
                desc: DmDescription("A library is not valid".to_string()),
            });
        }
        Ok(s)
    }
}

// miscellious
impl Mao {
    pub fn new_played_stack(&mut self, cards: &[Card], visible: bool) {
        self.stacks.push(Stack::new(
            cards.to_owned(),
            visible,
            vec![StackType::PlayedStack],
        ))
    }

    pub fn rules_valid(&self) -> bool {
        self.available_rules.iter().all(|l| l.is_valid_rule())
    }

    pub fn on_event(&mut self, event: MaoEvent) -> Result<Vec<MaoEventResult>, Error> {
        let mut results = Vec::with_capacity(self.activated_rules.len());
        for i in 0..self.activated_rules.len() {
            unsafe {
                results.push(self.activated_rules.get_unchecked(i).get_func()?(
                    &event, self,
                )?);
            }
        }
        Ok(results)
    }

    pub fn activate_rule(&mut self, rule_name: &str) -> Result<(), Error> {
        Ok(self.activated_rules.push(
            (*self
                .available_rules
                .iter()
                .filter(|rule| rule.get_name() == rule_name)
                .collect::<Vec<&Rule>>()
                .first()
                .ok_or_else(|| Error::RuleNotFound {
                    desc: DmDescription(format!("The rule {} has not been found", rule_name)),
                })?)
            .to_owned(),
        ))
    }

    pub fn init_player(&mut self, pseudo: String, nb_card: usize) -> Result<Player, Error> {
        let first_stack = self.stacks.first_mut().unwrap();
        let len = first_stack.get_cards().len();
        let hand: Vec<Card> = first_stack
            .get_cards_mut()
            .drain((len - nb_card)..)
            .collect();
        if hand.len() != nb_card {
            return Err(Error::NotEnoughCardsForInitilization);
        }
        Ok(Player::new(pseudo, hand))
    }

    pub fn get_specific_stacks_mut(
        &mut self,
        stack_types: &[StackType],
    ) -> Vec<(usize, &mut Stack)> {
        self.stacks
            .iter_mut()
            .enumerate()
            .filter(|stack| {
                stack
                    .1
                    .get_stack_types()
                    .iter()
                    .any(|t| stack_types.contains(t))
            })
            .collect()
    }

    pub fn get_specific_stacks(&self, stack_types: &[StackType]) -> Vec<(usize, &Stack)> {
        self.stacks
            .iter()
            .enumerate()
            .filter(|stack| {
                stack
                    .1
                    .get_stack_types()
                    .iter()
                    .any(|t| stack_types.contains(t))
            })
            .collect()
    }

    fn draw_stack_getter<'a>(
        mao: &Mao,
        stacks: &'a [(usize, &Stack)],
    ) -> Result<Option<(usize, &'a Stack)>, Error> {
        Ok(Some(match stacks.len() {
            0 => {
                println!("No drawable stacks available");
                return Ok(None);
            }
            1 => stacks.first().unwrap().to_owned(),
            _ => stacks
                .get(
                    ActionMsgRange::generate_stack_choice_str(&[StackType::Draw], mao, false)?
                        .get_action()?,
                )
                .unwrap()
                .to_owned(),
        }))
    }

    fn can_play(
        &self,
        player_index: usize,
        card: &Card,
        stack: Option<&Stack>,
    ) -> PlayerTurnResult {
        if player_index != self.player_turn {
            return PlayerTurnResult::WrongTurn;
        }
        if let Some(stack) = stack {
            if let Some(top_card) = stack.top() {
                if card.get_value() != top_card.get_value()
                    && card.get_color() != top_card.get_color()
                {
                    return PlayerTurnResult::CannotPlaceThisCard {
                        card_to_place: card.to_owned(),
                        placed_card: top_card.to_owned(),
                    };
                }
            }
        }

        PlayerTurnResult::CanPlay
    }

    /// # Error
    ///
    /// fails if stack_index is out if range
    ///
    /// this function does not edit the length of stacks
    pub fn refill_drawable_stacks(
        &mut self,
        stack_index: usize,
        check_rules: bool,
    ) -> Result<(), Error> {
        if check_rules {
            let event = MaoEvent::StackRunsOut {
                empty_stack_index: stack_index,
                removed_cards_number: 1,
            };
            if self
                .on_event(event)?
                .iter()
                .any(|r| r == &MaoEventResult::Handled)
            {
                return Ok(());
            }
        }
        let mut cards = Vec::new();
        let mut stacks_spe =
            self.get_specific_stacks_mut(&[StackType::PlayedStack, StackType::Discard]);
        // foreach add to cards and clear stacks
        for i in 0..stacks_spe.len() {
            let stack_cards = stacks_spe.get_mut(i).unwrap().1.deref_mut();
            cards.extend_from_slice(&stack_cards);
            stack_cards.clear();
        }
        // refill drawable stack
        let len = self.stacks.len();
        let stack = self
            .stacks
            .get_mut(stack_index)
            .ok_or(Error::StackNotFound { stack_index, len })?;
        stack.get_cards_mut().clear();
        stack.get_cards_mut().extend_from_slice(&cards);
        Ok(())
    }

    fn give_card_to_player(
        &mut self,
        player_index: usize,
        stack_index: Option<usize>,
    ) -> Result<(), Error> {
        let stack_index = match stack_index {
            None => ActionMsgRange::generate_stack_choice_str(&[StackType::Draw], self, false)?
                .get_action()?,
            Some(c) => c,
        };
        match self.stacks.get_mut(stack_index) {
            Some(stack) => {
                let card = match stack.pop() {
                    Some(c) => c,
                    None => {
                        self.refill_drawable_stacks(stack_index, true)?;
                        self.stacks
                            .get_mut(stack_index)
                            .unwrap()
                            .pop()
                            .ok_or(Error::NotEnoughCards)?
                    }
                };
                match self.players.get_mut(player_index) {
                    Some(player) => {
                        player.get_cards_mut().push(card.to_owned());
                        Ok(())
                    }
                    None => Err(Error::InvalidPlayerIndex {
                        player_index,
                        len: self.players.len(),
                    }),
                }
            }
            None => Err(Error::StackNotFound {
                stack_index,
                len: self.stacks.len(),
            }),
        }
    }
}

// players' actions
impl Mao {
    pub fn player_draws_card(
        mao: &mut Mao,
        player_index: usize,
    ) -> anyhow::Result<MaoActionResult> {
        let mut return_value = MaoActionResult::Nothing;
        let mut stacks = mao.get_specific_stacks(&[StackType::Draw]);
        // let nb_cards = ActionMsgRange::generate_nb_cards_draw_choice_str(30).get_action()?;
        let mut nb_cards = 1;
        // TODO ask if can draw as much cards

        let (mut stack_index, mut stack) =
            Self::draw_stack_getter(mao, &stacks)?.ok_or(Error::NoStackAvailable {
                stacks: vec![StackType::Draw],
            })?;

        // TODO handle multiple cards draw
        while nb_cards > 0 {
            nb_cards -= 1;
            // get top card of stack
            if let Some(card) = stack.top() {
                let event = MaoEvent::DrawedCardEvent(CardEvent::new(
                    card.to_owned(),
                    player_index,
                    Some(stack_index),
                ));

                drop(stack);
                drop(stacks);

                // call rules
                let res = mao.on_event(event)?;
                // remove card from stack and give it to player if all rules have ignored it
                if res.iter().all(|v| match v {
                    MaoEventResult::Ignored => true,
                    _ => false,
                }) {
                    mao.give_card_to_player(player_index, Some(stack_index))?;
                } else {
                    if return_value == MaoActionResult::Nothing {
                        return_value =
                            if res.iter().any(|v| v == &MaoEventResult::OverrideCommonRule) {
                                MaoActionResult::ChangedTurn
                            } else {
                                MaoActionResult::Nothing
                            };
                    }
                }
            } else {
                drop(stack);
                drop(stacks);
                // have to refill the draw stack
                mao.refill_drawable_stacks(stack_index, true)?;
            }
            stacks = mao.get_specific_stacks(&[StackType::Draw]);
            (stack_index, stack) =
                Self::draw_stack_getter(mao, &stacks)?.ok_or(Error::NoStackAvailable {
                    stacks: vec![StackType::Draw],
                })?;
        }
        Ok(return_value)
    }

    pub fn player_plays_card(
        mao: &mut Mao,
        player_index: usize,
    ) -> anyhow::Result<MaoActionResult> {
        // getting player move
        let mut stack_index: Option<usize> = Some(
            ActionMsgRange::generate_stack_choice_str(&[StackType::PlayedStack], mao, true)?
                .get_action()?,
        );
        if stack_index.as_ref().unwrap() == &std::usize::MAX {
            stack_index = None;
        }
        let player = mao.get_players().get(player_index).unwrap();
        let card_index = ActionMsgRange::generate_card_choice_str(player).get_action()?;
        let card = player.get_cards().get(card_index).unwrap().to_owned();

        // calling rules
        let event =
            MaoEvent::PlayedCardEvent(CardEvent::new(card.to_owned(), player_index, stack_index));
        let res = mao.on_event(event)?;
        // one rule did not ignored it
        if !res.iter().all(|r| match r {
            MaoEventResult::Ignored => true,
            _ => false,
        }) {
            let mut overrided = false;
            for mao_res in &res {
                match mao_res {
                    MaoEventResult::Disallow(disallow) => {
                        println!(
                            "Rule '{}' disallowed your action : {}",
                            disallow.rule, disallow.msg
                        )
                    }
                    MaoEventResult::Handled => (),
                    MaoEventResult::Ignored => unreachable!(),
                    MaoEventResult::OverrideCommonRule => overrided = true,
                }
            }
            return Ok(if overrided {
                MaoActionResult::ChangedTurn
            } else {
                MaoActionResult::Nothing
            });
        }

        // no interactions from external rules

        // check from official rules
        if mao.can_play(
            player_index,
            &card,
            stack_index.and_then(|i| mao.get_stacks().get(i)),
        ) != PlayerTurnResult::CanPlay
        {
            match mao.get_specific_stacks_mut(&[StackType::Draw]).first() {
                Some(stack) => mao.give_card_to_player(player_index, stack_index)?, // TODO
                None => {
                    return Err(Error::NoStackAvailable {
                        stacks: vec![StackType::Draw],
                    }
                    .into())
                }
            }
        }
        // TODO
        // push card into played stack
        if let Some(stack_index) = stack_index {
            mao.get_stacks_mut()
                .get_mut(stack_index)
                .unwrap()
                .get_cards_mut()
                .push(card.to_owned());
        } else {
            // insert new stack
            mao.new_played_stack(&[card.to_owned()], true)
        }
        // remove card from player's hand
        mao.get_players_mut()
            .get_mut(player_index)
            .unwrap()
            .get_cards_mut()
            .remove(card_index);

        Ok(MaoActionResult::Nothing)
    }

    pub fn player_giveup_turn(_: &mut Mao, _: usize) -> anyhow::Result<MaoActionResult> {
        Ok(MaoActionResult::Nothing)
    }
}

pub fn generate_common_draw() -> Vec<Card> {
    let types = &[
        CommonCardType::Spade,
        CommonCardType::Diamond,
        CommonCardType::Club,
        CommonCardType::Heart,
    ];
    let mut cards = Vec::new();
    for i in 1..=13 {
        for t in types {
            cards.push(Card::new(
                CardValue::Number(i as isize),
                CardType::Common(t.to_owned()),
                None,
            ));
        }
    }
    cards.shuffle(&mut thread_rng());
    cards
}
