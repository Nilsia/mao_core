pub mod card;
pub mod config;
pub mod error;
pub mod mao;
pub mod mao_event;
pub mod player;
pub mod rule;
pub mod stack;

pub const VERSION: &str = "1.0";

#[cfg(test)]
mod test {
    use super::mao::automaton::*;
    use crate::mao::mao_action::MaoInteraction;

    fn generate_path() -> Vec<Vec<NodeState>> {
        vec![
            vec![
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectCard),
                    None,
                    None,
                ),
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectPlayableStack),
                    Some(|_, _, _| Ok(vec![])),
                    None,
                ),
            ],
            vec![NodeState::new(
                MaoInteraction::new(None, PlayerAction::SelectDrawableStack),
                Some(|_, _, _| Ok(vec![])),
                None,
            )],
        ]
    }

    fn actions_to_add() -> Vec<Vec<NodeState>> {
        vec![
            vec![
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectCard),
                    None,
                    Some(String::from("actions to add")),
                ),
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectCard),
                    None,
                    Some(String::from("actions to add")),
                ),
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectDiscardableStack),
                    Some(|_, _, _| Ok(vec![])),
                    Some(String::from("actions to add")),
                ),
            ],
            vec![NodeState::new(
                MaoInteraction::new(None, PlayerAction::SelectPlayableStack),
                Some(|_, _, _| Ok(vec![])),
                Some(String::from("actions to add")),
            )],
        ]
    }

    #[test]
    fn extend_automaton() {
        let initial_actions = generate_path();

        let mut modified_actions = initial_actions.clone();
        modified_actions.extend_from_slice(&actions_to_add());

        let mut init_auto = Automaton::from_iter(initial_actions);
        init_auto.extend(actions_to_add());

        assert_eq!(init_auto, Automaton::from_iter(modified_actions));
    }

    #[test]
    fn remove_automaton_path() {
        let mut initial_actions = generate_path();
        let init_auto = Automaton::from_iter(&mut initial_actions);

        let mut other_act = initial_actions.to_owned();
        other_act.extend(actions_to_add());
        let mut other_auto = Automaton::from_iter(&mut other_act);
        other_auto.remove_paths(&mut actions_to_add());

        assert_eq!(init_auto, other_auto);
    }

    #[test]
    fn invert_automaton_path() {
        let initial_actions = actions_to_add();
        let mut inverted_actions = initial_actions.to_owned();
        inverted_actions.reverse();

        let init_auto = Automaton::from_iter(initial_actions);
        let inv_auto = Automaton::from_iter(inverted_actions);

        assert_eq!(init_auto, inv_auto);
    }
}
