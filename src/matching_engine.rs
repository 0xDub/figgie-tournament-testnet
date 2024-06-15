use super::{HTTPResponse, EndRoundUpdate, Card, Inventory, Order, Update, Trade, Direction, CL, CardBook};
use rand::prelude::SliceRandom;
use std::collections::HashMap;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::stream::SplitSink;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use futures_util::SinkExt;
use serde_json::json;


pub struct MatchingEngine {
    pub player_names: Vec<String>,
    pub starting_balance: i32,
    pub suits: [Card; 4],
    pub goal_suit: Card,
    pub common_suit: Card,
    pub player_points: HashMap<String, i32>,
    pub spades_book: CardBook,
    pub clubs_book: CardBook,
    pub diamonds_book: CardBook,
    pub hearts_book: CardBook,
    pub pot: usize,
    pub ante: usize,
    pub player_inventories: HashMap<String, Inventory>,
    pub initial_points: HashMap<String, i32>,
    pub starting_inventory: HashMap<Card, usize>,
    pub player_ws_map_hotpath: Arc<Mutex<HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>>>,
    pub rng: StdRng,
}


impl MatchingEngine {
    pub fn new(
        starting_balance: i32,
        player_ws_map_hotpath: Arc<Mutex<HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>>>,
    ) -> Self {

        Self {
            player_names: Vec::new(),
            starting_balance,
            suits: [Card::Spade, Card::Club, Card::Diamond, Card::Heart],
            goal_suit: Card::Spade,
            common_suit: Card::Club,
            player_points: HashMap::new(),
            spades_book: CardBook::new(),
            clubs_book: CardBook::new(),
            diamonds_book: CardBook::new(),
            hearts_book: CardBook::new(),
            pot: 0,
            ante: 0,
            player_inventories: HashMap::new(),
            initial_points: HashMap::new(),
            starting_inventory: HashMap::new(),
            player_ws_map_hotpath,
            rng: StdRng::from_entropy(),
        }
    }

    pub fn add_new_player_with_inventory(&mut self, player_name: String, inventory: Inventory) {
        self.player_names.push(player_name.clone());
        self.player_points.insert(player_name.clone(), self.starting_balance);
        self.player_inventories.insert(player_name.clone(), inventory);
        self.initial_points.insert(player_name.clone(), self.starting_balance);
    }

    pub fn print_all_players(&self) {
        println!("Players: {:?}", self.player_names);
    }

    pub fn delete_all_players(&mut self) {
        self.player_names.clear();
        self.player_points.clear();
        self.player_inventories.clear();
    }

    pub fn pick_new_common_suit(&mut self) {
        self.common_suit = self.suits[self.rng.gen_range(0..=3)].clone();
    }

    pub fn get_player_inventory(&self, player_name: &String) -> Inventory {
        self.player_inventories.get(player_name).unwrap().clone()
    }

    pub fn get_new_inventories(&mut self) -> HashMap<Card, usize> {
        let mut cards: Vec<Card> = Vec::new();
        let (goal_suit, suit_1, suit_2) = self.common_suit.get_other_cards();
        self.goal_suit = goal_suit.clone();

        for _ in 0..12 { cards.push(self.common_suit.clone()) }
        
        let mut starting_inventory = HashMap::new();

        println!("=---= Card Count =---=");
        println!("{} - {:?} | 12x{}", CL::Dull.get(), self.common_suit, CL::End.get());
        starting_inventory.insert(self.common_suit.clone(), 12);

        // randomly pick one of the other 3 suits to be the one with 8 cards
        let mut already_lucky = false;
        for (idx, suit) in [suit_1, suit_2, goal_suit].iter().enumerate() {
            let lucky_eight = rand::random::<bool>();
            if idx == 2 && !already_lucky {
                for _ in 0..8 { cards.push(suit.clone()) }
                println!("{} - {:?} | 8x{}", CL::Dull.get(), suit, CL::End.get());
                starting_inventory.insert(suit.clone(), 8);
            } else {
                if !already_lucky && lucky_eight {
                    for _ in 0..8 { cards.push(suit.clone()) }
                    println!("{} - {:?} | 8x{}", CL::Dull.get(), suit, CL::End.get());
                    starting_inventory.insert(suit.clone(), 8);
                    already_lucky = true;
                } else {
                    for _ in 0..10 { cards.push(suit.clone()) }
                    println!("{} - {:?} | 10x{}", CL::Dull.get(), suit, CL::End.get());
                    starting_inventory.insert(suit.clone(), 10);
                }
            }
        }

        cards.shuffle(&mut self.rng); // randomly shuffle the cards

        for (_, player_name) in self.player_names.iter().enumerate() { // for the testnet, we're not going to randomly draw cards - send out 3x of each
            let player_inventory = Inventory { spades: 3, clubs: 3, diamonds: 3, hearts: 3 };
            self.player_inventories.insert(player_name.clone(), player_inventory.clone());
        }

        starting_inventory
    }


