use std::io::Write;

use crate::{card::card::Card, error::Error, stack::stack_property::StackProperty};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct Player {
    pseudo: String,
    hand: Vec<Card>, // (rule name, card)
}

impl Player {
    pub fn new(pseudo: String, hand: Vec<Card>) -> Self {
        Self { pseudo, hand }
    }

    pub fn get_pseudo(&self) -> &str {
        &self.pseudo
    }

    /// Prints the [`Card`] as [`Self::print_self_cards`] but with comments
    ///
    /// # Errors
    ///
    /// This function will return an error if [`Self::print_self_cards`] fails
    pub fn print_self_cards_with_cmt(
        &self,
        elevated_index: Option<usize>,
        card_width: Option<usize>,
    ) -> Result<(), Error> {
        println!("Your hand ({}) :\n", self.pseudo);
        self.print_self_cards(elevated_index, card_width)
    }

    /// Prints the [`Card`] nicely like real cards
    ///
    /// # Errors
    ///
    /// This function will return an error if cannot flush data into std::stdout
    pub fn print_self_cards(
        &self,
        elevated_index: Option<usize>,
        card_width: Option<usize>,
    ) -> Result<(), Error> {
        // TODO show hidden card (back)
        let card_width = card_width.unwrap_or(9);
        let elevated_index = elevated_index.unwrap_or(self.hand.len());
        let lines: Vec<Vec<String>> = self
            .hand
            .iter()
            .filter_map(|c| {
                if c.owner_can_see_it() {
                    Some(
                        c.to_string()
                            .split("\n")
                            .map(String::from)
                            .collect::<Vec<String>>(),
                    )
                } else {
                    None
                }
            })
            .collect();

        for i in 0..(7 + 1) {
            for index in 0..lines.len() {
                if index != elevated_index {
                    if i == 0 {
                        print!("\x1b[{}C", card_width);
                    } else {
                        print!("{}", lines[index][i - 1]);
                        print!("\x1b[{}D", 9 - card_width);
                    }
                } else {
                    if i == 7 {
                        print!("\x1b[{}C", card_width);
                    } else {
                        print!("{}", lines[index][i]);
                        print!("\x1b[{}D", 9 - card_width);
                    }
                }
            }
            print!("\n");
        }
        std::io::stdout().flush()?;
        Ok(())
    }
}

impl StackProperty for Player {
    fn get_cards(&self) -> &[Card] {
        &self.hand
    }

    fn get_cards_mut(&mut self) -> &mut Vec<Card> {
        &mut self.hand
    }
}
