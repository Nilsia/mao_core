use core::result::Result;

use rand::{seq::SliceRandom, thread_rng};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    ops::DerefMut,
    path::PathBuf,
};

use crate::{
    card::{card_type::CardType, card_value::CardValue, common_card_type::CommonCardType, Card},
    config::Config,
    error::{DmDescription, Error},
    mao_event::{
        card_event::CardEvent,
        mao_event_result::{CallbackFunction, Disallow, MaoEventResult, MaoEventResultType},
        MaoEvent, StackTarget,
    },
    player::Player,
    rule::Rule,
    stack::{stack_property::StackProperty, stack_type::StackType, Stack},
};

use super::{
    automaton::{Automaton, MaoInteractionResult, NodeState, PlayerAction},
    mao_action::MaoInteraction,
};

pub fn log(msg: &[u8]) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("log.log")?;
    file.write_all(msg)?;
    file.write_all(&[10])?;
    Ok(())
}

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
        card_to_play: Card,
        card_on_stack: Card,
    },
    Other {
        desc: String,
    },
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestData {
    pub data_type: RequestDataEnum,
}

impl RequestData {
    pub fn new(data_type: RequestDataEnum) -> Self {
        Self { data_type }
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestDataEnum {
    StackChoice {
        stack_types: Vec<StackType>,
    },
    PlayerCardChoice {
        /// the index of the [`Player`] who has to chose
        player_chooser_index: usize,
        /// allows the player to choose a card among the other players' hand, if None the choice will be among it own cards
        among_other_players: Option<Vec<usize>>,
    },
}
pub enum RequestResponse {
    StackChoice(usize),
    PlayerCardChoice {
        /// the index of the [`Player`] who has choosen
        player_chooser_index: usize,
        /// the index of the choosen [`Player`], if None it is the player itself
        player_choosen_index: Option<usize>,
        /// the index of the card
        card_index: usize,
    },
}

pub type RequestCallback =
    fn(mao: &mut MaoCore, data: RequestData) -> anyhow::Result<RequestResponse>;
/// (column, row)
pub type Coords = (usize, usize);

#[derive(Debug)]
pub struct UiCallbacks {
    pub prompt_coords: fn() -> anyhow::Result<Coords>,
}

pub struct MaoCore {
    available_rules: Vec<Rule>,
    activated_rules: Vec<usize>,
    stacks: Vec<Stack>,
    players: Vec<Player>,
    player_turn: usize,
    /// the direction of the turn (-1 OR 1)
    turn: isize,
    /// all the events of a player that it did during its turn
    player_events: Vec<MaoEvent>,
    can_play_on_new_stack: bool,
    automaton: Automaton,
    dealer: usize,
}

// getters and setters
impl MaoCore {
    pub fn player_won(&self) -> Option<(usize, &Player)> {
        self.players
            .iter()
            .enumerate()
            .find(|(_, player)| player.get_cards().is_empty())
    }
    pub fn available_rules(&self) -> &[Rule] {
        &self.available_rules
    }
    pub fn activated_rules_indexes(&self) -> &[usize] {
        &self.activated_rules
    }
    pub fn players_events(&self) -> &[MaoEvent] {
        &self.player_events
    }
    pub fn dealer(&self) -> usize {
        self.dealer
    }
    pub fn set_dealer(&mut self, dealer: usize) {
        self.dealer = dealer;
    }
    pub fn automaton_mut(&mut self) -> &mut Automaton {
        &mut self.automaton
    }
    pub fn automaton(&self) -> &Automaton {
        &self.automaton
    }
    pub fn common_penality_to_player(&mut self, player_index: usize) -> Result<(), Error> {
        let card = self.draw_multiple_cards_unchosen(1)?.pop().unwrap();
        let len = self.players.len();
        match self.players.get_mut(player_index) {
            Some(player) => {
                player.get_cards_mut().push(card);
                Ok(())
            }
            None => Err(Error::InvalidPlayerIndex { player_index, len }),
        }
    }

    fn correct_player_action(&self, expected: &[PlayerAction], datas: &[PlayerAction]) -> bool {
        if expected.len() != datas.len() {
            return false;
        }
        if expected.iter().zip(datas).any(|(ver, data)| ver != data) {
            return false;
        }
        true
    }

    fn draw_interaction(
        player_index: usize,
        mao: &mut MaoCore,
        interactions: &[MaoInteraction],
    ) -> anyhow::Result<Vec<Disallow>> {
        let required = vec![PlayerAction::SelectDrawableStack];
        if !mao.correct_player_action(
            &required,
            &interactions
                .iter()
                .map(|v| v.action.to_owned())
                .collect::<Vec<PlayerAction>>(),
        ) {
            // TODO
            return Ok(Vec::new());
        }

        Ok(mao.on_draw_card(CardEvent {
            played_card: Card::default(),
            card_index: 0,
            player_index,
            stack_index: interactions[0].index,
        })?)
        // mao.give_card_to_player_no_rules_check(mao.player_turn, interactions[0].index)
        //     .unwrap();
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
    pub fn from_config(config: &Config) -> Result<Self, Error> {
        let path = PathBuf::from(&config.dirname);
        config.verify()?;
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

        let actions = vec![
            vec![
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectCard),
                    None,
                    None,
                ),
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectPlayableStack),
                    Some(|player_index, mao, datas| {
                        MaoCore::play_interaction(player_index, mao, datas)
                    }),
                    None,
                ),
            ],
            vec![NodeState::new(
                MaoInteraction::new(None, PlayerAction::SelectDrawableStack),
                Some(|player_index, mao, datas| {
                    MaoCore::draw_interaction(player_index, mao, datas)
                }),
                None,
            )],
        ];
        let mut s = Self::new(
            libraries,
            Self::init_stacks(),
            Vec::new(),
            Automaton::from_iter(actions),
        );
        // verify that all rules are valid
        // TODO just not put rules that are not valid in the carbage
        if let Err(e) = s.rules_valid() {
            return Err(Error::DlOpen2 {
                desc: DmDescription(
                    e.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>()
                        .join("\n"),
                ),
            });
        }
        // TODO removed
        // s.activate_rule("petit_pique_grand_coeur")?;