    pub async fn start_round(&mut self, round_number: usize) {
        self.pot = 200; // make sure the pot is always 200 no matter the number of players (this is only for the testnet)
        self.ante = 50;

        println!("{}==================== ROUND {} ===================={}", CL::Purple.get(), round_number, CL::End.get());
        println!("");
        println!("=---= Game Details =---=");
        println!("{} - Players: {}x{}", CL::Dull.get(), self.player_names.len(), CL::End.get());
        println!("{} - Ante: {}{}", CL::Dull.get(), self.ante, CL::End.get());
        println!("{} - Pot: 200{}", CL::Dull.get(), CL::End.get());
        println!("");
        
        self.initial_points = self.player_points.clone();
        for (player, points) in self.player_points.iter_mut() {
            // - Not worried about this for the testnet, probably going to void this in the real game as well
            // if *points < self.ante as i32 {
            //     println!("[!] Player {:?} does not have enough points to play", player);
            //     break;
            // }
            *points -= self.ante as i32;
            self.pot += self.ante;
        }

        self.pick_new_common_suit();
        self.starting_inventory = self.get_new_inventories();

        println!("{} - Common suit: {:?}{}", CL::Dull.get(), self.common_suit, CL::End.get());
        println!("{} - Goal suit: {}{:?}{}{}", CL::Dull.get(), CL::LimeGreen.get(), self.goal_suit, CL::End.get(), CL::End.get());
        println!("");

        println!("{}[+] Dealing cards...{}\n", CL::DimLightBlue.get(), CL::End.get());
        
        self.spades_book.reset_full_book();
        self.clubs_book.reset_full_book();
        self.diamonds_book.reset_full_book();
        self.hearts_book.reset_full_book();

        self.deal_cards().await;
    }

    pub async fn end_game(&mut self) {
        let full_update = json!({
            "kind": "end_game",
            "data": "Testnet game is over. Restarting another game in 15 seconds. All player_names and player_ids will be reset so please send another post to /register_testnet to get a new player_name",
        });
        let serialized_update = full_update.to_string();

        let mut removed_players = Vec::new();
        for (player_name, sender) in self.player_ws_map_hotpath.lock().await.iter_mut() {
            if let Err(_) = sender.send(Message::Text(serialized_update.clone())).await {
                println!("{}[!] Error sending message to player | Deleting from the map. Player must resubscribe{}", CL::Red.get(), CL::End.get());
                removed_players.push(player_name.clone());
            }
        }

        for player_name in removed_players {
            self.player_ws_map_hotpath.lock().await.remove(&player_name);
        }
    }

