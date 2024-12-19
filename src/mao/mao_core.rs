use core::result::Result;

use rand::{seq::SliceRandom, thread_rng};
use serde::Deserialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    ops::{DerefMut, Range},
    path::PathBuf,
    str::FromStr,
};

use crate::{
    card::{card_type::CardType, card_value::CardValue, common_card_type::CommonCardType, Card},
    config::{CardEffectsKey, CardPlayerAction, Config, SingOrMult, SingleCardEffect},
    error::{DmDescription, Error},
    mao_event::{
        card_event::CardEvent,
        mao_event_result::{
            CallbackFunction, Disallow, MaoEventResult, MaoEventResultType, WrongPlayerInteraction,
        },
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

pub fn log<T>(msg: T) -> anyhow::Result<()>
where
    T: AsRef<str>,
{
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("log.log")?;
    file.write_all(msg.as_ref().as_bytes())?;
    file.write_all(&[10])?;
    Ok(())
}

#[derive(Debug, Clone)]
pub enum PlayerTurnUpdater {
    Set(usize),
    Update(isize),
}

impl FromStr for PlayerTurnUpdater {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split('_').collect();
        match splitted.len() {
            2 => match *splitted.first().unwrap() {
                "set" => Ok(Self::Set(splitted.last().unwrap().parse()?)),
                "up" => Ok(Self::Update(splitted.last().unwrap().parse()?)),
                _ => Err(anyhow::anyhow!(
                    "Invalid identifier (1) for PlayerTurnUpdater"
                )),
            },
            _ => Err(anyhow::anyhow!(
                "Invalid identifier when parsing PlayerTurnUpdater"
            )),
        }
    }
}

impl Default for PlayerTurnUpdater {
    fn default() -> Self {
        Self::Update(1)
    }
}

#[derive(Debug, Clone)]
pub enum PlayerTurnChange {
    Update(PlayerTurnUpdater),
    Rotate(PlayerTurnUpdater),
}
struct PlayerTurnChangeVisitor;

impl<'dee> serde::de::Visitor<'dee> for PlayerTurnChangeVisitor {
    type Value = PlayerTurnChange;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "internal error when parsing PlayerTurnChange")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse::<PlayerTurnChange>().map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for PlayerTurnChange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(PlayerTurnChangeVisitor)
    }
}

impl FromStr for PlayerTurnChange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split('_').collect();
        match splitted.len() {
            3 => {
                let updater =
                    (splitted[1].to_string() + "_" + splitted[2]).parse::<PlayerTurnUpdater>()?;
                match *splitted.first().unwrap() {
                    "up" => Ok(Self::Update(updater)),
                    "ro" => Ok(Self::Rotate(updater)),
                    _ => Err(anyhow::anyhow!(
                        "Invalid identifier when parsing first item"
                    )),
                }
            }
            _ => Err(anyhow::anyhow!("Invalid parsing for PlayerTurnChange")),
        }
    }
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
    previous_player_turn: Option<usize>,
    /// the direction of the turn (-1 OR 1)
    turn: isize,
    /// all the events of a player that it did during its turn
    player_events: Vec<MaoEvent>,
    can_play_on_new_stack: bool,
    automaton: Automaton,
    dealer: usize,
    config: Config,
    possible_actions: Vec<String>,
}

