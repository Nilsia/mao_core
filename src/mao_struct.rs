use core::result::Result;
use rand::{seq::SliceRandom, thread_rng};
use std::{fs, ops::DerefMut, path::PathBuf};

use crate::{
    action_msg_range::ActionMsgRange,
    card::{
        card::Card, card_type::CardType, card_value::CardValue, common_card_type::CommonCardType,
    },
    config::Config,
    error::{DmDescription, Error},
    mao_event::{
        card_event::CardEvent,
        mao_event::{MaoEvent, StackTarget},
    },
    mao_event_result::{MaoEventResult, MaoEventResultType},
    player::Player,
    rule::Rule,
    stack::{stack::Stack, stack_property::StackProperty, stack_type::StackType},
};

#[derive(Debug)]
pub enum PlayerTurnUpdater {
    Set(usize),
    Update(isize),
}

impl Default for PlayerTurnUpdater {
    fn default() -> Self {
        Self::Update(1)
    }
}

#[derive(Debug)]
pub enum PlayerTurnChange {
    Update(PlayerTurnUpdater),
    Rotate(PlayerTurnUpdater),
}

impl Default for PlayerTurnChange {
    fn default() -> Self {
        Self::Update(PlayerTurnUpdater::default())
    }
}

#[derive(Clone, Debug)]
pub enum MaoActionResult {
    TurnAction {
        result: Vec<MaoEventResult>,
        event: MaoEvent,
    },
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
    turn: isize,
    player_events: Vec<MaoEvent>,
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
            turn: 1,
            player_events: Vec::new(),
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

