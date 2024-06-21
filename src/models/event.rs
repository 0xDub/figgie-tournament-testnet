use super::{Card, CardBook, Inventory};
use serde::ser::{SerializeStruct, Serializer, SerializeMap};
use serde::Serialize;
use std::collections::HashMap;



#[derive(Debug, Clone)]
pub enum Event {
    Update(Update),
    DealCards(HashMap<String, Inventory>),
    EndRound,
}


#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    pub card: Card,
    pub price: usize,
    pub buyer: String,
    pub seller: String,
}


#[derive(Debug, Clone)]
pub struct Update {
    pub spades: CardBook,
    pub clubs: CardBook,
    pub diamonds: CardBook,
    pub hearts: CardBook,
    pub trade: Option<Trade>,
}

impl Serialize for Update {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Update", 4)?;
        state.serialize_field("clubs", &self.clubs)?;
        state.serialize_field("diamonds", &self.diamonds)?;
        state.serialize_field("hearts", &self.hearts)?;
        state.serialize_field("spades", &self.spades)?;
        
        if let Some(trade) = &self.trade {
            let trade_str = format!("{},{},{},{}", trade.card.to_string().to_lowercase(), trade.price, trade.buyer, trade.seller);
            state.serialize_field("trade", &trade_str)?;
        } else {
            state.serialize_field("trade", &String::new())?;
        }

        state.end()
    }
}



#[derive(Debug, Clone, Serialize)]
pub struct EndGamePointsUpdate {
    #[serde(serialize_with = "serialize_player_points")]
    pub player_points: HashMap<String, i32>,
}






#[derive(Debug, Clone, Serialize)]
pub struct EndRoundUpdate {
    #[serde(serialize_with = "serialize_card_count")]
    pub card_count: HashMap<Card, usize>,
    #[serde(serialize_with = "serialize_player_inventories")]
    pub player_inventories: HashMap<String, Inventory>,
    #[serde(serialize_with = "serialize_player_points")]
    pub player_points: HashMap<String, i32>,
    #[serde(serialize_with = "serialize_suite")]
    pub goal_suit: Card,
    #[serde(serialize_with = "serialize_suite")]
    pub common_suit: Card,
}

fn serialize_card_count<S>(card_count: &HashMap<Card, usize>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(card_count.len()))?;
    for (card, count) in card_count {
        map.serialize_entry(&format!("{}s", card.to_string().to_lowercase()), count)?;
    }
    map.end()
}

fn serialize_suite<S>(suite: &Card, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&suite.to_string().to_lowercase())
}


// =-= Player Inventories =-= //

#[derive(Debug, Clone, Serialize)]
pub struct PlayerInventory {
    pub player_name: String,
    pub spades: usize,
    pub clubs: usize,
    pub diamonds: usize,
    pub hearts: usize,
}

fn serialize_player_inventories<S>(player_inventories: &HashMap<String, Inventory>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{

    let player_inventories_vec: Vec<PlayerInventory> = player_inventories.iter().map(|(k, v)| PlayerInventory {
        player_name: k.clone(),
        spades: v.spades,
        clubs: v.clubs,
        diamonds: v.diamonds,
        hearts: v.hearts,
    }).collect();
    
    player_inventories_vec.serialize(serializer)
}


// =-= Player Points =-= //

#[derive(Debug, Clone, Serialize)]
pub struct PlayerPoints {
    pub player_name: String,
    pub points: i32,
}

fn serialize_player_points<S>(player_points: &HashMap<String, i32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    
    let player_points_vec: Vec<PlayerPoints> = player_points.iter().map(|(k, v)| PlayerPoints {
        player_name: k.clone(),
        points: *v,
    }).collect();
    
    player_points_vec.serialize(serializer)
}

