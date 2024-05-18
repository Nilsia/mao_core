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

    pub fn print_self_cards_with_cmt(
        &self,
        elevated_index: Option<usize>,
        card_width: Option<usize>,
    ) -> Result<(), Error> {
        println!("Your hand ({}) :\n", self.pseudo);
        self.print_self_cards(elevated_index, card_width)
    }

    pub fn print_self_cards(
        &self,
        elevated_index: Option<usize>,
        card_width: Option<usize>,
    ) -> Result<(), Error> {
        let card_width = card_width.unwrap_or(9);
        let elevated_index = elevated_index.unwrap_or(self.hand.len());
        let lines: Vec<Vec<String>> = self
            .hand
            .iter()
            .map(|c| {
                c.to_string()
                    .split("\n")
                    .map(String::from)
                    .collect::<Vec<String>>()
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