    pub async fn end_round(&mut self) {
        // =-= End the Round =-= //

        println!("");
        println!("{}=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-={}", CL::Pink.get(), CL::End.get());
        println!("{}=-=-=-=-=-=-=-=-=-=-=-=-=-=-= Round over! =-=-=-=-=-=-=-=-=-=-=-=-=-=-={}", CL::Pink.get(), CL::End.get());
        println!("{}=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-={}", CL::Pink.get(), CL::End.get());
        println!("");
        
        println!("=---= Game Details =---=");
        println!("{} - Players: {}x{}", CL::Dull.get(), self.player_names.len(), CL::End.get());
        println!("{} - Ante: {}{}", CL::Dull.get(), self.ante, CL::End.get());
        println!("{} - Pot: {}{}", CL::Dull.get(), self.pot, CL::End.get());
        println!("");
        println!("=---= Card Count =---=");
        for (suit, amount) in self.starting_inventory.clone() {
            println!("{} - {:?} | {}x{}", CL::Dull.get(), suit, amount, CL::End.get());
        }
        println!("{} - Common suit: {:?}{}", CL::Dull.get(), self.common_suit, CL::End.get());
        println!("{} - Goal suit: {}{:?}{}{}", CL::Dull.get(), CL::LimeGreen.get(), self.goal_suit, CL::End.get(), CL::End.get());
        println!("");

        // calculate the scores, each player is awared goal_suit * 10

        let mut winner: (String, usize) = ("".to_string(), 0); // player_id, goal_cards
        let mut tied_winnders: Vec<String> = Vec::new(); // player_ids

        println!("=---------------------------- Inventory ----------------------------=");
        for player_name in &self.player_names {
            let inventory = self.player_inventories.get(player_name).unwrap();
            let player_points = self.player_points.get_mut(player_name).unwrap();
            let goal_cards = match self.goal_suit {
                Card::Spade => inventory.spades,
                Card::Club => inventory.clubs,
                Card::Diamond => inventory.diamonds,
                Card::Heart => inventory.hearts,
            };

            let (spade_color, club_color, diamond_color, heart_color) = match self.goal_suit {
                Card::Spade => (CL::LimeGreen.get(), CL::Dull.get(), CL::Dull.get(), CL::Dull.get()),
                Card::Club => (CL::Dull.get(), CL::LimeGreen.get(), CL::Dull.get(), CL::Dull.get()),
                Card::Diamond => (CL::Dull.get(), CL::Dull.get(), CL::LimeGreen.get(), CL::Dull.get()),
                Card::Heart => (CL::Dull.get(), CL::Dull.get(), CL::Dull.get(), CL::LimeGreen.get()),
            };

            println!("{}{}{}{} |:| Spades: {}{}x{} | Clubs: {}{}x{} | Diamonds: {}{}x{} | Hearts: {}{}x{}{}", CL::Dull.get(), CL::DimLightBlue.get(), player_name, CL::Dull.get(), spade_color, inventory.spades, CL::Dull.get(), club_color, inventory.clubs, CL::Dull.get(), diamond_color, inventory.diamonds, CL::Dull.get(), heart_color, inventory.hearts, CL::End.get(), CL::End.get());

            if goal_cards >= winner.1 {
                if goal_cards == winner.1 {
                    tied_winnders.push(player_name.clone());
                } else {
                    winner = (player_name.clone(), goal_cards);
                    tied_winnders.clear();
                }
            }

            *player_points += (goal_cards * 10) as i32;
            self.pot -= goal_cards * 10;
        }
        println!("");

        // if there's one winner, award them the pot
        // if there's a tie, split the pot evenly between the winners

        println!("=----------------------------- Results -----------------------------=");
        if winner.0 != "".to_string() {
            if tied_winnders.is_empty() {
                println!("{}[+] Player '{}' wins the whole pot of {} points{}", CL::Green.get(), winner.0, self.pot, CL::End.get());
                let winner_points = self.player_points.get_mut(&winner.0).unwrap();
                *winner_points += self.pot as i32;
            } else {
                let split = self.pot / (tied_winnders.len() + 1);
                println!("{}[+] Players tie for the pot of {} points{}\n", CL::Teal.get(), self.pot, CL::End.get());
                println!("{}------ Tied Players ------{}", CL::Dull.get(), CL::End.get());
                println!("{}{}{}{} | Goal Cards: {}x | Points: {}+{}x{}{}", CL::Dull.get(), CL::DimLightBlue.get(), winner.0, CL::Dull.get(), winner.1, CL::LimeGreen.get(), split, CL::End.get(), CL::End.get());
                for player_name in tied_winnders {
                    println!("{}{}{}{} | Goal Cards: {}x | Points: {}+{}x{}{}", CL::Dull.get(), CL::DimLightBlue.get(), player_name, CL::Dull.get(), winner.1, CL::LimeGreen.get(), split, CL::End.get(), CL::End.get());
                    let player_points = self.player_points.get_mut(&player_name).unwrap();
                    *player_points += split as i32;
                }
            }
        } else {
            println!("{}[-] Huh, looks like no one is playing right now{}", CL::Dull.get(), CL::End.get());
        }
        
        println!("");

        println!("=-------------------------- Updated Points -------------------------=");
        let mut inventory_string = String::from("");
        for player_name in &self.player_names {
            let initial_points = self.initial_points.get(player_name).unwrap();
            let player_points = self.player_points.get(player_name).unwrap();
            let point_change: i32 = *player_points as i32 - *initial_points as i32;

            let change_color = match point_change {
                x if x > 0 => CL::Green.get(),
                x if x < 0 => CL::Red.get(),
                _ => CL::Dull.get(),
            };

            inventory_string += &format!("{}: {} {}({}){} | ", player_name, player_points, change_color, point_change, CL::Dull.get());
        }
        inventory_string.truncate(inventory_string.len() - 3);
        println!("{}{}{}", CL::Dull.get(), inventory_string, CL::End.get());
        println!("");

        self.send_end_round_message().await;
    }