// getters and setters
impl MaoCore {
    pub fn activated_rules_indexes(&self) -> &[usize] {
        &self.activated_rules
    }
    pub fn automaton(&self) -> &Automaton {
        &self.automaton
    }
    pub fn automaton_mut(&mut self) -> &mut Automaton {
        &mut self.automaton
    }
    pub fn available_rules(&self) -> &[Rule] {
        &self.available_rules
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
    fn correct_player_action<I>(&self, expected: &[PlayerAction], datas: I) -> bool
    where
        I: IntoIterator,
        I::Item: Into<PlayerAction>,
    {
        let datas: Vec<PlayerAction> = datas.into_iter().map(|v| v.into()).collect();
        if expected.len() != datas.len() {
            return false;
        }
        if expected.iter().zip(datas).any(|(ver, data)| ver != &data) {
            return false;
        }
        true
    }
    pub fn dealer(&self) -> usize {
        self.dealer
    }

    pub fn on_say_action(&mut self, player_index: usize, message: String) -> anyhow::Result<()> {
        let event = MaoEvent::SayEvent {
            message,
            player_index,
        };
        let res = self.on_event(&event)?;
        let res = self.propagate_on_event_results_and_execute(player_index, &event, &res)?;
        for int in &res {
            match int {
                WrongPlayerInteraction::Disallow(d) => {
                    if let Some(pena) = d.penality {
                        pena(self, player_index)?;
                    } else {
                        self.on_penality(player_index)?;
                    }
                }
                WrongPlayerInteraction::ForgotSomething(_) => {
                    return Err(Error::OnMaoInteraction(String::from(
                        "Expected only Disallow found ForGotSomething from on_say_action",
                    ))
                    .into())
                }
            }
        }
        Ok(())
    }

    fn on_action_interaction(
        &mut self,
        player_index: usize,
        interactions: &[MaoInteraction],
    ) -> anyhow::Result<Vec<WrongPlayerInteraction>> {
        let required = &[PlayerAction::SelectPlayer, PlayerAction::DoAction];
        if !self.correct_player_action(required, interactions) {
            return Ok(vec![]);
        }

        let MaoInteraction { data, .. } = interactions.last().unwrap();
        let event = MaoEvent::PhysicalEvent {
            physical_name: data
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("physical cannot be None"))?
                .string_expecting()?
                .to_owned(),
            player_index,
        };
        let res = self.on_event(&event)?;
        let res = self.propagate_on_event_results_and_execute(player_index, &event, &res)?;

        for wrong_int in &res {
            match wrong_int {
                WrongPlayerInteraction::Disallow(d) => {
                    if let Some(penality) = d.penality {
                        penality(self, player_index)?
                    } else {
                        self.on_penality(player_index)?
                    }
                }
                WrongPlayerInteraction::ForgotSomething(_) => {
                    return Err(Error::OnMaoInteraction(
                        "While handling external rules return of action, ForgotSomething returned"
                            .to_string(),
                    )
                    .into())
                }
            }
        }
        // THERE
        Ok(res)
    }

    fn draw_interaction(
        player_index: usize,
        mao: &mut MaoCore,
        interactions: &[MaoInteraction],
    ) -> anyhow::Result<Vec<WrongPlayerInteraction>> {
        let required = vec![PlayerAction::SelectDrawableStack];
        if !mao.correct_player_action(&required, interactions) {
            // TODO
            return Ok(Vec::new());
        }

        let stack_index = match interactions[0].data.as_ref() {
            Some(is) => Some(is.index_expecting()?),
            None => None,
        };

        Ok(mao.on_draw_card(CardEvent {
            played_card: Card::default(),
            card_index: 0,
            player_index,
            stack_index,
        })?)
    }

