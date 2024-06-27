use serde::ser::{SerializeStruct, Serializer, SerializeSeq};
use serde::Serialize;


// =-= Serialziation =-= //

#[derive(Debug, Clone, Serialize)]
pub struct BookEntryStringified {
    pub player_name: String,
    pub price: String,
}

impl Serialize for CardBook {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CardBook", 2)?;

        // changed the serialization method to be more akin to how a crypto exchange does it, should be more client-friendly this way

        // legacy serialization method with dicts
        //let mut stringified_book = BookEntryStringified { player_name: "".to_string(), price: "".to_string() };
        //if let (Some(price), Some(player_name)) = self.get_best_ask() {
        //    stringified_book = BookEntryStringified {
        //        player_name: player_name.to_string(),
        //        price: price.to_string(),
        //    };
        //}

        // serialize it into a Vec<BookEntrySerialized> = Vec<(integer, String)>
        // but only serialize the first element (unless people want the full book published)
        //let bbo_bid = self.bids.first().map(|x| (x.price, x.player_name.clone())).into_iter().collect::<Vec<_>>();
        
        let bids = self.bids.iter().map(|x| (x.price, x.player_name.clone())).into_iter().collect::<Vec<_>>();
        state.serialize_field("bids", &bids)?;


    
        // legacy serialization method with dicts
        //let mut stringified_book = BookEntryStringified { player_name: "".to_string(), price: "".to_string() };
        //if let (Some(price), Some(player_name)) = self.get_best_bid() {
        //    stringified_book = BookEntryStringified {
        //        player_name: player_name.to_string(),
        //        price: price.to_string(),
        //    };
        //}
        //let bbo_ask = self.asks.first().map(|x| (x.price, x.player_name.clone())).into_iter().collect::<Vec<_>>();

        let asks: Vec<(usize, String)> = self.asks.iter().map(|x| (x.price, x.player_name.clone())).into_iter().collect::<Vec<_>>();
        state.serialize_field("asks", &asks)?;
        
        
        state.serialize_field(
            "last_trade",
            &self.last_trade.map(|x| x.to_string()).unwrap_or_else(|| "".to_string()),
        )?;
        state.end()
    }
}

impl Serialize for BookEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.player_name)?;
        seq.serialize_element(&self.price)?;
        seq.end()
    }
}



// =-= Base Models =-= //

#[derive(Debug, Clone)]
pub struct BookEntry {
    pub price: usize,
    pub player_name: String,
}

#[derive(Debug, Clone)]
pub struct CardBook {
    pub bids: Vec<BookEntry>,
    pub asks: Vec<BookEntry>,
    pub last_trade: Option<usize>,
}


impl CardBook {
    pub fn new() -> Self {
        Self {
            bids: Vec::with_capacity(20),
            asks: Vec::with_capacity(20),
            last_trade: None,
        }
    }

    pub fn cancel_bid(&mut self, player_name: String) {
        self.bids.retain(|bid| bid.player_name != player_name);
    }

    pub fn cancel_ask(&mut self, player_name: String) {
        self.asks.retain(|ask| ask.player_name != player_name);
    }

    pub fn update_bid(&mut self, price: usize, player_name: String) {

        let mut found = false;
        for bid in self.bids.iter_mut() {
            if bid.player_name == player_name {
                bid.price = price;
                found = true;
                break;
            }
        }
        
        if !found { // if the player_name is not found, add a new bid
            self.bids.push(BookEntry { price, player_name });
        }
        
        self.bids.sort_by(|a, b| b.price.cmp(&a.price)); // sort the bids in descending order
    }

    pub fn update_ask(&mut self, price: usize, player_name: String) {
        
        let mut found = false;
        for ask in self.asks.iter_mut() {
            if ask.player_name == player_name {
                ask.price = price;
                found = true;
                break;
            }
        }
        
        if !found { // if the player_name is not found, add a new ask
            self.asks.push(BookEntry { price, player_name });
        }

        self.asks.sort_by(|a, b| a.price.cmp(&b.price)); // sort the asks in ascending order
    }

    pub fn get_best_bid(&self) -> (Option<usize>, Option<String>) {
        if let Some(bid) = self.bids.first() {
            (Some(bid.price), Some(bid.player_name.clone()))
        } else {
            (None, None)
        }
    }

    pub fn get_best_ask(&self) -> (Option<usize>, Option<String>) {
        if let Some(ask) = self.asks.first() {
            (Some(ask.price), Some(ask.player_name.clone()))
        } else {
            (None, None)
        }
    }

    pub fn reset_quotes(&mut self) {
        self.bids = Vec::new();
        self.asks = Vec::new();
    }

    pub fn reset_full_book(&mut self) {
        self.bids = Vec::new();
        self.asks = Vec::new();
        self.last_trade = None;
    }

}