use std::io::{stdin, stdout, Write};

use crate::{
    error::Error,
    mao_struct::Mao,
    player::Player,
    stack::{stack_property::StackProperty, stack_type::StackType},
};

pub struct ActionMsgRange {
    msg: String,
    // std::usize::MAX means new one
    possible_values: Vec<usize>,
}

impl ActionMsgRange {
    pub fn new(msg: String, possible_values: Vec<usize>) -> Self {
        Self {
            msg,
            possible_values,
        }
    }

    pub fn get_action(&self) -> Result<usize, Error> {
        let mut ans = String::new();
        let mut index: Option<usize> = None;
        print!("{}", self.msg);
        stdout().flush()?;
        while index.is_none() {
            ans.clear();
            stdin().read_line(&mut ans)?;
            index = ans.trim().parse().ok();
            if index.is_some_and(|v| !self.possible_values.contains(&v)) {
                index = None;
            }
        }
        Ok(index.unwrap())
    }
    pub fn generate_stack_choice_str(
        stack_types: &[StackType],
        mao: &Mao,
        new_stack: bool,
    ) -> Result<Self, Error> {
        if stack_types.is_empty() {
            return Err(Error::GivenSliceEmpty);
        }
        let mut values = mao
            .get_stacks()
            .iter()
            .enumerate()
            .filter(|s| {
                s.1.get_stack_types()
                    .iter()
                    .any(|v| stack_types.contains(v))
            })
            .fold(
                (Vec::<String>::new(), Vec::<usize>::new()),
                |(mut strs, mut indexes), (index, _)| {
                    indexes.push(index);
                    strs.push(format!("Stack ({}),", index));
                    (strs, indexes)
                },
            );
        if values.0.is_empty() {
            return Err(Error::NoStackAvailable {
                stacks: stack_types.to_vec(),
            });
        }
        if new_stack {
            values.0.push(format!("New Stack ({})", std::usize::MAX));
            values.1.push(std::usize::MAX);
        }
        Ok(ActionMsgRange::new(values.0.join("\n") + ": ", values.1))
    }

    pub fn generate_card_choice_str(player: &Player) -> Self {
        Self::new(
            "Starting from 0, give the card's index: ".to_owned(),
            (0..player.get_cards().len()).collect(),
        )
    }

    pub fn generate_nb_cards_draw_choice_str(to: usize) -> Self {
        Self::new(
            format!("How many cards do you want to take (from 0 to {}) ? ", to),
            (1..to).collect(),
        )
    }
}
