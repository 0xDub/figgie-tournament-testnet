use serde::{Deserialize, Serialize};


// =-= Responses =-= //

#[derive(Deserialize, Serialize, Debug)]
pub struct SubscribeMessage {
    pub action: String,
    pub playerid: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HTTPResponse {
    pub status: String,
    pub message: String,
}

// =-= RestAPI =-= //

#[derive(Deserialize, Serialize, Debug)]
pub struct RawOrderData {
    pub card: String,
    pub price: usize,
    pub direction: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AdminRequest {
    pub action: String,
    pub players: String,
}