    pub async fn process_order(&mut self, order: Order) -> HTTPResponse {
        //let start = minstant::Instant::now();

        // quick check that all the HashMaps have the player_name before we start
        if !self.player_inventories.contains_key(&order.player_name) || !self.player_points.contains_key(&order.player_name) {
            return HTTPResponse {
                status: "UNKNOWN_PLAYER".to_string(),
                message: "Player does not exist. Please post to /register_testnet first with your chosen playerid in the headers and no body. You can choose anything on the testnet".to_string(),
            };
        }

        let book = match order.card {
            Card::Spade => &mut self.spades_book,
            Card::Club => &mut self.clubs_book,
            Card::Diamond => &mut self.diamonds_book,
            Card::Heart => &mut self.hearts_book,
        };

        let trade: Option<Trade> = match order.direction {
            Direction::Buy => {

                let player_points = self.player_points.get(&order.player_name).unwrap();
                if *player_points < order.price as i32 {
                    return HTTPResponse {
                        status: "INSUFFICIENT_FUNDS".to_string(),
                        message: "You don't have enough points to buy this card!".to_string(),
                    };
                }

                if let Some(best_ask) = book.asks.first() {
                    if order.price >= best_ask.price {
                        //println!("{}[-] Aggressing Player: {:?} | {:?} |:| Matched buy order!{}", CL::Green.get(), order.player_name, order.card, CL::End.get());

                        if order.player_name == best_ask.player_name {
                            return HTTPResponse {
                                status: "SELF_TRADE".to_string(),
                                message: "You can't trade with yourself!".to_string(),
                            };
                        }

                        // we don't need to update either book since both will be reset after the trade

                        // =-= Update the Inventories =-= //
                        let buyer_inventory = self.player_inventories.get_mut(&order.player_name).unwrap();
                        buyer_inventory.change(order.card, true);
    
                        let seller_inventory = self.player_inventories.get_mut(&best_ask.player_name).unwrap();
                        seller_inventory.change(order.card, false);
    
    
                        // =-= Update the Points =-= //
                        let buyer_points = self.player_points.get_mut(&order.player_name).unwrap();
                        *buyer_points -= best_ask.price as i32;
    
                        let seller_points = self.player_points.get_mut(&best_ask.player_name).unwrap();
                        *seller_points += best_ask.price as i32;
    
    
                        // =-= Package Trade =-= //
                        book.last_trade = Some(best_ask.price);
                        let trade = Trade {
                            card: order.card,
                            price: best_ask.price,
                            buyer: order.player_name,
                            seller: best_ask.player_name.clone(),
                        };
                        Some(trade)
    
                    } else {
                        book.update_bid(order.price, order.player_name);
                        None
                    }
                } else {
                    book.update_bid(order.price, order.player_name);
                    None
                }
                
            },
            Direction::Sell => {

                // check if the user has the inventory to sell this Card
                let seller_inventory = self.player_inventories.get(&order.player_name).unwrap();
                if seller_inventory.get(&order.card) == 0 {
                    return HTTPResponse {
                        status: "NO_INVENTORY".to_string(),
                        message: format!("Card: {}, You don't have enough inventory to place this trade", order.card.to_string().to_lowercase()),
                    };
                }

                if let Some(best_bid) = book.bids.first() {
                    if order.price <= best_bid.price {
                        //println!("{}[-] Aggressing Player: {:?} | {:?} |:| Matched sell order!{}", CL::Red.get(), order.player_name, order.card, CL::End.get());
    
                        if order.player_name == best_bid.player_name {
                            return HTTPResponse {
                                status: "SELF_TRADE".to_string(),
                                message: "You can't trade with yourself!".to_string(),
                            };
                        }

                        // we don't need to update either book since both will be reset after the trade

                        // =-= Update the Inventories =-= //
                        let buyer_inventory = self.player_inventories.get_mut(&best_bid.player_name).unwrap();
                        buyer_inventory.change(order.card, true);
    
                        let seller_inventory = self.player_inventories.get_mut(&order.player_name).unwrap();
                        seller_inventory.change(order.card, false);
    
    
                        // =-= Update the Points =-= //
                        let buyer_points = self.player_points.get_mut(&best_bid.player_name).unwrap();
                        *buyer_points -= best_bid.price as i32;
    
                        let seller_points = self.player_points.get_mut(&order.player_name).unwrap();
                        *seller_points += best_bid.price as i32;
    
    
                        // =-= Package Trade =-= //
                        book.last_trade = Some(best_bid.price);
                        let trade = Trade {
                            card: order.card,
                            price: best_bid.price,
                            buyer: best_bid.player_name.clone(),
                            seller: order.player_name,
                        };
                        Some(trade)
    
                    } else {
                        book.update_ask(order.price, order.player_name);
                        None
                    }
                } else {
                    book.update_ask(order.price, order.player_name);
                    None
                }
                
            },
        };

        if let Some(_) = trade.clone() {
            // =-= Reset all the Books =-= //
            // - Like the website, we'll reset all the books after a match occurs
            self.spades_book.reset_quotes();
            self.clubs_book.reset_quotes();
            self.diamonds_book.reset_quotes();
            self.hearts_book.reset_quotes();
        }

        // =-= Print the Game =-= //
        println!("\n=---------------------------------------------------------------------------------=");
        for (player_name, inventory) in &self.player_inventories {
            let (spade_color, club_color, diamond_color, heart_color) = match self.goal_suit {
                Card::Spade => (CL::LimeGreen.get(), CL::Dull.get(), CL::Dull.get(), CL::Dull.get()),
                Card::Club => (CL::Dull.get(), CL::LimeGreen.get(), CL::Dull.get(), CL::Dull.get()),
                Card::Diamond => (CL::Dull.get(), CL::Dull.get(), CL::LimeGreen.get(), CL::Dull.get()),
                Card::Heart => (CL::Dull.get(), CL::Dull.get(), CL::Dull.get(), CL::LimeGreen.get()),
            };
            println!("{}{}{}{} |:| Spades: {}{}x{} | Clubs: {}{}x{} | Diamonds: {}{}x{} | Hearts: {}{}x{}{}", CL::Dull.get(), CL::DimLightBlue.get(), player_name, CL::Dull.get(), spade_color, inventory.spades, CL::Dull.get(), club_color, inventory.clubs, CL::Dull.get(), diamond_color, inventory.diamonds, CL::Dull.get(), heart_color, inventory.hearts, CL::End.get(), CL::End.get());
        }
        println!("");
        let spades_bid = self.spades_book.get_best_bid();
        let spades_ask = self.spades_book.get_best_ask();
        let spades_last_trade = self.spades_book.last_trade;
        let clubs_bid = self.clubs_book.get_best_bid();
        let clubs_ask = self.clubs_book.get_best_ask();
        let clubs_last_trade = self.clubs_book.last_trade;
        let diamonds_bid = self.diamonds_book.get_best_bid();
        let diamonds_ask = self.diamonds_book.get_best_ask();
        let diamonds_last_trade = self.diamonds_book.last_trade;
        let hearts_bid = self.hearts_book.get_best_bid();
        let hearts_ask = self.hearts_book.get_best_ask();
        let hearts_last_trade = self.hearts_book.last_trade;
        let (spades_color, clubs_color, diamonds_color, hearts_color) = self.goal_suit.get_book_colors();
        println!("{}Spades    {}|:| Bid: ({}{:?}{}, {:?}) | Ask: ({}{:?}{}, {:?}) |:|{} Last trade: {}{:?}{}", spades_color.get(), CL::Dull.get(), CL::Green.get(), spades_bid.0,    CL::Dull.get(), spades_bid.1,    CL::PeachRed.get(),  spades_ask.0,    CL::Dull.get(),  spades_ask.1,    CL::Dull.get(),  CL::DimLightBlue.get(),  spades_last_trade,    CL::End.get());
        println!("{}Clubs     {}|:| Bid: ({}{:?}{}, {:?}) | Ask: ({}{:?}{}, {:?}) |:|{} Last trade: {}{:?}{}", clubs_color.get(), CL::Dull.get(), CL::Green.get(), clubs_bid.0,     CL::Dull.get(), clubs_bid.1,     CL::PeachRed.get(),  clubs_ask.0,     CL::Dull.get(),  clubs_ask.1,     CL::Dull.get(),  CL::DimLightBlue.get(),  clubs_last_trade,     CL::End.get());
        println!("{}Diamonds  {}|:| Bid: ({}{:?}{}, {:?}) | Ask: ({}{:?}{}, {:?}) |:|{} Last trade: {}{:?}{}", diamonds_color.get(), CL::Dull.get(), CL::Green.get(), diamonds_bid.0,  CL::Dull.get(), diamonds_bid.1,  CL::PeachRed.get(),  diamonds_ask.0,  CL::Dull.get(),  diamonds_ask.1,  CL::Dull.get(),  CL::DimLightBlue.get(),  diamonds_last_trade,  CL::End.get());
        println!("{}Hearts    {}|:| Bid: ({}{:?}{}, {:?}) | Ask: ({}{:?}{}, {:?}) |:|{} Last trade: {}{:?}{}", hearts_color.get(), CL::Dull.get(), CL::Green.get(), hearts_bid.0,    CL::Dull.get(), hearts_bid.1,    CL::PeachRed.get(),  hearts_ask.0,    CL::Dull.get(),  hearts_ask.1,    CL::Dull.get(),  CL::DimLightBlue.get(),  hearts_last_trade,    CL::End.get());
        let mut inventory_string = format!("{}Points    {}|:|{} ", CL::DullGreen.get(), CL::Dull.get(), CL::DullGreen.get());
        for player_name in &self.player_names {
            let player_points = self.player_points.get(player_name).unwrap();
            inventory_string += &format!("{}: {} | ", player_name, player_points);
        }
        inventory_string.truncate(inventory_string.len() - 3);
        println!("{}{}", inventory_string, CL::End.get());
        println!("=---------------------------------------------------------------------------------=\n");

        let book_event = Update {
            spades: self.spades_book.clone(),
            clubs: self.clubs_book.clone(),
            diamonds: self.diamonds_book.clone(),
            hearts: self.hearts_book.clone(),
            trade,
        };

        let full_update = json!({
            "kind": "update",
            "data": book_event,
        });

        //let elapsed = start.elapsed().as_micros();
        //println!("{}[+] Time taken to process the order: {} microseconds{}", CL::DullTeal.get(), elapsed, CL::End.get());

        let mut removed_players = Vec::new();
        for (player_name, sender) in self.player_ws_map_hotpath.lock().await.iter_mut() {
            if let Err(_) = sender.send(Message::Text(full_update.to_string())).await {
                println!("{}[!] Error sending message to player | Deleting from the map. Player must resubscribe{}", CL::Red.get(), CL::End.get());
                removed_players.push(player_name.clone());
            }
        }

        for player_name in removed_players {
            self.player_ws_map_hotpath.lock().await.remove(&player_name);
        }

        return HTTPResponse {
            status: "SUCCESS".to_string(),
            message: format!("{},{},{}", order.card.to_string().to_lowercase(), order.direction.to_string().to_lowercase(), order.price),
        };
    }


