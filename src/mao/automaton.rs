use std::sync::Arc;

use indextree::{Arena, NodeId};

use crate::{
    error::Error, mao_event::mao_event_result::PlayerInteractionResult,
    stack::stack_type::StackType,
};

use super::{mao_action::MaoInteraction, mao_core::MaoCore};

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
    SelectRule,
    DoAction,
}

impl From<&MaoInteraction> for PlayerAction {
    fn from(value: &MaoInteraction) -> Self {
        value.action.to_owned()
    }
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
                PlayerAction::SelectRule => "rule",
                PlayerAction::DoAction => "do action",
            }
        )
    }
}

/// (mao, player_index, datas)
pub type CallbackInteraction =
    fn(usize, &mut MaoCore, &[MaoInteraction]) -> anyhow::Result<Vec<PlayerInteractionResult>>;

#[derive(Debug, Clone, Default)]
pub struct NodeState {
    pub action: MaoInteraction,
    pub rule: Option<Arc<str>>,
    pub func: Option<CallbackInteraction>,
}

impl Ord for NodeState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.action, &self.rule).cmp(&(&other.action, &other.rule))
    }
}

impl PartialOrd for NodeState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NodeState {
    fn eq(&self, other: &Self) -> bool {
        self.action == other.action && self.rule == other.rule
    }
}

impl Eq for NodeState {}

impl NodeState {
    pub fn new(
        action: MaoInteraction,
        func: Option<CallbackInteraction>,
        rule: Option<String>,
    ) -> Self {
        Self {
            action,
            func,
            rule: rule.map(|v| v.into()),
        }
    }
}

#[derive(Clone)]
pub struct Automaton {
    arena: Arena<NodeState>,
    current_state: NodeId,
    root: NodeId,
    previous_interactions: Vec<MaoInteraction>,
}

impl std::fmt::Debug for Automaton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "current state id : {}", self.current_state)?;
        write!(f, "{}", self.current_state.debug_pretty_print(&self.arena))
    }
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.action,
            self.rule
                .as_ref()
                .map_or(String::new(), |r| format!("({})", r)),
            self.func.map_or(String::new(), |f| format!("({f:?})"))
        )
    }
}

impl Automaton {
    /// Cancels the last action and returns the previous action if exists
    /// set `self.current_state` if not initial state
    pub fn cancel_last(&mut self) -> Option<&NodeState> {
        if let Some(parent) = self.current_state.ancestors(&self.arena).nth(1) {
            let save = self.current_state;
            self.current_state = parent;
            return Some(self.arena.get(save).unwrap().get());
        }
        None
    }
    // fn get_all_leaves(&self) -> Vec<NodeId> {

    fn children_of(&self, node_id: NodeId) -> Vec<NodeId> {
        node_id
            .children(&self.arena)
            .filter(|v| !v.is_removed(&self.arena))
            .collect()
    }

    /// Removes the field `rule` of [`NodeState`] of all the nodes
    fn clear_path<T>(mut path: T)
    where
        T: AsMut<Vec<NodeState>>,
    {
        if path.as_mut().is_empty() {
            return;
        }
        for node_state in path.as_mut().split_last_mut().unwrap().1.iter_mut() {
            node_state.rule = None;
        }
    }