    fn generate_actions() -> Vec<Vec<NodeState>> {
        vec![
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
            vec![
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectPlayer),
                    None,
                    None,
                ),
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::DoAction),
                    // TODO HERE
                    Some(|player_index, mao, datas| {
                        MaoCore::on_action_interaction(mao, player_index, datas)
                    }),
                    None,
                ),
            ],
        ]
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
    pub fn from_config(config: &mut Config) -> Result<Self, Error> {
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

        let mut s = Self::new(
            libraries,
            Self::init_stacks(),
            Vec::new(),
            Automaton::from_iter(Self::generate_actions()),
        );
        s.config = config.to_owned();
        s.possible_actions = config.get_all_physical_actions().into_iter().collect();
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

    pub fn init_stacks() -> Vec<Stack> {
        let mut stacks = vec![Stack::new(
            Self::generate_common_draw(),
            false,
            vec![StackType::Drawable],
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
            config: Config::default(),
            previous_player_turn: None,
            possible_actions: Vec::new(),
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

    fn on_play_card(
        &mut self,
        card_event: CardEvent,
    ) -> Result<Vec<WrongPlayerInteraction>, Error> {
        // let turn_ends_wront_int = self.on_turn_ends()?;
        let event = MaoEvent::PlayedCardEvent(card_event.to_owned());

        // calling rules
        let res = self.on_event(&event)?;

        let mut wrong_int =
            self.propagate_on_event_results_and_execute(card_event.player_index, &event, &res)?;
        if !wrong_int.is_empty() {
            for int in &wrong_int {
                match int {
                    WrongPlayerInteraction::Disallow(disallow) => {
                        if let Some(func) = disallow.penality {
                            func(self, card_event.player_index)?;
                        } else {
                            self.on_penality(card_event.player_index)?;
                        }
                    }
                    // this should never occur
                    // TODO REMOVE
                    WrongPlayerInteraction::ForgotSomething(f) => {
                        if let Some(func) = f.penality {
                            // TODO it is not always the player hand
                            func(self, card_event.player_index)?;
                        } else {
                            self.on_penality(card_event.player_index)?;
                        }
                    }
                }
            }
            wrong_int.extend(self.on_turn_ends(true)?);
            // wrong_int.extend(turn_ends_wront_int);
            return Ok(wrong_int);
        }
        // no interactions from external rules
        // check from official rules
        let player_turn_res = self.can_play(
            card_event.player_index,
            &card_event.played_card,
            card_event.stack_index.and_then(|i| self.stacks().get(i)),
        );
        // cannot play disallowed
        if !matches!(player_turn_res, PlayerTurnResult::CanPlay) {
            let mut res_wrong_int = self.on_turn_ends(true)?;
            self.on_penality(card_event.player_index)?;
            self.next_player(card_event.player_index, &event, true)?;
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
            res_wrong_int.push(WrongPlayerInteraction::Disallow(Disallow::new(
                "Basic Rules".to_string(),
                msg,
                None,
            )));
            return Ok(res_wrong_int);
        } else {
            // player can play
            // push card into played stack
            let res_wront_int = self.on_turn_ends(false)?;
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
            self.next_player(card_event.player_index, &event, false)?;
            return Ok(res_wront_int);
        }
    }

    /// Finish the turn of player
    /// This function should be only called from an action which has the ability to change to turn
    /// And this function should be called BEFORE the player turn change
    ///
    /// this function will call [`Self::on_event`] with [`MaoEvent::EndPlayerTurn`]
    /// and then clear player's actions
    /// `wrong_interaction is required` to see if the previous player who played has
    /// well played in case of a false, the move should not have been accorded by the game
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    fn on_turn_ends(
        &mut self,
        wrong_interaction: bool,
    ) -> Result<Vec<WrongPlayerInteraction>, Error> {
        // maybe add some check before like checking if the last inserted is a changeable turn
        // maybe add this wront interaction ?
        // punched has been triggered test other ones
        let last_index = self.player_events.len().saturating_sub(1);
        let _last_wrong_int = match wrong_interaction {
            true => self.player_events.pop(),
            false => None,
        };
        let mut range: Option<Range<usize>> = None;
        let mut indexes: Vec<(usize, bool)> = Vec::new();
        for i in (0..last_index).rev() {
            let event = self.player_events.get(i).unwrap();
            if event.can_change_turn() {
                range = Some(Range {
                    start: 0,
                    end: i + 1,
                });
            } else {
                let event_res = match event {
                    MaoEvent::PlayedCardEvent(card_event)
                    | MaoEvent::DiscardCardEvent(card_event)
                    | MaoEvent::DrawedCardEvent(card_event) => Some(card_event.player_index),
                    MaoEvent::SayEvent { player_index, .. }
                    | MaoEvent::PhysicalEvent { player_index, .. } => Some(*player_index),
                    MaoEvent::GiveCardEvent { .. } => None,
                    _ => None,
                };

                // The current action of this player turn can be either related to the turn before him or its own turn
                if event_res.is_none() || event_res.is_some_and(|v| v != self.player_turn) {
                    indexes.push((i, true));
                } else {
                    indexes.push((i, false));
                }
            }
        }
        // events are in reversed order (newer...last_ones)
        let mut datas: Vec<MaoEvent> = Vec::new();
        for (index, remove) in indexes.drain(..) {
            if remove {
                datas.push(self.player_events.remove(index));
            } else {
                datas.push(self.player_events.get(index).unwrap().to_owned());
            }
        }
        if let Some(range) = range {
            datas.extend(self.player_events.drain(range).rev());
        }
        if datas.is_empty() {
            return Ok(Vec::new());
        }
        let event = MaoEvent::EndPlayerTurn {
            // events: self.player_events.extract_if(|_| false).collect(),
            events: datas,
        };
        let res = self.on_event(&event)?;
        let wrong_int =
            self.propagate_on_event_results_and_execute(self.player_turn, &event, &res)?;
        if !wrong_int.is_empty() {
            for int in &wrong_int {
                match int {
                    WrongPlayerInteraction::Disallow(disallow) => {
                        if let Some(func) = disallow.penality {
                            func(self, self.player_turn)?;
                        } else {
                            self.on_penality(self.player_turn)?;
                        }
                    }
                    WrongPlayerInteraction::ForgotSomething(f) => {
                        if let Some(func) = f.penality {
                            func(self, self.player_turn)?;
                        } else {
                            self.on_penality(self.player_turn)?;
                        }
                    }
                }
            }
            return Ok(wrong_int);
        }
        let player_pseudo = self
            .previous_player_turn
            .and_then(|index| self.players.get(index))
            .map(|player| player.get_pseudo());
        if player_pseudo.is_none() {
            return Ok(vec![]);
        }
        let player_pseudo = player_pseudo.unwrap();

        let mut wrong_int: Vec<WrongPlayerInteraction> = Vec::new();

        match event {
            MaoEvent::EndPlayerTurn { events } => {
                let say_events: Vec<(&str, usize)> = events
                    .iter()
                    .flat_map(|event| match event {
                        MaoEvent::SayEvent {
                            message,
                            player_index,
                        } => Some((message.as_str(), *player_index)),
                        _ => None,
                    })
                    .collect();
                // check in all events played card type
                for previous_event in events.iter() {
                    if let MaoEvent::PlayedCardEvent(card_event) = previous_event {
                        // get all card's effects
                        let card_effects = self.get_card_effects(&card_event.played_card);
                        // check if all card's effects are been done
                        for effect in card_effects.iter() {
                            if let SingleCardEffect::CardPlayerAction(card_action) = effect {
                                match card_action {
                                    CardPlayerAction::Say(words_to_say) => {
                                        for word_to_say in words_to_say {
                                            match word_to_say {
                                                SingOrMult::Single(word) => {
                                                    // check if the required word as been said earlier
                                                    if !say_events.iter().any(
                                                        |(message, player_index)| {
                                                            *player_index == card_event.player_index
                                                                && message.contains(word)
                                                        },
                                                    ) {
                                                        wrong_int.push(
                                                            WrongPlayerInteraction::forgot_saying_basic(None, player_pseudo));
                                                        break;
                                                    }
                                                }
                                                // check if all the words have been said earlier
                                                SingOrMult::Multiple(words) => {
                                                    if !say_events.iter().any(
                                                        |(message, player_index)| {
                                                            *player_index == card_event.player_index
                                                                && words.iter().any(|word| {
                                                                    message.contains(word)
                                                                })
                                                        },
                                                    ) {
                                                        wrong_int.push(
                                                            WrongPlayerInteraction::forgot_saying_basic(None, player_pseudo));
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    CardPlayerAction::Physical(physical_name) => {
                                        if !events.contains(&MaoEvent::PhysicalEvent {
                                            physical_name: physical_name.to_owned(),
                                            player_index: card_event.player_index,
                                        }) {
                                            wrong_int.push(
                                                WrongPlayerInteraction::forgot_doing_basic(
                                                    None,
                                                    player_pseudo,
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => unreachable!(),
        }

        for int in wrong_int.iter() {
            match int {
                WrongPlayerInteraction::ForgotSomething(f) => {
                    if let Some(func) = f.penality {
                        func(self, self.previous_player_turn.unwrap())?;
                    } else {
                        self.on_penality(self.previous_player_turn.unwrap())?;
                    }
                }
                // TODO change this return type
                _ => unreachable!(),
            }
        }
        Ok(wrong_int)
    }

    fn play_interaction(
        player_index: usize,
        mao: &mut MaoCore,
        interactions: &[MaoInteraction],
    ) -> anyhow::Result<Vec<WrongPlayerInteraction>> {
        let expected = vec![PlayerAction::SelectCard, PlayerAction::SelectPlayableStack];
        if !mao.correct_player_action(&expected, interactions) {
            // TODO
            return Err(Error::InvalidMaoInteraction {
                expected,
                received: interactions.iter().map(|v| v.action.to_owned()).collect(),
            }
            .into());
        }
        let card_index = interactions[0]
            .data
            .as_ref()
            .ok_or_else(|| Error::InvalidCardIndex {
                card_index: usize::MAX,
                len: 0,
            })?
            .index_expecting()?;

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
            .get(card_index)
            .ok_or(Error::InvalidCardIndex {
                card_index,
                len: player.get_cards().len(),
            })
            .unwrap()
            .to_owned(); // TODO

        let stack_index = match interactions[1].data.as_ref() {
            Some(is) => Some(is.index_expecting()?),
            None => None,
        };
        if stack_index.is_some_and(|index| mao.stacks.get(index).is_none()) {
            return Err(Error::InvalidStackIndex {
                stack_index: stack_index.unwrap(),
                len: mao.stacks.len(),
            }
            .into()); // TODO
        }
        Ok(mao.on_play_card(CardEvent {
            card_index,
            played_card: card,
            player_index,
            stack_index,
        })?)
        // mao.playsc
    }

    pub fn player_turn(&self) -> usize {
        self.player_turn
    }

    pub fn player_won(&self) -> Option<(usize, &Player)> {
        self.players
            .iter()
            .enumerate()
            .find(|(_, player)| player.get_cards().is_empty())
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn players_events(&self) -> &[MaoEvent] {
        &self.player_events
    }

    pub fn players_mut(&mut self) -> &mut Vec<Player> {
        &mut self.players
    }

    pub fn possible_actions(&self) -> &[String] {
        &self.possible_actions
    }

    pub fn set_can_play_on_new_stack(&mut self, can_play_on_new_stack: bool) {
        self.can_play_on_new_stack = can_play_on_new_stack;
    }

    pub fn set_dealer(&mut self, dealer: usize) {
        self.dealer = dealer;
    }

    pub fn stacks(&self) -> &[Stack] {
        &self.stacks
    }

    pub fn stacks_mut(&mut self) -> &mut Vec<Stack> {
        &mut self.stacks
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
                .any(|(_, stack)| !stack.get_cards().is_empty())
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
            .cloned()
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
        self.players
            .last_mut()
            .unwrap()
            .get_cards_mut()
            .push(Card::new(
                CardValue::Number(9),
                CardType::Common(CommonCardType::Spade),
                None,
            ));
        Ok(())
    }

    pub fn init_all_players(&mut self, nb_card: usize) -> Result<(), Error> {
        for i in 0..self.players.len() {
            let cards: Vec<Card> = self.draw_multiple_cards_unchosen(nb_card)?.to_vec();
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

    fn get_card_effect(&self, key: CardEffectsKey) -> Vec<&SingleCardEffect> {
        if let Some(v) = self.config.cards_effects.get(&key) {
            match v {
                SingOrMult::Single(s) => return vec![s],
                SingOrMult::Multiple(v) => return v.iter().collect(),
            }
        }
        vec![]
    }

    /// Returns all the [`CardEffects`] that a [`Card`] has on
    fn get_card_effects(&self, card: &Card) -> Vec<&SingleCardEffect> {
        let mut effects = vec![];
        // Searching effects with only its value
        effects.extend(
            self.get_card_effect(CardEffectsKey::new(None, Some(card.get_value().to_owned()))),
        );
        // Searching effects with only its type
        effects.extend(
            self.get_card_effect(CardEffectsKey::new(Some(card.get_sign().to_owned()), None)),
        );
        // Searching effects with the card itself
        effects.extend(self.get_card_effect(CardEffectsKey::new(
            Some(card.get_sign().to_owned()),
            Some(card.get_value().to_owned()),
        )));

        effects
    }

    /// Handle turn change when a [`MaoEvent`] occurs, player index is used to check if it is the player turn
    pub fn next_player(
        &mut self,
        player_index: usize,
        event: &MaoEvent,
        took_penality: bool,
    ) -> Result<(), Error> {
        match event {
            MaoEvent::PlayedCardEvent(card_event) => {
                // no need to remove / add cards handled before
                // TODO try to call rules if some want to change the turn
                if player_index == self.player_turn {
                    if took_penality {
                        self.update_turn(PlayerTurnChange::default());
                        return Ok(());
                    }
                    self.previous_player_turn = Some(self.player_turn);
                    let changes: Vec<&PlayerTurnChange> = self
                        .get_card_effects(&card_event.played_card)
                        .iter()
                        .filter_map(|card_effect| match card_effect {
                            SingleCardEffect::PlayerTurnChange(change) => Some(change),
                            SingleCardEffect::CardPlayerAction(_) => None,
                        })
                        .collect();
                    if changes.is_empty() {
                        self.update_turn(PlayerTurnChange::default());
                    } else {
                        let changes: Vec<PlayerTurnChange> =
                            changes.iter().map(|&v| v.to_owned()).collect();
                        for change in changes {
                            self.update_turn(change);
                        }
                    }
                    return Ok(());
                }
            }
            MaoEvent::DiscardCardEvent(_) => todo!(),
            MaoEvent::DrawedCardEvent(_) => {
                if player_index == self.player_turn {
                    self.update_turn(PlayerTurnChange::default());
                    self.previous_player_turn = Some(self.player_turn);
                    return Ok(());
                }
            }
            MaoEvent::GiveCardEvent { .. } => (),
            MaoEvent::StackPropertyRunsOut { .. } => (),
            MaoEvent::GameStart => (),
            MaoEvent::EndPlayerTurn { .. } => (),
            MaoEvent::VerifyEvent => unreachable!("verify event"),
            MaoEvent::PlayerPenality { .. } => (),
            MaoEvent::SayEvent { .. } => todo!(),
            MaoEvent::PhysicalEvent { .. } => todo!(),
        }
        Ok(())
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
        if event.is_recordable() {
            self.player_events.push(event.to_owned());
        }
        let mut results = Vec::with_capacity(self.activated_rules.len());
        for i in 0..self.activated_rules.len() {
            results.push(self
                .available_rules
                .get(self.activated_rules[i])
                .unwrap()
                .get_on_event_func()(event, self)?);
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
    pub fn propagate_on_event_results_and_execute<'a, I>(
        &mut self,
        player_index: usize,
        previous_event: &MaoEvent,
        event_results_iter: I,
    ) -> Result<Vec<WrongPlayerInteraction>, Error>
    where
        I: IntoIterator<Item = &'a MaoEventResult>,
    {
        let event_results: Vec<&MaoEventResult> =
            event_results_iter.into_iter().map(|v| v).collect();
        if event_results
            .iter()
            .any(|r| !matches!(r.res_type, MaoEventResultType::Ignored,))
        {
            let mut wrong_interactions: Vec<WrongPlayerInteraction> = Vec::new();
            // one rule did not ignored it
            let mut not_ignored: Vec<&MaoEventResult> = Vec::new();
            for mao_res in &event_results {
                match &mao_res.res_type {
                    MaoEventResultType::Disallow(disallow) => {
                        wrong_interactions
                            .push(WrongPlayerInteraction::Disallow(disallow.to_owned()));
                    }
                    MaoEventResultType::ForgetSomething(s) => wrong_interactions
                        .push(WrongPlayerInteraction::ForgotSomething(s.to_owned())),
                    MaoEventResultType::Ignored => (),
                    MaoEventResultType::OverrideBasicRule(_)
                    | MaoEventResultType::ExecuteBeforeTurnChange(_)
                    | MaoEventResultType::ExecuteAfterTurnChange(_) => not_ignored.push(mao_res),
                }
            }

            let mut results_on_results = Vec::new();
            for res in event_results {
                if let Some(func) = res.other_rules_callback.as_ref() {
                    results_on_results.push(func(self, previous_event, &not_ignored)?);
                }
            }
            for res in &results_on_results {
                match res.res_type {
                    // cannot disallow the action of a rule
                    MaoEventResultType::Ignored
                    | MaoEventResultType::Disallow(_)
                    | MaoEventResultType::ForgetSomething(_) => (),
                    MaoEventResultType::OverrideBasicRule(_)
                    | MaoEventResultType::ExecuteBeforeTurnChange(_)
                    | MaoEventResultType::ExecuteAfterTurnChange(_) => not_ignored.push(res),
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
                            MaoEventResultType::ExecuteBeforeTurnChange(f) => bef.push(f),
                            MaoEventResultType::ExecuteAfterTurnChange(f) => aft.push(f),
                            _ => (),
                        }
                        (bef, aft)
                    },
                );

                for bef in &before {
                    bef(self, player_index)?;
                }
                if !overrided {
                    self.next_player(player_index, previous_event, false)?;
                }
                for aft in &after {
                    aft(self, player_index)?;
                }
            }
            return Ok(wrong_interactions);
        }
        Ok(Vec::new())
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
        self.get_stack_target(target_index)?.add_card(card);
        Ok(())
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
            if self
                .on_event(&event)?
                .iter()
                .any(|r| !matches!(r.res_type, MaoEventResultType::Ignored))
            {
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

    fn on_draw_card(
        &mut self,
        mut card_event: CardEvent,
    ) -> Result<Vec<WrongPlayerInteraction>, Error> {
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
        // call rules back propagate will be called later on
        let res = self.on_event(&event)?;

        let turn_ends_wrong_int = if card_event.player_index == self.player_turn {
            self.on_turn_ends(true)?
        } else {
            vec![]
        };

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
                    values.push(result);
                }
            }
            // TODO
            let mut disallows = self.propagate_on_event_results_and_execute(
                card_event.player_index,
                &event,
                values,
            )?;
            if !disallows.is_empty() {
                disallows.extend(turn_ends_wrong_int);
                return Ok(disallows);
            }
        }

        Ok(turn_ends_wrong_int)
    }
}

// players' actions
impl MaoCore {
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
