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
    /// the direction of the turn (-1 OR 1)
    turn: isize,
    /// all the events of a player that it did during its turn
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

    pub fn new_light(libraries: Vec<Rule>, stacks: Vec<Stack>) -> Self {
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

    /// this function loads the Mao structure from the `config` argument
    ///
    /// # Panics
    ///
    /// Panics if some paths of `config` are not valid
    ///
    /// # Errors
    ///
    /// This function will return an error if
    /// + the config is not valid
    /// + a rule cannot be found
    pub fn from_directory(config: &Config) -> Result<Self, Error> {
        let path = PathBuf::from(&config.dirname);
        let rules: Vec<String> = fs::read_dir(&path)
            .map_err(|e| {
                Err(Error::InvalidConfig {
                    desc: e.to_string(),
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

        let mut s = Self::new_light(libraries, stacks);
        // verify that all rules are valid
        if let Err(e) = s.rules_valid() {
            return Err(Error::LibLoading {
                desc: DmDescription(
                    e.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>()
                        .join("\n"),
                ),
            });
        }

        Ok(s)
    }
}

// miscellious
impl Mao {
    /// Enable a rule according to its name, searching from the available rules
    ///
    /// # Errors
    ///
    /// This function will return an error if the [`Rule`] has not been found according to `rule_name`
    pub fn activate_rule(&mut self, rule_name: &str) -> Result<(), Error> {
        let rule_name = "lib".to_owned() + rule_name;
        Ok(self.activated_rules.push(
            (*self
                .get_avalaible_rule_by_name(&rule_name)
                .ok_or_else(|| Error::RuleNotFound {
                    desc: DmDescription(format!("The rule {} has not been found", rule_name)),
                })?)
            .to_owned(),
        ))
    }

    /// Checks if a player can play its card according to the initial Mao rules
    ///
    /// This function will firstly check if it is the player turn and therefore check the values and the color of the concerned card
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

    /// Retrieve a stack requested from the player
    ///
    /// If there is only one stack, not intaction with the player will occur, otherwise the player will have to make a choice
    ///
    /// # Errors
    ///
    /// This function will return an error if it is not possible to retrieve the player's choice
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

    /// Returns the [`Rule`] which as to be activated according to `rule_name`
    #[allow(dead_code)]
    fn get_activated_rule_by_name(&self, rule_name: &str) -> Option<&Rule> {
        Mao::get_rule_by_name(&self.activated_rules, rule_name)
    }

    /// Returns the [`Rule`] from all rules according to `rule_name`
    fn get_avalaible_rule_by_name(&self, rule_name: &str) -> Option<&Rule> {
        Mao::get_rule_by_name(&self.available_rules, rule_name)
    }

    /// Returns a [`Vec`] of a reference to a drawable stack and its index
    pub fn get_drawable_stacks(&self) -> Vec<(usize, &Stack)> {
        self.get_specific_stacks(&[StackType::Drawable])
    }

    /// Returns a [`Vec`] of a mutable reference to a drawable stack and its index
    pub fn get_drawable_stacks_mut(&mut self) -> Vec<(usize, &mut Stack)> {
        self.get_specific_stacks_mut(&[StackType::Drawable])
    }

    /// Returns a [`Vec`] of a reference to a playable stack and its index
    pub fn get_playable_stacks(&self) -> Vec<(usize, &Stack)> {
        self.get_specific_stacks(&[StackType::Playable])
    }

    /// Returns a [`Vec`] of a mutable reference to a playable stack and its index
    pub fn get_playable_stacks_mut(&mut self) -> Vec<(usize, &mut Stack)> {
        self.get_specific_stacks_mut(&[StackType::Playable])
    }

    /// Returns a [`Rule`] according to `rule_name`
    /// if the rule is not present None is returned
    fn get_rule_by_name<'a>(rules: &'a [Rule], rule_name: &str) -> Option<&'a Rule> {
        rules
            .iter()
            .filter(|rule| rule.get_name() == rule_name)
            .collect::<Vec<&Rule>>()
            .first()
            .map(|v| *v)
    }

    /// Returns the stacks which contain the given `stack_types` with their index
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

    /// Returns the mutable stacks which contain the given `stack_types` with their index
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

    /// Returns a mutable reference to a structure that implements [`StackProperty`]
    ///
    /// # Errors
    ///
    /// This function will return an error if the given indexes `target_index` are not valid
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

    /// Return the first [`Card`] of the [`Stack`] according to `stack_index`
    ///
    /// # Errors
    ///
    /// This function will return an error if `stack_index` is not valid
    pub fn get_top_card_playable_stack(&self, stack_index: usize) -> Result<Option<&Card>, Error> {
        self.stacks
            .get(stack_index)
            .ok_or(Error::InvalidStackIndex {
                stack_index,
                len: self.stacks.len(),
            })
            .map(|stack| stack.top())
    }

    /// This function allows you to give a card to a player
    /// if stack_index is given as None, the player will have to choice if there are more than one drawable stack
    ///
    ///
    /// # Errors
    ///
    /// This function will return an error if
    /// + there is not available drawable stacks,
    /// + cannot refill an empty drawable stack
    /// + the player index is invalid
    pub fn give_card_to_player(
        &mut self,
        player_index: usize,
        stack_index: Option<usize>,
    ) -> Result<(), Error> {
        let stack_index = match stack_index {
            None => {
                let drawable_stacks = self.get_drawable_stacks();
                match drawable_stacks.len() {
                    0 => {
                        return Err(Error::NoStackAvailable {
                            stacks: vec![StackType::Drawable],
                        })
                    }
                    1 => drawable_stacks.first().unwrap().0,
                    _ => ActionMsgRange::generate_stack_choice_str(
                        &[StackType::Drawable],
                        self,
                        false,
                    )?
                    .get_action()?,
                }
            }
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

    /// Initialize player by giving it `nb_card` [`Card`]s
    ///
    /// This function will search for drawable stacks and drain them into the player's hand
    ///
    /// # Errors
    ///
    /// This function will return an error if
    /// + there is not Drawable Stack available
    /// + there is not enough cards inside all Drawable Stacks
    pub fn init_player(&mut self, pseudo: String, nb_card: usize) -> Result<Player, Error> {
        let mut stacks = self.get_drawable_stacks_mut();
        let first_stack = stacks
            .first_mut()
            .map(|(_, s)| s)
            .ok_or(Error::NoStackAvailable {
                stacks: vec![StackType::Drawable],
            })?;
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

    /// Add a new played stack filled with the given `cards`
    pub fn new_played_stack(&mut self, cards: &[Card], visible: bool) {
        self.stacks.push(Stack::new(
            cards.to_owned(),
            visible,
            vec![StackType::Playable],
        ))
    }

    /// Handle turn change when a [`MaoEvent`] occurs
    pub fn next_player(&mut self, player_index: usize, event: MaoEvent) {
        match event {
            MaoEvent::PlayedCardEvent(card_event) => {
                // no need to remove / add cards handled before
                if player_index == self.player_turn {
                    let changes: PlayerTurnChange = match card_event.played_card.get_value() {
                        CardValue::Number(i) => match i {
                            2 => PlayerTurnChange::Update(PlayerTurnUpdater::Update(2)),
                            10 => PlayerTurnChange::Rotate(PlayerTurnUpdater::Update(1)),

                            _ => PlayerTurnChange::default(),
                        },
                        CardValue::MinusInfinity => PlayerTurnChange::default(),
                        CardValue::PlusInfinity => PlayerTurnChange::default(),
                    };
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
            MaoEvent::StackPropertyRunsOut { .. } => (),
            MaoEvent::GameStart => (),
            MaoEvent::EndPlayerTurn { .. } => (),
            MaoEvent::VerifyEvent => unreachable!("verify event"),
        }
    }

    /// Call [`Rule::on_event`] on each activated rules
    /// Returns all the results of the activated rules
    ///
    /// this function use unsafe code and could failed
    ///
    /// # Errors
    ///
    /// This function will return an error if
    /// + there is not `on_event` functions inside the rule (should never occur)
    /// + the `on_event` function from the rule fails
    pub fn on_event(&mut self, event: &MaoEvent) -> Result<Vec<MaoEventResult>, Error> {
        self.player_events.push(event.to_owned());
        let mut results = Vec::with_capacity(self.activated_rules.len());
        for i in 0..self.activated_rules.len() {
            unsafe {
                results.push(self
                    .activated_rules
                    .get_unchecked(i)
                    .get_on_event_func()?(&event, self)?);
            }
        }
        Ok(results)
    }

    /// Finish the turn of player
    ///
    /// this function will call [`Self::on_event`] with [`MaoEvent::EndPlayerTurn`]
    /// and then clear player's actions
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn player_finish_turn(&mut self) -> Result<(), Error> {
        let event = MaoEvent::EndPlayerTurn {
            events: self.player_events.to_owned(),
        };
        self.on_event(&event)?;
        self.player_events.clear();
        Ok(())
    }

    /// print all top cards of the Playable [`Stack`]
    pub fn print_all_top_card_playable_stacks(&self) {
        let playable_stacks = self.get_playable_stacks();
        for (i, stack) in playable_stacks {
            println!(
                "Stack ({}) : \n{}",
                i,
                stack
                    .top()
                    .map(|v| v.to_string())
                    .unwrap_or(String::from("empty"))
            );
        }
    }

    /// Add a new [`Card`] into the `target` according to [`StackTarget`]
    ///
    /// # Errors
    ///
    /// This function will return an error if the `stack_target` is invalid
    pub fn push_card_into_stack_target(
        &mut self,
        target_index: StackTarget,
        card: Card,
    ) -> Result<(), Error> {
        Ok(self.get_stack_target(target_index)?.add_card(card))
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
            let event = MaoEvent::StackPropertyRunsOut {
                empty_stack_index: StackTarget::Stack(stack_index),
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

    /// Removes a [`Card`] according to `card_index` of the target to suit `target_index`
    ///
    /// # Errors
    ///
    /// This function will return an error if `target_index` OR `card_index` is invalid
    pub fn remove_card_from_stack_target(
        &mut self,
        target_index: StackTarget,
        card_index: usize,
    ) -> Result<Card, Error> {
        self.get_stack_target(target_index)?.remove_card(card_index)
    }

    /// Return Ok(()) if all rules are valid and an [`Error`] otherwise
    ///
    /// this function check all rules and returns a [`Vec`] of Error if there a some
    ///
    /// # Errors
    ///
    /// This function will return an error if a rule is not valid as a [`Vec`]
    pub fn rules_valid(&mut self) -> Result<(), Vec<Error>> {
        let mut invalids = Vec::new();
        for i in 0..self.available_rules.len() {
            let rule = self.available_rules.get(i).unwrap().to_owned();
            if let Err(e) = rule.is_valid_rule(self) {
                invalids.push(e);
            }
        }
        if invalids.is_empty() {
            Ok(())
        } else {
            Err(invalids)
        }
    }

    /// Updates the player turn to suit `changes`
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
                        match disallow.penality {
                            Some(func) => func(mao, player_index)?,
                            None => mao.give_card_to_player(player_index, None)?,
                        }
                    }
                    MaoEventResultType::Ignored => (),
                    MaoEventResultType::OverrideBasicRule(_)
                    | MaoEventResultType::ExecuteBeforeTurnChange(_)
                    | MaoEventResultType::ExecuteAfterTurnChange(_) => {
                        not_ignored.push(mao_res.to_owned())
                    }
                }
            }

            for mao_res in &res {
                if let Some(func) = mao_res.other_rules_callback {
                    let callback_res = func(mao, &event, &not_ignored)?;
                    match callback_res.res_type {
                        MaoEventResultType::Ignored | MaoEventResultType::Disallow(_) => (),
                        MaoEventResultType::OverrideBasicRule(_)
                        | MaoEventResultType::ExecuteBeforeTurnChange(_)
                        | MaoEventResultType::ExecuteAfterTurnChange(_) => {
                            not_ignored.push(callback_res)
                        }
                    }
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
                "{}, as penality you took one card, you cannot play this card : \n{}",
                player.get_pseudo(),
                card.to_string()
            );
            player.print_self_cards(None, None)?;
        } else {
            // player can play
            // push card into played stack
            if let Some(stack_index) = stack_index {
                mao.push_card_into_stack_target(StackTarget::Stack(stack_index), card.to_owned())?;
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