    pub fn get_player_turn(&self) -> usize {
        self.player_turn
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
            vec![StackType::Drawable],
        )];
        let first_card = stacks.first_mut().unwrap().draw_card().unwrap();
        stacks.push(Stack::new(
            vec![first_card],
            true,
            vec![StackType::Playable],
        ));

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
            vec![StackType::Playable],
        ))
    }

    pub fn get_stack_target(
        &mut self,
        target_index: StackTarget,
    ) -> Result<&mut dyn StackProperty, Error> {
        let s: &mut dyn StackProperty = match target_index {
            StackTarget::Player(i) => {
                let len = self.players.len();
                self.players.get_mut(i).ok_or(Error::InvalidPlayerIndex {
                    player_index: i,
                    len,
                })?
            }
            StackTarget::Stack(i) => {
                let len = self.stacks.len();
                self.stacks.get_mut(i).ok_or(Error::InvalidStackIndex {
                    stack_index: i,
                    len,
                })?
            }
        };
        Ok(s)
    }

    pub fn remove_card_from_stack_target(
        &mut self,
        target_index: StackTarget,
        card_index: usize,
    ) -> Result<Card, Error> {
        self.get_stack_target(target_index)?.remove_card(card_index)
    }

    pub fn push_card_to_stack_target(
        &mut self,
        target_index: StackTarget,
        card: Card,
    ) -> Result<(), Error> {
        Ok(self.get_stack_target(target_index)?.add_card(card))
    }

    pub fn rules_valid(&self) -> bool {
        self.available_rules.iter().all(|l| l.is_valid_rule())
    }

    pub fn player_finish_turn(&mut self) -> Result<(), Error> {
        let event = MaoEvent::EndPlayerTurn {
            events: self.player_events.to_owned(),
        };
        self.on_event(&event)?;
        self.player_events.clear();
        Ok(())
    }

    pub fn on_event(&mut self, event: &MaoEvent) -> Result<Vec<MaoEventResult>, Error> {
        self.player_events.push(event.to_owned());
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

    pub fn get_playable_stacks(&self) -> Vec<(usize, &Stack)> {
        self.get_specific_stacks(&[StackType::Playable])
    }
    pub fn get_playable_stacks_mut(&mut self) -> Vec<(usize, &mut Stack)> {
        self.get_specific_stacks_mut(&[StackType::Playable])
    }

    pub fn get_drawable_stacks(&self) -> Vec<(usize, &Stack)> {
        self.get_specific_stacks(&[StackType::Drawable])
    }
    pub fn get_drawable_stacks_mut(&mut self) -> Vec<(usize, &mut Stack)> {
        self.get_specific_stacks_mut(&[StackType::Drawable])
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
                    ActionMsgRange::generate_stack_choice_str(&[StackType::Drawable], mao, false)?
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
        // checking rules before refilling the stack
        if check_rules {
            let event = MaoEvent::StackRunsOut {
                empty_stack_index: stack_index,
                removed_cards_number: 1,
            };
            if self
                .on_event(&event)?
                .iter()
                .any(|r| r.res_type != MaoEventResultType::Ignored)
            {
                return Ok(());
            }
        }

        let mut cards = Vec::new();
        let mut stacks_spe =
            self.get_specific_stacks_mut(&[StackType::Playable, StackType::Discardable]);
        // foreach add to cards and clear stacks
        let mut tmp: Vec<Card>;
        for i in 0..stacks_spe.len() {
            let stack_cards = stacks_spe.get_mut(i).unwrap().1.deref_mut();
            tmp = stack_cards
                .drain(..stack_cards.len() - 1)
                .collect::<Vec<Card>>();
            cards.extend_from_slice(&tmp);
        }
        // refill drawable stack
        let len = self.stacks.len();
        let stack = self
            .stacks
            .get_mut(stack_index)
            .ok_or(Error::InvalidStackIndex { stack_index, len })?;
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
            None => ActionMsgRange::generate_stack_choice_str(&[StackType::Drawable], self, false)?
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
            None => Err(Error::InvalidStackIndex {
                stack_index,
                len: self.stacks.len(),
            }),
        }
    }

    pub fn update_turn(&mut self, changes: PlayerTurnChange) {
        let nb_players = self.players.len();
        if let Some(step) = match changes {
            PlayerTurnChange::Update(v) => match v {
                PlayerTurnUpdater::Set(i) => {
                    self.player_turn = i;
                    None
                }
                PlayerTurnUpdater::Update(i) => Some(i),
            },
            PlayerTurnChange::Rotate(v) => match v {
                PlayerTurnUpdater::Set(i) => {
                    self.turn *= -1;
                    self.player_turn = i;
                    None
                }
                PlayerTurnUpdater::Update(i) => {
                    self.turn *= -1;
                    Some(i)
                }
            },
        } {
            self.player_turn = (self.player_turn as isize
                + (self.turn * step) % (nb_players as isize))
                .rem_euclid(nb_players as isize) as usize;
        }
    }

    pub fn next_player(&mut self, player_index: usize, event: MaoEvent) {
        println!("called");
        match event {
            MaoEvent::PlayedCardEvent(card_event) => {
                // no need to remove / add cards handled before
                if player_index == self.player_turn {
                    let changes: PlayerTurnChange = match card_event.card.get_value() {
                        CardValue::Number(i) => match i {
                            2 => PlayerTurnChange::Update(PlayerTurnUpdater::Update(2)),
                            10 => PlayerTurnChange::Rotate(PlayerTurnUpdater::Update(1)),

                            _ => PlayerTurnChange::default(),
                        },
                        CardValue::MinusInfinity => PlayerTurnChange::default(),
                        CardValue::PlusInfinity => PlayerTurnChange::default(),
                    };
                    println!("changes = {changes:?}");
                    self.update_turn(changes);
                }
            }
            MaoEvent::DiscardCardEvent(_) => todo!(),
            MaoEvent::DrawedCardEvent(_) => {
                if player_index == self.player_turn {
                    self.update_turn(PlayerTurnChange::default());
                }
            }
            MaoEvent::GiveCardEvent { .. } => (),
            MaoEvent::StackRunsOut { .. } => (),
            MaoEvent::GameStart => (),
            MaoEvent::EndPlayerTurn { .. } => (),
        }
    }
}