    fn convert_into_node_ids<'a>(
        &self,
        path: impl IntoIterator<Item = &'a NodeState>,
    ) -> Option<Vec<NodeId>> {
        let mut data = vec![self.root];
        for node_state in path.into_iter() {
            match self.get_node_id_from_children(data.last().unwrap().to_owned(), node_state) {
                Some(node_id) => data.push(node_id),
                None => {
                    return None;
                }
            }
        }
        data.remove(0);
        Some(data)
    }

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
            None
        } else {
            Some(
                self.arena
                    .get(self.current_state)
                    .map(|node| node.get())
                    .unwrap(),
            )
        }
    }

    /// Returns the leaves' id (executable nodes) of this [`Automaton`] according to `self.current_state`
    fn get_currrent_leaves(&self, action: PlayerAction) -> Vec<NodeId> {
        self.children_of(self.current_state)
            .iter()
            .filter(|id| {
                let node_data = self.arena.get(**id).map(|node| node.get()).unwrap();
                action == node_data.action.action && node_data.func.is_some()
            })
            .cloned()
            .collect()
    }

    /// Returns the executed actions of this [`Automaton`], ordered by time
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

    pub fn get_executed_mao_interactions(&self) -> Vec<&MaoInteraction> {
        self.get_executed_actions()
            .iter()
            .map(|v| &v.action)
            .collect::<Vec<&MaoInteraction>>()
    }

    fn get_leaves(&self, node_id: NodeId) -> Vec<NodeId> {
        self.children_of(node_id)
            .iter()
            .filter(|&&id| {
                let node = self.arena.get(id).unwrap().get();
                node.func.is_some()
            })
            .cloned()
            .collect()
    }

    // }
    /// Returns the NodeId according to PartialEq from `self.current_state` if it exists
    /// Only a node with no function is returned
    fn get_node_from_current(&self, action: PlayerAction) -> Option<NodeId> {
        self.get_node_id_of(self.current_state, action)
    }

    fn get_node_id_from_children(&self, node_id: NodeId, node_state: &NodeState) -> Option<NodeId> {
        self.children_of(node_id)
            .iter()
            .find(|&&id| {
                let node = self.arena.get(id).unwrap();
                !node.is_removed() && node_state == node.get()
            })
            .cloned()
    }

    fn get_node_id_of(&self, node_id: NodeId, action: PlayerAction) -> Option<NodeId> {
        self.children_of(node_id)
            .iter()
            .find(|&id| {
                let node_data = self.arena.get(*id).map(|node| node.get()).unwrap();
                action == node_data.action.action && node_data.func.is_none()
            })
            .cloned()
    }

    pub fn previous_interactions(&self) -> &[MaoInteraction] {
        &self.previous_interactions
    }

    pub fn previous_interactions_mut(&mut self) -> &mut Vec<MaoInteraction> {
        &mut self.previous_interactions
    }

    /// Insert this iterator,
    /// the first one is the parent, and each next one is the child of the previous one
    fn insert_iter(&mut self, datas: &[NodeState]) {
        if datas.is_empty() {
            return;
        }
        let mut parent = self.root;
        // create only nodes which do not holds function
        for data in &datas[..datas.len().saturating_sub(1)] {
            parent = match self.get_node_id_of(parent, data.action.action.to_owned()) {
                Some(par) => par,
                None => parent.append_value(
                    NodeState::new(data.action.to_owned(), data.func, None),
                    &mut self.arena,
                ),
            };
        }
        // append function/leaf in order to check if the leaf is different the all the other ones
        let last = datas.last().unwrap();
        let leaves: Vec<&NodeState> = self
            .get_leaves(parent)
            .iter()
            .map(|&id| self.arena.get(id).unwrap().get())
            .collect();
        if leaves.contains(&last) {
            panic!("Cannot add node {last:?} because it is already present inside Self");
        } else {
            parent.append_value(last.to_owned(), &mut self.arena);
        }
    }

    fn nodes_from_ids(&self, nodes: &[NodeId]) -> Vec<&NodeState> {
        nodes
            .iter()
            .map(|id| self.arena.get(*id).unwrap().get())
            .collect()
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
                    // TODO remove this reset
                    self.reset();
                    // TODO avoid copying
                    self.previous_interactions = interactions.clone();
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
                        .data = interaction.data;
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
                            self.reset();
                            // TODO avoid cloning
                            self.previous_interactions = interactions.to_owned();
                            Ok(MaoInteractionResult::Leaf { interactions, func })
                        }
                        // is a node so advance in it
                        None => {
                            self.current_state = self
                                .get_node_id_of(self.current_state, interaction.action.to_owned())
                                .unwrap();
                            self.arena
                                .get_mut(self.current_state)
                                .unwrap()
                                .get_mut()
                                .action
                                .data = interaction.data;
                            Ok(MaoInteractionResult::AdvancedNextState)
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
            _ => Err(Error::OnMaoInteraction(
                "Provided index does not lead to multiple nodes".to_owned(),
            )),
        }
    }

    pub fn path_exists(&self, path: &[PlayerAction]) -> bool {
        let mut current = self.current_state;
        for action in &path[..path.len().saturating_sub(1)] {
            if let Some(node_id) = self.get_node_id_of(current, action.to_owned()) {
                current = node_id;
            } else {
                return false;
            }
        }
        true
    }

    fn put_node_at_end(nodes: &mut [&NodeState]) {
        if let Some(index) = nodes
            .iter()
            .enumerate()
            .find_map(|(i, v)| v.func.is_none().then_some(i))
        {
            let len = nodes.len();
            nodes.swap(index, len.saturating_sub(1));
        }
    }

    pub fn remove_paths<T>(&mut self, paths: T)
    where
        T: IntoIterator,
        T::Item: AsMut<Vec<NodeState>> + AsRef<Vec<NodeState>>,
    {
        for mut path in paths.into_iter() {
            if path.as_ref().is_empty() {
                continue;
            }
            Self::clear_path(&mut path);
            Self::verify_action_path(path.as_ref());
            let node_ids = self.convert_into_node_ids(path.as_ref());

            if let Some(node_ids) = node_ids {
                for node_id in node_ids.iter().rev() {
                    if self.children_of(*node_id).is_empty() {
                        node_id.remove(&mut self.arena);
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.current_state = self.root;
        self.previous_interactions.clear();
    }

    fn same_node(&self, self_node_id: NodeId, (other, node_id): (&Self, NodeId)) -> bool {
        let (self_actions, other_actions): (Vec<NodeId>, Vec<NodeId>) = (
            self.children_of(self_node_id).into_iter().collect(),
            other.children_of(node_id).into_iter().collect(),
        );
        // assert_eq!(self_actions.len(), other_actions.len());
        if self_actions.len() != other_actions.len() {
            return false;
        }
        let mut self_actions: Vec<(NodeId, &NodeState)> = self_actions
            .iter()
            .map(|s| (*s, self.arena.get(*s).unwrap().get()))
            .collect();
        let mut other_actions: Vec<(NodeId, &NodeState)> = other_actions
            .iter()
            .map(|o| (*o, other.arena.get(*o).unwrap().get()))
            .collect();
        self_actions.sort_by(|(_, node1), (_, node2)| node1.cmp(node2));
        other_actions.sort_by(|(_, node1), (_, node2)| node1.cmp(node2));

        for ((s_id, s_node), (o_id, o_node)) in self_actions.iter().zip(&other_actions) {
            // assert_eq!(s_node, o_node);
            if s_node.func.is_some() != o_node.func.is_some() {
                return false;
            }
            if s_node.func.is_some() {
                // there are leaves (exectable nodes)
                if s_node != o_node {
                    return false;
                }
            } else {
                // there only nodes
                if s_node.action != o_node.action {
                    return false;
                }
            }
            if !self.same_node(*s_id, (other, *o_id)) {
                return false;
            }
        }
        true
    }

    /// Search among the children of the current state if there is any node which matches to `action`
    fn search_type(&self, action: PlayerAction) -> Vec<NodeId> {
        self.children_of(self.current_state)
            .iter()
            .filter(|&id| {
                self.arena
                    .get(*id)
                    .map(|node| node.get())
                    .unwrap()
                    .action
                    .action
                    == action
            })
            .cloned()
            .collect()
    }

    fn verify_action_path(datas: &[NodeState]) {
        assert!(datas.last().is_some_and(|v| v.func.is_some()));
        assert!(datas[..datas.len().saturating_sub(1)]
            .iter()
            .all(|v| v.func.is_none()));
    }
}

impl<V> FromIterator<V> for Automaton
where
    V: AsRef<Vec<NodeState>> + AsMut<Vec<NodeState>>,
{
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        let mut arena: Arena<NodeState> = Arena::new();
        let root = arena.new_node(NodeState::new(MaoInteraction::default(), None, None));
        let mut ret = Self {
            arena,
            current_state: root,
            root,
            previous_interactions: vec![],
        };
        for mut datas in iter.into_iter() {
            Self::verify_action_path(datas.as_ref());
            Self::clear_path(datas.as_mut());
            ret.insert_iter(datas.as_ref());
        }
        ret
    }
}

impl<V> Extend<V> for Automaton
where
    V: AsRef<Vec<NodeState>> + AsMut<Vec<NodeState>>,
{
    fn extend<T: IntoIterator<Item = V>>(&mut self, iter: T) {
        for mut datas in iter.into_iter() {
            Self::verify_action_path(datas.as_ref());
            Self::clear_path(&mut datas);
            self.insert_iter(datas.as_ref());
        }
    }
}

impl PartialEq for Automaton {
    fn eq(&self, other: &Self) -> bool {
        self.same_node(self.root, (other, other.root))
    }
}
