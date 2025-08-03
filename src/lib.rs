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

    use std::sync::Arc;

    use super::config::OneOrMoreWords;
    use super::mao::automaton::*;
    use crate::{
        card::{card_type::CardType, card_value::CardValue, common_card_type::CommonCardType},
        config::{
            CardEffects, CardEffectsInner, CardEffectsKey, CardEffectsStruct, CardPlayerAction,
            RuleCardsEffect, SingleCardEffect,
        },
        mao::{
            mao_action::MaoInteraction,
            mao_core::{PlayerTurnChange, PlayerTurnUpdater},
        },
    };

    fn generate_path_soft() -> Vec<Vec<NodeState>> {
        let mut actions = ActionsBuilder::build();
        NodeStatesBuilder::new()
            .add_next_node(NodeState::action(PlayerAction::SelectCard))
            .add_next_node(NodeState::action(PlayerAction::SelectPlayableStack))
            .last_node_function(|_, _, _| Ok(vec![]))
            .insert_into(&mut actions);
        NodeStatesBuilder::new()
            .add_next_node(NodeState::action(PlayerAction::SelectDrawableStack))
            .last_node_function(|_, _, _| Ok(vec![]))
            .insert_into(&mut actions);
        actions.into()
    }

    fn generate_path_hard() -> Vec<Vec<NodeState>> {
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

    fn actions_to_add_soft() -> Vec<Vec<NodeState>> {
        let rule: Arc<str> = Arc::from("rule");
        let mut actions = ActionsBuilder::build().rule(&rule);
        NodeStatesBuilder::new()
            .add_next_action(PlayerAction::SelectCard)
            .add_next_action(PlayerAction::SelectCard)
            .add_next_action(PlayerAction::SelectDiscardableStack)
            .last_node_function(|_, _, _| Ok(vec![]))
            .insert_into(&mut actions);
        NodeState::builder()
            .add_next_action(PlayerAction::SelectPlayableStack)
            .last_node_function(|_, _, _| Ok(vec![]))
            .insert_into(&mut actions);
        actions.into()
    }

    fn actions_to_add() -> Vec<Vec<NodeState>> {
        let rule: Arc<str> = Arc::from("rule");
        vec![
            vec![
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectCard),
                    None,
                    Some(Arc::clone(&rule)),
                ),
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectCard),
                    None,
                    Some(Arc::clone(&rule)),
                ),
                NodeState::new(
                    MaoInteraction::new(None, PlayerAction::SelectDiscardableStack),
                    Some(|_, _, _| Ok(vec![])),
                    Some(Arc::clone(&rule)),
                ),
            ],
            vec![NodeState::new(
                MaoInteraction::new(None, PlayerAction::SelectPlayableStack),
                Some(|_, _, _| Ok(vec![])),
                Some(Arc::clone(&rule)),
            )],
        ]
    }

    /// Returns true if all Arc points to the same memory address
    fn check_rules_arc(nodes: &[Vec<NodeState>]) -> bool {
        let ptr = nodes
            .first()
            .unwrap()
            .first()
            .unwrap()
            .rule
            .as_ref()
            .unwrap()
            .as_ptr();
        for _nodes in nodes.iter() {
            for node in _nodes {
                if node.rule.as_ref().unwrap().as_ptr() != ptr {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn path_equal() {
        assert_eq!(generate_path_hard(), generate_path_soft());

        let actions_to_add = actions_to_add();
        let actions_to_add_soft = actions_to_add_soft();
        assert_eq!(actions_to_add, actions_to_add_soft);

        // check pointers
        assert!(check_rules_arc(&actions_to_add_soft));
        assert!(check_rules_arc(&actions_to_add));
    }

    #[test]
    fn extend_automaton() {
        let initial_actions = generate_path_hard();

        let mut modified_actions = initial_actions.clone();
        modified_actions.extend_from_slice(&actions_to_add());

        let mut init_auto = Automaton::from_iter(initial_actions);
        init_auto.extend(actions_to_add());

        assert_eq!(init_auto, Automaton::from_iter(modified_actions));
    }

    #[test]
    fn remove_automaton_path() {
        let mut initial_actions = generate_path_hard();
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

    #[test]
    fn card_effect_struct_deserialisation() -> anyhow::Result<()> {
        let data = r#"
        1_diamond = [
            { effect = {type = "say", values = ["diamond", ["1", "as", "one"]]}, rule_info = {rule_name = "Rulename1"} },
            {effects = {type = "physical", values = "punch"}},
            {effect = "update_update_2"}
        ]
        8 = {effect = "rotate_update_2"}
        5 = {effect = {type = "physical", values = "cry"}}"#;
        let b: CardEffectsStruct = toml::from_str(data)?;
        let mut c = CardEffectsStruct::default();
        c.insert(
            CardEffectsKey::new(
                Some(CardType::Common(CommonCardType::Diamond)),
                Some(CardValue::Number(1)),
            ),
            CardEffects::multiple(vec![
                CardEffectsInner::new(
                    SingleCardEffect::CardPlayerAction(CardPlayerAction::Say(vec![
                        OneOrMoreWords(vec!["diamond".to_owned()]),
                        OneOrMoreWords(
                            vec!["1", "as", "one"]
                                .iter()
                                .map(|&s| String::from(s))
                                .collect(),
                        ),
                    ])),
                    RuleCardsEffect {
                        rule_name: "Rulename1".to_owned(),
                        error_message: None,
                    },
                ),
                CardEffectsInner::only(SingleCardEffect::CardPlayerAction(
                    CardPlayerAction::Physical("punch".to_owned()),
                )),
                CardEffectsInner::only(SingleCardEffect::PlayerTurnChange(
                    PlayerTurnChange::Update(PlayerTurnUpdater::Update(2)),
                )),
            ]),
        );
        c.insert(
            CardEffectsKey::new(None, Some(CardValue::Number(8))),
            CardEffects::single(CardEffectsInner::only(SingleCardEffect::PlayerTurnChange(
                PlayerTurnChange::Rotate(PlayerTurnUpdater::Update(2)),
            ))),
        );
        c.insert(
            CardEffectsKey::new(None, Some(CardValue::Number(5))),
            CardEffects::single(CardEffectsInner::only(SingleCardEffect::CardPlayerAction(
                CardPlayerAction::Physical("cry".to_owned()),
            ))),
        );
        assert_eq!(b, c);
        Ok(())
    }
}
