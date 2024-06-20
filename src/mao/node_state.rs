use indextree::{Arena, NodeId};

use crate::mao_event::mao_event_result::Disallow;

use super::{
    mao_action::MaoInteraction,
    mao_internal::{log, MaoInternal},
};

#[derive(Debug)]
pub enum MaoInteractionResult<'a> {
    NoInteractionFound,
    Leaf {
        interactions: Vec<MaoInteraction>,
        func: CallbackInteraction,
    },
    Nodes(Vec<&'a NodeState>),
    AdvancedNextState,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum PlayerAction {
    #[default]
    SelectCard,
    SelectPlayer,
    SelectPlayableStack,
    SelectDrawableStack,
}

impl std::fmt::Display for PlayerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PlayerAction::SelectCard => "card",
                PlayerAction::SelectPlayer => "player",
                PlayerAction::SelectPlayableStack => "playable stack",
                PlayerAction::SelectDrawableStack => "drawable stack",
            }
        )
    }
}

/// (mao, player_index, datas)
pub type CallbackInteraction =
    fn(usize, &mut MaoInternal, &[MaoInteraction]) -> anyhow::Result<Vec<Disallow>>;

#[derive(Debug, Clone, Default)]
pub struct NodeState {
    pub action: MaoInteraction,
    // information: String
    // data: String,
    pub func: Option<CallbackInteraction>,
}

impl NodeState {
    pub fn new(action: MaoInteraction, func: Option<CallbackInteraction>) -> Self {
        Self { action, func }
    }
}

#[derive(Debug)]
pub struct Automaton {
    arena: Arena<NodeState>,
    current_state: NodeId,
    root: NodeId,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.action.fmt(f)
    }
}

impl Automaton {
    /// Returns the NodeId according to PartialEq from `self.current_state` if it exists
    fn get_node(&self, action: PlayerAction) -> Option<NodeId> {
        self.current_state
            .children(&self.arena)
            .filter(|id| {
                let node_data = self.arena.get(*id).map(|node| node.get()).unwrap();
                action == node_data.action.action && node_data.func.is_none()
            })
            .next()
    }

    /// Returns the leaves' id (executable nodes) of this [`Automaton`]
    fn get_leaves(&self, action: PlayerAction) -> Vec<NodeId> {
        self.current_state
            .children(&self.arena)
            .filter(|id| {
                let node_data = self.arena.get(*id).map(|node| node.get()).unwrap();
                action == node_data.action.action && node_data.func.is_some()
            })
            .collect()
    }

    /// Search among the children of the current state if there is any node which matches to `action`
    fn search_type(&self, action: PlayerAction) -> Vec<NodeId> {
        self.current_state
            .children(&self.arena)
            .filter(|id| {
                self.arena
                    .get(*id)
                    .map(|node| node.get())
                    .unwrap()
                    .action
                    .action
                    == action
            })
            .collect()
    }

    /// Insert this iterator,
    /// the first one is the parent, and each next one is the child of the previous one
    fn insert_iter(&mut self, datas: &[NodeState]) {
        let mut parent = self.root;
        for data in datas {
            parent = match self.get_node(data.action.action.to_owned()) {
                Some(par) => par,
                None => parent.append_value(data.to_owned(), &mut self.arena),
            };
        }
    }

    /// Cancels the last action and returns the previous action if exists
    /// set `self.current_state` if not initial state
    pub fn cancel_last(&mut self) -> Option<&NodeState> {
        if let Some(parent) = self.current_state.ancestors(&mut self.arena).skip(1).next() {
            let save = self.current_state;
            self.current_state = parent;
            return Some(self.arena.get(save).unwrap().get());
        }
        None
    }

    /// Advances to the next `action` if presents then returns the next actions which match to `action`
    /// the returned actions can be the nodes (func = None)
    /// or the executable actions (action != None), in that case the automaton is reseted
    pub fn on_action(&mut self, interaction: MaoInteraction) -> MaoInteractionResult {
        let nodes: Vec<NodeId> = self.search_type(interaction.action.to_owned());
        match nodes.len() {
            0 => MaoInteractionResult::NoInteractionFound,
            1 => match self.arena.get(*nodes.first().unwrap()).unwrap().get().func {
                Some(_) => {
                    let mut interactions: Vec<MaoInteraction> = self
                        .get_executed_actions()
                        .iter()
                        .map(|node| node.action.to_owned())
                        .collect();
                    interactions.push(interaction);
                    self.reset();
                    return MaoInteractionResult::Leaf {
                        func: self
                            .arena
                            .get(*nodes.first().unwrap())
                            .unwrap()
                            .get()
                            .func
                            .unwrap(),
                        interactions,
                    };
                }
                None => {
                    self.current_state = *nodes.first().unwrap();
                    self.arena
                        .get_mut(self.current_state)
                        .unwrap()
                        .get_mut()
                        .action
                        .index = interaction.index;
                    MaoInteractionResult::AdvancedNextState
                }
            },
            _ => MaoInteractionResult::Nodes(
                nodes
                    .iter()
                    .map(|id| self.arena.get(*id).unwrap().get())
                    .collect(),
            ),
        }
    }

    pub fn reset(&mut self) {
        self.current_state = self.root;
    }

    // pub fn advance_next_action(&mut self, action: PlayerAction) -> Result<(), Error> {
    //     if let Some(node) = self.get_node(action) {
    //         self.current_state = node;
    //         Ok(())
    //     } else {
    //         Err(Error::StateNotFound)
    //     }
    // }

    /// Returns the current state, returning None if no action has been done yet
    ///
    /// # Panics
    ///
    /// Panics if the current state is absent, this sould never occur
    pub fn current_state(&self) -> Option<&NodeState> {
        if self
            .arena
            .get(self.current_state)
            .unwrap()
            .parent()
            .is_none()
        {
            return None;
        } else {
            Some(
                self.arena
                    .get(self.current_state)
                    .map(|node| node.get())
                    .unwrap(),
            )
        }
    }

    /// Returns the executed actions of this [`Automaton`], ordered by time
    ///
    /// # Panics
    ///
    /// Cannot panic
    pub fn get_executed_actions(&self) -> Vec<&NodeState> {
        let mut a: Vec<&NodeState> = self
            .current_state
            .ancestors(&self.arena)
            .map(|id| self.arena.get(id).unwrap().get())
            .collect();
        a.pop();
        a.reverse();
        a
    }
}

impl FromIterator<Vec<NodeState>> for Automaton {
    fn from_iter<T: IntoIterator<Item = Vec<NodeState>>>(iter: T) -> Self {
        let mut arena: Arena<NodeState> = Arena::new();
        let root = arena.new_node(NodeState {
            action: MaoInteraction::default(),
            func: None,
        });
        let mut ret = Self {
            arena,
            current_state: root,
            root,
        };
        for datas in iter.into_iter() {
            assert!(datas.last().is_some_and(|v| v.func.is_some()));
            assert!(datas[..datas.len().saturating_sub(1)]
                .iter()
                .all(|v| v.func.is_none()));
            ret.insert_iter(&datas);
        }
        ret
        // Self {
        //     arena: arena,
        //     current_state: root,
        // }
    }
}
