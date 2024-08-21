use serde::{Deserialize, Serialize};
use super::CL;


#[derive(Debug, Clone)]
pub enum Direction {
    Buy,
    Sell,
}

impl ToString for Direction {
    fn to_string(&self) -> String {
        match self {
            Direction::Buy => "buy".to_string(),
            Direction::Sell => "sell".to_string(),
        }
    }
}



#[derive(Debug, Clone)]
pub struct Order {
    pub player_name: String,
    pub card: Card,
    pub direction: Direction,
    pub price: Option<usize>,
}


#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub enum Card {
    Spade,
    Club,
    Diamond,
    Heart,
}

impl ToString for Card {
    fn to_string(&self) -> String {
        match self {
            Card::Spade => "spade".to_string(),
            Card::Club => "club".to_string(),
            Card::Diamond => "diamond".to_string(),
            Card::Heart => "heart".to_string(),
        }
    }
}

impl Card {
    pub fn get_other_cards(&self) -> (Card, Card, Card) { // common_suite, suit_1, suit_2
        match self {
            Card::Spade => (Card::Club, Card::Diamond, Card::Heart),
            Card::Club => (Card::Spade, Card::Diamond, Card::Heart),
            Card::Heart => (Card::Diamond, Card::Spade, Card::Club),
            Card::Diamond => (Card::Heart, Card::Spade, Card::Club),
        }
    }

    pub fn get_goal_suit(&self) -> Card {
        match self {
            Card::Spade => Card::Club,
            Card::Club => Card::Spade,
            Card::Heart => Card::Diamond,
            Card::Diamond => Card::Heart,
        }
    }

    pub fn get_book_colors(&self) -> (CL, CL, CL, CL) {
        match self {
            Card::Spade => (CL::LimeGreen, CL::DullTeal, CL::DullTeal, CL::DullTeal),
            Card::Club => (CL::DullTeal, CL::LimeGreen, CL::DullTeal, CL::DullTeal),
            Card::Heart => (CL::DullTeal, CL::DullTeal, CL::DullTeal, CL::LimeGreen),
            Card::Diamond => (CL::DullTeal, CL::DullTeal, CL::LimeGreen, CL::DullTeal),
        }

        // (spades_color, clubs_color, diamonds_color, hearts_color) | apologies for the diagonal line not linin
    } 
}




#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Inventory {
    pub spades: usize,
    pub clubs: usize,
    pub diamonds: usize,
    pub hearts: usize,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            spades: 0,
            clubs: 0,
            diamonds: 0,
            hearts: 0,
        }
    }

    pub fn count(&mut self, cards: Vec<Card>) {
        for card in cards {
            match card {
                Card::Spade => self.spades += 1,
                Card::Club => self.clubs += 1,
                Card::Diamond => self.diamonds += 1,
                Card::Heart => self.hearts += 1,
            }
        }
    }

    pub fn change(&mut self, card: Card, add: bool) {
        match card {
            Card::Spade => {
                let new_amount: usize;
                if add {
                    new_amount = self.spades + 1;
                } else {
                    new_amount = self.spades - 1;
                }
                self.spades = new_amount;
            },
            Card::Club => {
                let new_amount: usize;
                if add {
                    new_amount = self.clubs + 1;
                } else {
                    new_amount = self.clubs - 1;
                }
                self.clubs = new_amount;
            },
            Card::Diamond => {
                let new_amount: usize;
                if add {
                    new_amount = self.diamonds + 1;
                } else {
                    new_amount = self.diamonds - 1;
                }
                self.diamonds = new_amount;
            },
            Card::Heart => {
                let new_amount: usize;
                if add {
                    new_amount = self.hearts + 1;
                } else {
                    new_amount = self.hearts - 1;
                }
                self.hearts = new_amount;
            },
        }
    }

    pub fn get(&self, card: &Card) -> usize {
        match card {
            Card::Spade => self.spades,
            Card::Club => self.clubs,
            Card::Diamond => self.diamonds,
            Card::Heart => self.hearts,
        }
    }
}