    pub async fn send_end_round_message(&mut self) {

        let end_round_update = EndRoundUpdate {
            card_count: self.starting_inventory.clone(),
            player_inventories: self.player_inventories.clone(),
            player_points: self.player_points.clone(),
            goal_suit: self.goal_suit.clone(),
            common_suit: self.common_suit.clone(),
        };
        let full_update = json!({
            "kind": "end_round",
            "data": end_round_update,
        });
        let serialized_update = full_update.to_string();

        let mut removed_players = Vec::new();
        for (player_name, sender) in self.player_ws_map_hotpath.lock().await.iter_mut() {
            if let Err(_) = sender.send(Message::Text(serialized_update.clone())).await {
                println!("{}[!] Error sending message to player | Deleting from the map. Player must resubscribe{}", CL::Red.get(), CL::End.get());
                removed_players.push(player_name.clone());
            }
        }

        for player_name in removed_players {
            self.player_ws_map_hotpath.lock().await.remove(&player_name);
        }
    }


    pub async fn deal_cards(&mut self) {
        let mut removed_players = Vec::new();
        for (player_name, sender) in self.player_ws_map_hotpath.lock().await.iter_mut() {

            let inventory = self.player_inventories.get(player_name);
            if let Some(inventory) = inventory {
                let full_update = json!({
                    "kind": "dealing_cards",
                    "data": inventory,
                });
    
                if let Err(_) = sender.send(Message::Text(full_update.to_string())).await {
                    println!("{}[!] Error sending message to player | Deleting from the map. Player must resubscribe{}", CL::Red.get(), CL::End.get());
                    removed_players.push(player_name.clone());
                }
            }
        }

        for player_name in removed_players {
            self.player_ws_map_hotpath.lock().await.remove(&player_name);
        }

        println!("{}[+] Cards dealt. Let's begin!{}", CL::DullTeal.get(), CL::End.get());

        // send out empty books to everyone and get this going
        let book_event = Update {
            spades: self.spades_book.clone(),
            clubs: self.clubs_book.clone(),
            diamonds: self.diamonds_book.clone(),
            hearts: self.hearts_book.clone(),
            trade: None,
        };

        let full_update = json!({
            "kind": "update",
            "data": book_event,
        });

        let mut removed_players = Vec::new();
        for (player_name, sender) in self.player_ws_map_hotpath.lock().await.iter_mut() {
            if let Err(_) = sender.send(Message::Text(full_update.to_string())).await {
                println!("{}[!] Error sending message to player | Deleting from the map. Player must resubscribe{}", CL::Red.get(), CL::End.get());
                removed_players.push(player_name.clone());
            }
        }

        for player_name in removed_players {
            self.player_ws_map_hotpath.lock().await.remove(&player_name);
        }

    }


}