// players' actions
impl Mao {
    pub fn player_draws_card(
        mao: &mut Mao,
        player_index: usize,
    ) -> anyhow::Result<MaoActionResult> {
        let mut stacks = mao.get_specific_stacks(&[StackType::Drawable]);
        // let nb_cards = ActionMsgRange::generate_nb_cards_draw_choice_str(30).get_action()?;
        let mut nb_cards = 1;
        // TODO ask if can draw as much cards

        let (mut stack_index, mut stack) =
            Self::draw_stack_getter(mao, &stacks)?.ok_or(Error::NoStackAvailable {
                stacks: vec![StackType::Drawable],
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
                let res = mao.on_event(&event)?;
                // remove card from stack and give it to player if all rules have ignored it
                if res.iter().all(|v| match v.res_type {
                    MaoEventResultType::Ignored => true,
                    _ => false,
                }) {
                    // all rules have ignored the event
                    mao.give_card_to_player(player_index, Some(stack_index))?;
                } else {
                    let mut values: Vec<MaoEventResult> = Vec::new();
                    for result in &res {
                        match &result.res_type {
                            MaoEventResultType::Ignored => (),
                            MaoEventResultType::Disallow(d) => {
                                d.print_warning();
                            }
                            _ => values.push(result.to_owned()),
                        }
                    }
                    if !values.is_empty() {
                        return Ok(MaoActionResult::TurnAction {
                            result: values,
                            event: event.to_owned(),
                        });
                    }
                }
            } else {
                drop(stack);
                drop(stacks);
                // have to refill the draw stack
                mao.refill_drawable_stacks(stack_index, true)?;
            }
            stacks = mao.get_specific_stacks(&[StackType::Drawable]);
            (stack_index, stack) =
                Self::draw_stack_getter(mao, &stacks)?.ok_or(Error::NoStackAvailable {
                    stacks: vec![StackType::Drawable],
                })?;
        }
        Ok(MaoActionResult::Nothing)
    }

    pub fn player_plays_card(
        mao: &mut Mao,
        player_index: usize,
    ) -> anyhow::Result<MaoActionResult> {
        // getting player move
        let mut stack_index: Option<usize> = Some(
            ActionMsgRange::generate_stack_choice_str(&[StackType::Playable], mao, true)?
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
        let res = mao.on_event(&event)?;

        if !res.iter().all(|r| match r.res_type {
            MaoEventResultType::Ignored => true,
            _ => false,
        }) {
            // one rule did not ignored it
            let mut not_ignored = Vec::new();
            for mao_res in &res {
                match &mao_res.res_type {
                    MaoEventResultType::Disallow(disallow) => {
                        disallow.print_warning();
                    }
                    MaoEventResultType::Ignored => (),
                    _ => not_ignored.push(mao_res.to_owned()),
                }
            }
            return Ok(if !not_ignored.is_empty() {
                MaoActionResult::TurnAction {
                    result: not_ignored,
                    event: event.to_owned(),
                }
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
            // player cannot play
            match mao.get_specific_stacks(&[StackType::Drawable]).first() {
                Some((stack_index, _)) => {
                    mao.give_card_to_player(player_index, Some(*stack_index))?
                } // TODO
                None => {
                    return Err(Error::NoStackAvailable {
                        stacks: vec![StackType::Drawable],
                    }
                    .into())
                }
            }
            let player = mao.players.get(player_index).unwrap();
            println!(
                "{}, you cannot play this card : {}, you took one card",
                player.get_pseudo(),
                card.to_string()
            );
            player.print_self_cards(None, None)?;
        } else {
            // player can play
            // push card into played stack
            if let Some(stack_index) = stack_index {
                mao.push_card_to_stack_target(StackTarget::Stack(stack_index), card.to_owned())?;
            } else {
                // insert new stack
                mao.new_played_stack(&[card.to_owned()], true)
            }
            // remove card from player's hand
            mao.remove_card_from_stack_target(StackTarget::Player(player_index), card_index)?;
        }

        Ok(MaoActionResult::TurnAction {
            result: vec![],
            event,
        })
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