        Ok(s)
    }

    pub fn init_stacks() -> Vec<Stack> {
        let mut stacks = vec![Stack::new(
            Self::generate_common_draw(),
            false,
            vec![StackType::Drawable, StackType::Discardable],
        )];
        let first_card = stacks.first_mut().unwrap().draw_card().unwrap();
        stacks.push(Stack::new(
            vec![first_card],
            true,
            vec![StackType::Playable],
        ));
        stacks.push(Stack::new(vec![], true, vec![StackType::Discardable]));
        stacks
    }

    pub fn get_can_play_on_new_stack(&self) -> bool {
        self.can_play_on_new_stack
    }

    pub fn get_player_hand_len(&self, player_index: usize) -> Result<usize, Error> {
        Ok(self
            .players
            .get(player_index)
            .ok_or(Error::InvalidPlayerIndex {
                player_index,
                len: self.players.len(),
            })?
            .get_cards()
            .len())
    }

    pub fn player_turn(&self) -> usize {
        self.player_turn
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn players_mut(&mut self) -> &mut Vec<Player> {
        &mut self.players
    }

    pub fn stacks(&self) -> &[Stack] {
        &self.stacks
    }

    pub fn stacks_mut(&mut self) -> &mut Vec<Stack> {
        &mut self.stacks
    }

    pub fn new(
        available_libraries: Vec<Rule>,
        stacks: Vec<Stack>,
        players: Vec<Player>,
        automaton: Automaton,
    ) -> Self {
        Self {
            available_rules: available_libraries,
            activated_rules: Vec::new(),
            stacks,
            players,
            player_turn: 1,
            turn: 1,
            player_events: Vec::new(),
            can_play_on_new_stack: false,
            automaton,
            dealer: 0,
        }
    }

    pub fn on_action(&mut self, interaction: MaoInteraction) -> MaoInteractionResult {
        self.automaton.on_action(interaction)
    }

    fn on_penality(&mut self, player_index: usize) -> anyhow::Result<()> {
        let event = MaoEvent::PlayerPenality {
            player_target: player_index,
        };
        let res = self.on_event(&event)?;
        // all rules have ignored the penality
        if res
            .iter()
            .all(|ev| matches!(ev.res_type, MaoEventResultType::Ignored))
        {
            self.common_penality_to_player(player_index)?;
        }
        Ok(())
    }

    fn on_play_card(&mut self, card_event: CardEvent) -> Result<Vec<Disallow>, Error> {
        let event = MaoEvent::PlayedCardEvent(card_event.to_owned());

        // calling rules
        let res = self.on_event(&event)?;

        let disallows = self.propagate_on_event_results_and_execute(
            card_event.player_index,
            &event,
            &res.iter().map(|v| v).collect::<Vec<&MaoEventResult>>(),
        )?;
        if !disallows.is_empty() {
            for disallow in &disallows {
                if let Some(func) = disallow.penality {
                    func(self, card_event.player_index)?;
                } else {
                    self.on_penality(card_event.player_index)?;
                }
            }
            return Ok(disallows);
        }
        // no interactions from external rules
        // check from official rules
        let player_turn_res = self.can_play(
            card_event.player_index,
            &card_event.played_card,
            card_event.stack_index.and_then(|i| self.stacks().get(i)),
        );
        if !matches!(player_turn_res, PlayerTurnResult::CanPlay) {
            self.on_penality(card_event.player_index)?;
            self.next_player(card_event.player_index, &event, true);
            let msg: String = match player_turn_res {
                PlayerTurnResult::CanPlay => unreachable!("can play"),
                PlayerTurnResult::WrongTurn => "It is not your turn".to_string(),
                PlayerTurnResult::CannotPlaceThisCard {
                    card_to_play: placed_card,
                    ..
                } => format!(
                    "You cannot play this card {}",
                    placed_card.to_string_light()
                ),
                PlayerTurnResult::Other { desc } => desc.to_owned(),
            };
            return Ok(vec![Disallow::new("Basic Rules".to_string(), msg, None)]);
        } else {
            // player can play
            // push card into played stack
            if let Some(stack_index) = card_event.stack_index {
                self.push_card_into_stack_target(
                    StackTarget::Stack(stack_index),
                    card_event.played_card,
                )?;
            } else {
                // insert new stack
                self.new_played_stack(&[card_event.played_card.to_owned()], true)
            }
            // remove card from player's hand
            self.remove_card_from_stack_target(
                StackTarget::Player(card_event.player_index),
                card_event.card_index,
            )?;
            self.next_player(card_event.player_index, &event, false);
        }

        Ok(Vec::new())
    }

    fn play_interaction(
        player_index: usize,
        mao: &mut MaoCore,
        interactions: &[MaoInteraction],
    ) -> anyhow::Result<Vec<Disallow>> {
        let expected = vec![PlayerAction::SelectCard, PlayerAction::SelectPlayableStack];
        if !mao.correct_player_action(
            &expected,
            &interactions
                .iter()
                .map(|v| v.action.to_owned())
                .collect::<Vec<PlayerAction>>(),
        ) {
            // TODO
            return Err(Error::InvalidMaoInteraction {
                expected,
                received: interactions.iter().map(|v| v.action.to_owned()).collect(),
            }
            .into());
        }
        if interactions[0].index.is_none() {
            return Err(Error::InvalidCardIndex {
                card_index: usize::MAX,
                len: 0,
            }
            .into());
        }
        let player = mao
            .players
            .get(player_index)
            .ok_or(Error::InvalidPlayerIndex {
                player_index,
                len: mao.players.len(),
            })
            .unwrap(); // TODO remove
        let card = player
            .get_cards()
            .get(interactions[0].index.unwrap())
            .ok_or(Error::InvalidCardIndex {
                card_index: interactions[0].index.unwrap(),
                len: player.get_cards().len(),
            })
            .unwrap()
            .to_owned(); // TODO
        if interactions[1]
            .index
            .is_some_and(|index| mao.stacks.get(index).is_none())
        {
            return Err(Error::InvalidStackIndex {
                stack_index: interactions[1].index.unwrap(),
                len: mao.stacks.len(),
            }
            .into()); // TODO
        }
        Ok(mao.on_play_card(CardEvent {
            card_index: interactions[0].index.unwrap(),
            played_card: card,
            player_index,
            stack_index: interactions[1].index,
        })?)
        // mao.playsc
    }

    pub fn set_can_play_on_new_stack(&mut self, can_play_on_new_stack: bool) {
        self.can_play_on_new_stack = can_play_on_new_stack;
    }
}

