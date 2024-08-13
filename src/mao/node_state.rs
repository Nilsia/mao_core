use indextree::{Arena, NodeId};

use crate::{error::Error, mao_event::mao_event_result::Disallow, stack::stack_type::StackType};

use super::{mao_action::MaoInteraction, mao_internal::MaoInternal};

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
    SelectDiscardableStack,
}

impl From<StackType> for PlayerAction {
    fn from(value: StackType) -> Self {
        match value {
            StackType::Playable => PlayerAction::SelectPlayableStack,
            StackType::Drawable => PlayerAction::SelectDrawableStack,
            StackType::Discardable => PlayerAction::SelectDiscardableStack,
        }
    }
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
                PlayerAction::SelectDiscardableStack => "discardable stack",
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
    pub rule: Option<String>,
    pub func: Option<CallbackInteraction>,
}

impl NodeState {
    pub fn new(
        action: MaoInteraction,
        func: Option<CallbackInteraction>,
        rule: Option<String>,
    ) -> Self {
        Self { action, func, rule }
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
    /// Only a node with no function is returned
    fn get_node(&self, action: PlayerAction) -> Option<NodeId> {
        self.get_node_of(self.current_state, action)
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

    pub fn on_action_indexed(
        &mut self,
        interaction: MaoInteraction,
        index: usize,
    ) -> Result<MaoInteractionResult, Error> {
        match self.on_action(interaction.to_owned()) {
            MaoInteractionResult::Nodes(nodes) => {
                match nodes.get(index) {
                    Some(&node) => match node.func {
                        Some(func) => {
                            let mut interactions: Vec<MaoInteraction> = self
                                .get_executed_actions()
                                .iter()
                                .map(|&v| v.action.to_owned())
                                .collect();
                            interactions.push(interaction);
                            return Ok(MaoInteractionResult::Leaf { interactions, func });
                        }
                        // is a node so advance in it
                        None => {
                            self.current_state =
                                self.get_node(interaction.action.to_owned()).unwrap();
                            self.arena
                                .get_mut(self.current_state)
                                .unwrap()
                                .get_mut()
                                .action
                                .index = interaction.index;
                            return Ok(MaoInteractionResult::AdvancedNextState);
                        }
                    },
                    // index is invalid
                    None => Err(Error::OnMaoInteraction(format!(
                        "Invalid index when retrieving action : {} out of {}",
                        index,
                        nodes.len()
                    ))),
                }
            }
            // not nodes
            _ => Err(Error::OnMaoInteraction(format!(
                "Provided index does not lead to multiple nodes"
            ))),
        }
    }

    fn put_node_at_end(nodes: &mut Vec<&NodeState>) {
        if let Some(index) = nodes
            .iter()
            .enumerate()
            .filter(|(_, v)| v.func.is_none())
            .next()
            .map(|v| v.0)
        {
            let len = nodes.len();
            nodes.swap(index, len.saturating_sub(1));
        }
    }

    /// Tries to advance to the next action, according to `interaction`
    /// if there are multiple actions they are all returned
    /// In case of returned leaves and uniq node, the node will always be at the end of the [`Vec`]
    /// If there is only a node or a leaf, these one are returned
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
            _ => {
                let mut nodes: Vec<&NodeState> = self.nodes_from_ids(&nodes);
                Self::put_node_at_end(&mut nodes);
                MaoInteractionResult::Nodes(nodes)
            }
        }
    }

    fn nodes_from_ids(&self, nodes: &[NodeId]) -> Vec<&NodeState> {
        nodes
            .iter()
            .map(|id| self.arena.get(*id).unwrap().get())
            .collect()
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

    fn get_node_of(&self, node_id: NodeId, action: PlayerAction) -> Option<NodeId> {
        node_id
            .children(&self.arena)
            .filter(|id| {
                let node_data = self.arena.get(*id).map(|node| node.get()).unwrap();
                action == node_data.action.action && node_data.func.is_none()
            })
            .next()
    }

    pub fn path_exists(&self, path: &[PlayerAction]) -> bool {
        let mut current = self.current_state;
        for action in &path[..path.len().saturating_sub(1)] {
            if let Some(node_id) = self.get_node_of(current, action.to_owned()) {
                current = node_id;
            } else {
                return false;
            }
        }
        return true;
    }
}

impl FromIterator<Vec<NodeState>> for Automaton {
    fn from_iter<T: IntoIterator<Item = Vec<NodeState>>>(iter: T) -> Self {
        let mut arena: Arena<NodeState> = Arena::new();
        let root = arena.new_node(NodeState::new(MaoInteraction::default(), None, None));
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