// miscellious
impl MaoCore {
    pub fn get_executed_actions(&self) -> Vec<&NodeState> {
        self.automaton.get_executed_actions()
    }
    /// This function will draw `nb` cards from all avaible drawable stacks,
    /// it returns a [`Vec`] with exactly `nb` cards
    ///
    /// # Errors
    ///
    /// This function will return an error if
    /// - it cannot find a drawable stack
    /// - there is not enough cards inside all drawable stacks together
    pub fn draw_multiple_cards_unchosen(&mut self, mut nb: usize) -> Result<Vec<Card>, Error> {
        let mut cards = Vec::with_capacity(nb);
        let mut empty_first = false;
        while nb != 0 {
            let drawable_stacks = self.get_drawable_stacks_mut();
            // all stack are empty
            if !drawable_stacks
                .iter()
                .any(|(_, stack)| stack.get_cards().len() != 0)
            {
                if empty_first {
                    return Err(Error::NotEnoughCards);
                } else {
                    self.refill_drawable_stacks(None, true)?;
                    empty_first = true;
                    continue;
                }
            }
            for (_, stack) in drawable_stacks {
                if stack.get_cards().len() >= nb {
                    let index = stack.get_cards().len().saturating_sub(nb);
                    cards.extend_from_slice(
                        &(*stack)
                            .get_cards_mut()
                            .drain(index..)
                            .collect::<Vec<Card>>(),
                    );

                    nb = 0;
                    break;
                } else {
                    nb -= stack.get_cards().len();
                    cards.append((*stack).get_cards_mut());
                }
            }

            self.refill_drawable_stacks(None, false)?;
        }
        Ok(cards)
    }

    /// Enable a rule according to its name, searching from the available rules
    ///
    /// # Errors
    ///
    /// This function will return an error if the [`Rule`] has not been found according to `rule_name`
    pub fn activate_rule(&mut self, rule_name: &str) -> Result<(), Error> {
        let rule_name = "lib".to_owned() + rule_name;
        let rule_index = self
            .get_avalaible_rule_by_name(&rule_name)
            .ok_or_else(|| Error::RuleNotFound {
                desc: DmDescription(format!("The rule {} has not been found", rule_name)),
            })?
            .0;
        self.activated_rules.push(rule_index);
        Ok(())
    }

    pub fn activate_rule_by_index(&mut self, index: usize) -> Result<(), Error> {
        // the index des not correspond to an available rule
        match self.available_rules.get(index) {
            Some(rule) => {
                // the rule has already been activated
                if self.activated_rules.contains(&index) {
                    return Err(Error::RuleAlreadyActivated {
                        rule_name: rule.name().to_owned(),
                    });
                }

                if let Some(actions) = rule.get_actions() {
                    self.automaton.extend(actions);
                }
                self.activated_rules.push(index);
                Ok(())
            }
            None => Err(Error::InvalidRuleIndex {
                rule_index: index,
                len: self.available_rules.len(),
            }),
        }
    }

    pub fn deactivate_rule_by_index(&mut self, index: usize) -> Result<(), Error> {
        // TODO remove actions that the rule added
        // the index des not correspond to an available rule
        if self.available_rules.get(index).is_none() {
            return Err(Error::InvalidRuleIndex {
                rule_index: index,
                len: self.available_rules.len(),
            });
        }

        // the rule has already been activated
        if !self.activated_rules.contains(&index) {
            return Err(Error::RuleNotActivated {
                rule_name: self.available_rules.get(index).unwrap().name().to_owned(),
            });
        }
        self.activated_rules.retain(|&id| id != index);
        Ok(())
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
                        card_to_play: card.to_owned(),
                        card_on_stack: top_card.to_owned(),
                    };
                }
            }
        }

        PlayerTurnResult::CanPlay
    }

    /// Returns the [`Rule`] which as to be activated according to `rule_name`
    #[allow(dead_code)]
    fn get_activated_rule_by_name(&self, rule_name: &str) -> Option<(usize, &Rule)> {
        self.get_avalaible_rule_by_name(rule_name)
            .and_then(|(i, rule)| self.activated_rules.get(i).and(Some((i, rule))))
    }

    /// Returns the [`Rule`] from all rules according to `rule_name`
    fn get_avalaible_rule_by_name(&self, rule_name: &str) -> Option<(usize, &Rule)> {
        MaoCore::get_rule_by_light_filename(&self.available_rules, rule_name)
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

    /// Returns a [`Rule`] according to `rule_name` with its index
    /// if the rule is not present None is returned
    fn get_rule_by_light_filename<'t>(
        rules: &'t [Rule],
        rule_light_filename: &str,
    ) -> Option<(usize, &'t Rule)> {
        rules
            .iter()
            .enumerate()
            .filter(|rule| rule.1.light_filename() == rule_light_filename)
            .collect::<Vec<(usize, &Rule)>>()
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
    // pub fn give_card_to_player(
    //     &mut self,
    //     player_index: usize,
    //     stack_index: Option<usize>,
    //     ui: DynMaoArc,
    // ) -> Result<(), Error> {
    //     let stack_index = match stack_index {
    //         None => {
    //             let drawable_stacks = self.get_drawable_stacks();
    //             match drawable_stacks.len() {
    //                 0 => {
    //                     return Err(Error::NoStackAvailable {
    //                         stacks: vec![StackType::Drawable],
    //                     })
    //                 }
    //                 1 => drawable_stacks.first().unwrap().0,
    //                 _ => {
    //                     match ui.request_stack_choice(
    //                         self,
    //                         RequestData::new(RequestDataEnum::StackChoice {
    //                             stack_types: vec![StackType::Drawable],
    //                         }),
    //                     )? {
    //                         RequestResponse::StackChoice(i) => i,
    //                         _ => return Err(Error::InvalidRequestResponse),
    //                     }
    //                 }
    //             }
    //         }
    //         Some(c) => c,
    //     };
    //     match self.stacks.get_mut(stack_index) {
    //         Some(stack) => {
    //             let card = match stack.pop() {
    //                 Some(c) => c,
    //                 None => {
    //                     self.refill_drawable_stacks(stack_index, true)?;
    //                     self.stacks
    //                         .get_mut(stack_index)
    //                         .unwrap()
    //                         .pop()
    //                         .ok_or(Error::NotEnoughCards)?
    //                 }
    //             };
    //             match self.players.get_mut(player_index) {
    //                 Some(player) => {
    //                     player.get_cards_mut().push(card.to_owned());
    //                     Ok(())
    //                 }
    //                 None => Err(Error::InvalidPlayerIndex {
    //                     player_index,
    //                     len: self.players.len(),
    //                 }),
    //             }
    //         }
    //         None => Err(Error::InvalidStackIndex {
    //             stack_index,
    //             len: self.stacks.len(),
    //         }),
    //     }
    // }

    fn give_card_to_player_no_rules_check(
        &mut self,
        player_index: usize,
        stack_index: usize,
    ) -> Result<(), Error> {
        match self.stacks.get_mut(stack_index) {
            Some(stack) => {
                let card = match stack.pop() {
                    Some(c) => c,
                    None => {
                        self.refill_drawable_stacks(Some(stack_index), true)?;
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
        Ok(Player::new(
            pseudo,
            self.draw_multiple_cards_unchosen(nb_card)?,
        ))
    }

    pub fn init_players(&mut self, pseudos: &[String], nb_card: usize) -> Result<(), Error> {
        let mut players = Vec::with_capacity(pseudos.len());
        for pseudo in pseudos {
            players.push(self.init_player(pseudo.to_owned(), nb_card)?);
        }
        self.players = players;
        Ok(())
    }

    pub fn init_all_players(&mut self, nb_card: usize) -> Result<(), Error> {
        for i in 0..self.players.len() {
            let cards: Vec<Card> = self
                .draw_multiple_cards_unchosen(nb_card)?
                .iter()
                .cloned()
                .collect();
            self.players
                .get_mut(i)
                .unwrap()
                .get_cards_mut()
                .extend(cards);
        }
        Ok(())
    }

    pub fn init_new_game(&mut self, nb_card: usize) -> Result<(), Error> {
        // TODO set dealer
        for player in self.players.iter_mut() {
            player.get_cards_mut().clear();
        }
        self.stacks = Self::init_stacks();
        self.player_events.clear();
        self.automaton.reset();

        self.init_all_players(nb_card)?;
        Ok(())
    }

    /// Add a new played stack filled with the given `cards`
    pub fn new_played_stack(&mut self, cards: &[Card], visible: bool) {
        self.stacks.push(Stack::new(
            cards.to_owned(),
            visible,
            vec![StackType::Playable],
        ))
    }

    /// Handle turn change when a [`MaoEvent`] occurs, player index is used to check if it is the player turn
    pub fn next_player(&mut self, player_index: usize, event: &MaoEvent, took_penality: bool) {
        match event {
            MaoEvent::PlayedCardEvent(card_event) => {
                // no need to remove / add cards handled before
                // TODO try to call rules if some want to change the turn
                if player_index == self.player_turn {
                    if took_penality {
                        self.update_turn(PlayerTurnChange::default());
                        return;
                    }
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
            MaoEvent::PlayerPenality { .. } => (),
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
            results.push(self
                .available_rules
                .get(self.activated_rules[i])
                .unwrap()
                .get_on_event_func()(&event, self)?);
        }
        Ok(results)
    }

    /// This function calls callback functions after all first rules execution
    /// This function returns all [`Disallow`] inside `event_results`
    /// this means that if the action choice was approved by all rules, the [`Vec`] will be empty
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn propagate_on_event_results_and_execute(
        &mut self,
        player_index: usize,
        previous_event: &MaoEvent,
        event_results: &[&MaoEventResult],
    ) -> Result<Vec<Disallow>, Error> {
        if event_results
            .iter()
            .any(|r| !matches!(r.res_type, MaoEventResultType::Ignored,))
        {
            let mut disallows: Vec<Disallow> = Vec::new();
            // one rule did not ignored it
            let mut not_ignored: Vec<&MaoEventResult> = Vec::new();
            for mao_res in event_results {
                match &mao_res.res_type {
                    MaoEventResultType::Disallow(disallow) => {
                        disallows.push(disallow.to_owned());
                        // disallow.print_warning(Arc::clone(&ui))?;
                        // match disallow.penality.as_ref() {
                        //     Some(func) => func(self, player_index, Arc::clone(&ui))?,
                        //     None => {
                        //         self.give_card_to_player(player_index, None, Arc::clone(&ui))?
                        //     }
                        // }
                    }
                    MaoEventResultType::Ignored => (),
                    MaoEventResultType::OverrideBasicRule(_)
                    | MaoEventResultType::ExecuteBeforeTurnChange(_)
                    | MaoEventResultType::ExecuteAfterTurnChange(_) => not_ignored.push(mao_res),
                }
            }

            let mut results_on_results = Vec::new();
            for res in event_results {
                if let Some(func) = res.other_rules_callback.as_ref() {
                    results_on_results.push(func(self, &previous_event, &not_ignored)?);
                }
            }
            for res in &results_on_results {
                match res.res_type {
                    // cannot disallow the action of a rule
                    MaoEventResultType::Ignored | MaoEventResultType::Disallow(_) => (),
                    MaoEventResultType::OverrideBasicRule(_)
                    | MaoEventResultType::ExecuteBeforeTurnChange(_)
                    | MaoEventResultType::ExecuteAfterTurnChange(_) => not_ignored.push(&res),
                }
            }

            {
                let overrided = not_ignored
                    .iter()
                    .any(|v| !matches!(v.res_type, MaoEventResultType::OverrideBasicRule(_)));
                let (before, after) = not_ignored.iter().fold(
                    (
                        Vec::<&CallbackFunction>::new(),
                        Vec::<&CallbackFunction>::new(),
                    ),
                    |(mut bef, mut aft), v| {
                        match &v.res_type {
                            MaoEventResultType::ExecuteBeforeTurnChange(f) => bef.push(&f),
                            MaoEventResultType::ExecuteAfterTurnChange(f) => aft.push(&f),
                            _ => (),
                        }
                        (bef, aft)
                    },
                );

                for bef in &before {
                    bef(self, player_index)?;
                }
                if !overrided {
                    self.next_player(player_index, previous_event, false);
                }
                for aft in &after {
                    aft(self, player_index)?;
                }
            }
            return Ok(disallows);
        }
        Ok(Vec::new())
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

    /// Returns all top cards of the Playable [`Stack`]s as String followed by the index of the stack
    pub fn all_top_card_playable_stacks_string(&self) -> Vec<(usize, String)> {
        self.get_playable_stacks()
            .iter()
            .map(|(i, stack)| {
                (
                    *i,
                    stack
                        .top()
                        .map(|v| v.to_string())
                        .unwrap_or(String::from("empty")),
                )
            })
            .collect()
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

    /// if `stack_index` is None, the first drawable stack will be gotten
    ///
    /// this function does not edit the length of stacks
    ///
    /// # Error
    ///
    /// fails if stack_index is out if range or if there is no drawable stack available
    pub fn refill_drawable_stacks(
        &mut self,
        stack_index: Option<usize>,
        check_rules: bool,
    ) -> Result<(), Error> {
        // checking rules before refilling the stack
        let stack_index = stack_index.unwrap_or(
            self.get_drawable_stacks()
                .first()
                .ok_or(Error::NoStackAvailable {
                    stacks: vec![StackType::Drawable],
                })?
                .0,
        );
        if check_rules {
            let event = MaoEvent::StackPropertyRunsOut {
                empty_stack_index: StackTarget::Stack(stack_index),
            };
            if self.on_event(&event)?.iter().any(|r| match r.res_type {
                MaoEventResultType::Ignored => false,
                _ => true,
            }) {
                return Ok(());
            }
        }

        let mut stacks_spe =
            self.get_specific_stacks_mut(&[StackType::Playable, StackType::Discardable]);
        let mut cards = Vec::with_capacity(
            stacks_spe
                .iter()
                .map(|(_, stack)| stack.get_cards().len())
                .sum(),
        );
        // foreach add to cards and clear stacks
        for i in 0..stacks_spe.len() {
            let (_, stack) = stacks_spe.get_mut(i).unwrap();
            if stack.get_stack_types().contains(&StackType::Playable) {
                let last_card = stack.pop();
                if let Some(last_card) = last_card {
                    cards.append(stack.deref_mut());
                    stack.deref_mut().push(last_card);
                }
            } else {
                cards.append(stack.deref_mut());
            }
        }
        // refill drawable stack
        let len = self.stacks.len();
        let stack = self
            .stacks
            .get_mut(stack_index)
            .ok_or(Error::InvalidStackIndex { stack_index, len })?;
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

    pub fn get_none_empty_drawable_stack_mut(&mut self) -> Option<(usize, &mut Stack)> {
        let mut stacks = self.get_drawable_stacks_mut();
        let x = stacks
            .drain(..)
            .flat_map(|(i, stack)| {
                if stack.is_empty() {
                    None
                } else {
                    Some((i, stack))
                }
            })
            .next();
        x
    }
    pub fn get_none_empty_drawable_stack(&self) -> Option<(usize, &Stack)> {
        let stacks = self.get_drawable_stacks();
        let x = stacks
            .iter()
            .flat_map(|(i, stack)| {
                if stack.is_empty() {
                    None
                } else {
                    Some((*i, *stack))
                }
            })
            .next();
        x
    }

    fn verify_or_get_none_empty_drawable_stack(
        &mut self,
        stack_index: Option<usize>,
    ) -> Result<usize, Error> {
        Ok(match stack_index {
            Some(index) => {
                let stack = self.stacks.get(index).ok_or(Error::InvalidStackIndex {
                    stack_index: index,
                    len: self.stacks.len(),
                })?;
                // stack is empty refill it
                if stack.is_empty() {
                    self.refill_drawable_stacks(stack_index, true)?;
                    let stack = self.stacks.get(index).unwrap();
                    if stack.is_empty() {
                        return Err(Error::NotEnoughCards);
                    }
                }
                index
            }
            None => match self.get_none_empty_drawable_stack_mut() {
                Some((i, _)) => i,
                None => {
                    self.refill_drawable_stacks(None, true)?;
                    self.get_none_empty_drawable_stack()
                        .ok_or(Error::NoStackAvailable {
                            stacks: vec![StackType::Drawable],
                        })?
                        .0
                }
            },
        })
    }

    fn on_draw_card(&mut self, mut card_event: CardEvent) -> Result<Vec<Disallow>, Error> {
        // get the stack if present otherwise get drawable stack
        let stack_index = self.verify_or_get_none_empty_drawable_stack(card_event.stack_index)?;

        if self.players.get(card_event.player_index).is_none() {
            return Err(Error::InvalidPlayerIndex {
                player_index: card_event.player_index,
                len: self.players.len(),
            });
        }

        let card = self.stacks.get_mut(stack_index).unwrap().pop().unwrap();
        card_event.played_card = card.to_owned();
        card_event.stack_index = Some(stack_index);
        let event = MaoEvent::DrawedCardEvent(card_event.to_owned());
        // call rules
        let res = self.on_event(&event)?;
        // give the card to the player if all rules have ignored it
        if res
            .iter()
            .all(|v| matches!(v.res_type, MaoEventResultType::Ignored))
        {
            if self.player_turn == card_event.player_index {
                self.update_turn(PlayerTurnChange::Update(PlayerTurnUpdater::Update(1)));
            }
            // all rules have ignored the event
            self.players
                .get_mut(card_event.player_index)
                .unwrap()
                .get_cards_mut()
                .push(card.to_owned());
        } else {
            let mut values: Vec<&MaoEventResult> = Vec::new();
            // push back the card into the stack (been removed before)
            self.stacks.get_mut(stack_index).unwrap().push(card);
            for result in &res {
                if !matches!(&result.res_type, MaoEventResultType::Ignored) {
                    // MaoEventResultType::Disallow(d) => {
                    // d.print_warning(Arc::clone(&ui))?;
                    // }
                    values.push(result);
                }
            }
            // TODO
            let disallows = self.propagate_on_event_results_and_execute(
                card_event.player_index,
                &event,
                &values,
            )?;
            if !disallows.is_empty() {
                return Ok(disallows);
            }
            // if !values.is_empty() {
            //     return Ok(());
            // }
        }

        Ok(Vec::new())
    }
}

// players' actions
impl MaoCore {
    // pub fn on_mao_event(&mut self, mao_event: MaoEvent) -> Result<Vec<Disallow>, Error> {
    //     match mao_event {
    //         MaoEvent::PlayedCardEvent(e) => self.on_play_card(e),
    //         MaoEvent::DiscardCardEvent(_) => todo!(),
    //         MaoEvent::DrawedCardEvent(e) => self.on_draw_card(e),
    //         MaoEvent::GiveCardEvent {
    //             card,
    //             from_player_index,
    //             target,
    //         } => todo!(),
    //         MaoEvent::StackPropertyRunsOut { empty_stack_index } => todo!(),
    //         MaoEvent::GameStart => todo!(),
    //         MaoEvent::EndPlayerTurn { events } => todo!(),
    //         MaoEvent::VerifyEvent => todo!(),
    //         MaoEvent::PlayerPenality { player_target } => todo!(),
    //     }
    // }

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
}
