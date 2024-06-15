use actix_web::{post, web, App, HttpServer, HttpRequest, HttpResponse, Responder};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::net::TcpStream;
use futures_util::stream::StreamExt;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use tokio::sync::oneshot::{Sender as OneshotSender};
use kanal::AsyncSender;


mod utils;
pub use utils::*;

mod models;
pub use models::*;

mod matching_engine;
use matching_engine::MatchingEngine;


const STARTING_BALANCE: i32 = 500;


#[post("/admin")]
async fn admin_handler(
    req: HttpRequest,
    data: web::Json<AdminRequest>,
    started_game: web::Data<Arc<AtomicBool>>,
    matching_engine: web::Data<Arc<Mutex<MatchingEngine>>>,
    playerid_playername_map: web::Data<Arc<RwLock<HashMap<String, String>>>>,
    playername_rate_limit_map: web::Data<Arc<Mutex<HashMap<String, u8>>>>,
) -> impl Responder {

    println!("{}[+] ADMIN |:| Received POST request with admin details{}", CL::DimLightBlue.get(), CL::End.get());
    let round_duration: Duration = tokio::time::Duration::from_secs(60 * 2);
    let headers = req.headers();
    let num_of_rounds = 4;

    if let Some(admin_id) = headers.get("adminid") {
        let admin_id = admin_id.to_str().unwrap();
        match admin_id == "admin" { // changed in prod of course
            true => {
                println!("{}[+] ADMIN |:| Authentication passed{}", CL::Green.get(), CL::End.get());

                if data.action == "start_game" {
                    if started_game.load(Ordering::Acquire) {
                        println!("{}[!] ADMIN |:| Game already started{}", CL::Orange.get(), CL::End.get());
                    } else {
                        println!("{}[+] ADMIN |:| Starting game{}", CL::Green.get(), CL::End.get());
                        started_game.store(true, Ordering::Release);

                        let started_inside = Arc::clone(&started_game);
                        let match_maker_inside = Arc::clone(&matching_engine);
                        tokio::task::spawn_local(async move {
                            loop {
                                for i in 0..num_of_rounds {
                                    let start = tokio::time::Instant::now();
    
                                    started_inside.store(true, Ordering::Release);
                                    match_maker_inside.lock().await.start_round(i).await;
                                    while start.elapsed() < round_duration {
                                        sleep(Duration::from_secs(1)).await;
                                    }
    
                                    started_inside.store(false, Ordering::Release);
                                    match_maker_inside.lock().await.end_round().await;
                                }
    
                                match_maker_inside.lock().await.end_game().await;
                                println!("{}[+] ADMIN |:| Game has ended{}", CL::Green.get(), CL::End.get());
                                started_inside.store(false, Ordering::Release);

                                // Clear all players to keep the testnet lightweight
                                match_maker_inside.lock().await.delete_all_players();

                                playerid_playername_map.write().await.clear();
                                playername_rate_limit_map.lock().await.clear();

                                sleep(Duration::from_secs(15)).await;
                            }
                        });
                    }
                    return HttpResponse::Ok().body("Game started");
                }

                println!("{}[!] ADMIN |:| Invalid action: {}{}", CL::Red.get(), data.action, CL::End.get());
                return HttpResponse::Ok().body("Invalid action");
            },
            false => {
                println!("{}[!] ADMIN |:| Authentication Failed{}", CL::Red.get(), CL::End.get());
                return HttpResponse::Ok().body("Authentication Failed");
            }
        }
    } else {
        println!("{}[!] ADMIN |:| Admin ID not found{}", CL::Red.get(), CL::End.get());
        return HttpResponse::Ok().body("Admin ID not in headers");
    }
}



#[post("/order")]
async fn order_handler(
    req: HttpRequest,
    data: web::Json<RawOrderData>,
    started_game: web::Data<Arc<AtomicBool>>,
    sender_arc: web::Data<Arc<AsyncSender<(Order, OneshotSender<HTTPResponse>)>>>,
    playerid_playername_map: web::Data<Arc<RwLock<HashMap<String, String>>>>,
    playername_rate_limit_map: web::Data<Arc<Mutex<HashMap<String, u8>>>>,
) -> impl Responder {
    println!("{}[+] ORDER |:| Received new order from the API{}", CL::DimLightBlue.get(), CL::End.get());

    let rate_limit_per_second = 10; // rate limit is shared with /inventory and /order
    let headers = req.headers();

    // in this section of the code we need to filter out bad orders, get the headers and match it with the player name
    // if it's a valid order and the player name is found, then we check if the player name is within their allowed rolling rate limit allocation
    // if this all passes, we send it through the matching engine to be processed

    if !started_game.load(Ordering::Acquire) {
        let response = HTTPResponse { status: "NO_GAME".to_string(), message: "Game hasn't started yet. Sit tight and make sure your websocket connection is up and connected".to_string()};
        let serialized_response = serde_json::to_string(&response).unwrap();
        return HttpResponse::Ok().json(serialized_response);
    }

    if let Some(player_id) = headers.get("playerid") {
        let player_id = player_id.to_str().unwrap();
        let playerid_playername_map_guard = playerid_playername_map.read().await;
        if let Some(player_name) = playerid_playername_map_guard.get(player_id) {
            
            // I don't want to keep a lock on the rate limit map for too long, so let's get the data, clone it, then drop it
            let mut playername_rate_limit_map_guard = playername_rate_limit_map.lock().await;
            let mut outside_rate_limit: u8 = 0;
            if let Some(rate_limit) = playername_rate_limit_map_guard.get_mut(player_name) {
                *rate_limit += 1;
                outside_rate_limit = rate_limit.clone();
            } else {
                println!("{}[!] {:?} | Rate limit not found for playername{}", CL::Red.get(), player_name, CL::End.get());
                let response = HTTPResponse { status: "UNKNOWN_PLAYER".to_string(), message: "Player name not found. Have you sent a post to /register_testnet?".to_string()};
                let serialized_response = serde_json::to_string(&response).unwrap();
                return HttpResponse::Ok().json(serialized_response);
            }
            drop(playername_rate_limit_map_guard);




            match outside_rate_limit > rate_limit_per_second {
                true => {
                    let response = HTTPResponse { status: "RATE_LIMIT".to_string(), message: "Settle down there mate, you've reached >10 orders/second. Please wait 1 second till your limits are reset".to_string()};
                    let serialized_response = serde_json::to_string(&response).unwrap();
                    return HttpResponse::Ok().json(serialized_response);
                },
                false => {

                    let direction = match data.direction.as_str() {
                        "buy" => Direction::Buy,
                        "sell" => Direction::Sell,
                        _ => {
                            println!("{}[!] Invalid direction{}", CL::Red.get(), CL::End.get());
                            let response = HTTPResponse { status: "INVALID_DIRECTION".to_string(), message: "For the direction, please send either `buy` or `sell`".to_string()};
                            let serialized_response = serde_json::to_string(&response).unwrap();
                            return HttpResponse::Ok().json(serialized_response);
                        }
                    };

                    let card = match data.card.as_str() {
                        "spade" => Card::Spade,
                        "club" => Card::Club,
                        "diamond" => Card::Diamond,
                        "heart" => Card::Heart,
                        _ => {
                            println!("{}[!] Invalid card{}", CL::Red.get(), CL::End.get());
                            let response = HTTPResponse { status: "INVALID_CARD".to_string(), message: "For the card, please send either `spade`, `club`, `diamond`, or `heart`".to_string()};
                            let serialized_response = serde_json::to_string(&response).unwrap();
                            return HttpResponse::Ok().json(serialized_response);
                        }
                    };

                    if data.price <= 0 || data.price >= 100 {
                        println!("{}[!] Invalid price{}", CL::Red.get(), CL::End.get());
                        let response = HTTPResponse { status: "INVALID_PRICE".to_string(), message: "For the price, please send a number between 0 and 99".to_string()};
                        let serialized_response = serde_json::to_string(&response).unwrap();
                        return HttpResponse::Ok().json(serialized_response);
                    }

                    let order_data = Order {
                        card,
                        price: data.price,
                        direction,
                        player_name: player_name.to_string(),
                    };

                    let (oneshot_sender, receiver) = oneshot::channel();
                    sender_arc.send((order_data, oneshot_sender)).await.unwrap();

                    let response = receiver.await.unwrap();
                    let serialized_response = serde_json::to_string(&response).unwrap();
                    return HttpResponse::Ok().json(serialized_response);
                }
            }
        } else {
            println!("{}[!] Player name not found{}", CL::Orange.get(), CL::End.get());
            let response = HTTPResponse { status: "UNKNOWN_PLAYER".to_string(), message: "Player name not found. Have you sent a post to /register_testnet?".to_string()};
            let serialized_response = serde_json::to_string(&response).unwrap();
            return HttpResponse::Ok().json(serialized_response);
        }
    } else {
        println!("{}[!] Required headers not found, please send 'playerid' header with your request{}", CL::Orange.get(), CL::End.get());
        let response = HTTPResponse { status: "MISSING_HEADER".to_string(), message: "Required headers not found, please send 'playerid' header with your request. If this is for testnet, send anything. During the tournament you'll be given a unique ID that should be placed here".to_string()};
        let serialized_response = serde_json::to_string(&response).unwrap();
        return HttpResponse::Ok().json(serialized_response);
    }
}



#[post("/inventory")]
async fn inventory_handler(
    req: HttpRequest,
    started_game: web::Data<Arc<AtomicBool>>,
    matching_engine: web::Data<Arc<Mutex<MatchingEngine>>>,
    playerid_playername_map: web::Data<Arc<RwLock<HashMap<String, String>>>>,
    playername_rate_limit_map: web::Data<Arc<Mutex<HashMap<String, u8>>>>,
) -> impl Responder {
    let rate_limit_per_second = 10; // rate limit is shared with /inventory and /order
    let headers = req.headers();

    if !started_game.load(Ordering::Acquire) {
        println!("{}[!] Game hasn't started yet{}", CL::Red.get(), CL::End.get());
        let response = HTTPResponse { status: "NO_GAME".to_string(), message: "Game hasn't started yet. Sit tight and make sure your websocket connection is up and connected".to_string()};
        let serialized_response = serde_json::to_string(&response).unwrap();
        return HttpResponse::Ok().json(serialized_response);
    }

    if let Some(player_id) = headers.get("playerid") {
        let player_id = player_id.to_str().unwrap();
        let playerid_playername_map_guard = playerid_playername_map.read().await;
        if let Some(player_name) = playerid_playername_map_guard.get(player_id) {
            
            // I don't want to keep a lock on the rate limit map for too long, so let's get the data, clone it, then drop it (there's gotta be a better way to do this)
            let mut playername_rate_limit_map_guard = playername_rate_limit_map.lock().await;
            let mut outside_rate_limit: u8 = 0;
            if let Some(rate_limit) = playername_rate_limit_map_guard.get_mut(player_name) {
                *rate_limit += 1;
                outside_rate_limit = rate_limit.clone();
            } else {
                println!("{}[!] {:?} | Rate limit not found for playername{}", CL::Red.get(), player_name, CL::End.get());
                let response = HTTPResponse { status: "UNKNOWN_PLAYER".to_string(), message: "Player name not found. Have you sent a post to /register_testnet?".to_string()};
                let serialized_response = serde_json::to_string(&response).unwrap();
                return HttpResponse::Ok().json(serialized_response);
            }
            drop(playername_rate_limit_map_guard);


            match outside_rate_limit > rate_limit_per_second {
                true => {
                    let response = HTTPResponse { status: "RATE_LIMIT".to_string(), message: "Settle down there mate. The inventory rate limit is 1x / 5 seconds".to_string()};
                    let serialized_response = serde_json::to_string(&response).unwrap();
                    return HttpResponse::Ok().json(serialized_response);
                },
                false => {
                    let inventory = matching_engine.lock().await.get_player_inventory(player_name);
                    let response = HTTPResponse { status: "SUCCESS".to_string(), message: format!("{},{},{},{}", inventory.spades, inventory.clubs, inventory.diamonds, inventory.hearts) };
                    let serialized_response = serde_json::to_string(&response).unwrap();
                    return HttpResponse::Ok().json(serialized_response);
                }
            }
        } else {
            println!("{}[!] Player name not found{}", CL::Orange.get(), CL::End.get());
            let response = HTTPResponse { status: "UNKNOWN_PLAYER".to_string(), message: "Player name not found. Have you sent a post to /register_testnet?".to_string()};
            let serialized_response = serde_json::to_string(&response).unwrap();
            return HttpResponse::Ok().json(serialized_response);
        }
    } else {
        println!("{}[!] Required headers not found, please send 'playerid' header with your request{}", CL::Orange.get(), CL::End.get());
        let response = HTTPResponse { status: "MISSING_HEADER".to_string(), message: "Required headers not found, please send 'playerid' header with your request".to_string()};
        let serialized_response = serde_json::to_string(&response).unwrap();
        return HttpResponse::Ok().json(serialized_response);
    }

}

fn generate_random_player_name() -> String {
    let first_word = random_word::gen_len(5, random_word::Lang::En).unwrap();
    let first_word = format!("{}{}", first_word.chars().next().unwrap().to_uppercase().collect::<String>(), &first_word[1..]);

    let second_word = random_word::gen_len(5, random_word::Lang::En).unwrap();
    let second_word = format!("{}{}", second_word.chars().next().unwrap().to_uppercase().collect::<String>(), &second_word[1..]);

    let player_name = format!("{}{}", first_word, second_word);
    
    return player_name;
}

#[post("/register_testnet")]
async fn register_testnet_handler(
    req: HttpRequest,
    matching_engine: web::Data<Arc<Mutex<MatchingEngine>>>,
    playerid_playername_map: web::Data<Arc<RwLock<HashMap<String, String>>>>,
    playername_rate_limit_map: web::Data<Arc<Mutex<HashMap<String, u8>>>>,
) -> impl Responder {
    let headers = req.headers();
    // get their supplied playerid and then generate a random player_name (String format),
    // then add that player_name and playerid to the maps
    // return that name to them while echoing back their playerid

    if let Some(player_id) = headers.get("playerid") {
        let player_id = player_id.to_str().unwrap().to_owned();

        // if player_id is already present, return the player_name
        let mut playerid_playername_map_guard = playerid_playername_map.write().await; // write lock since we'll be adding a player later if needed, on the return it'll drop the lock automatically
        if let Some(player_name) = playerid_playername_map_guard.get(&player_id) {
            let response = HTTPResponse { status: "SUCCESS".to_string(), message: format!("Temp player name: {}. Testnet will always send out 3 cards of each suit to test with", player_name) };
            let serialized_response = serde_json::to_string(&response).unwrap();
            return HttpResponse::Ok().json(serialized_response);
        }

        let player_name = generate_random_player_name();

        playerid_playername_map_guard.insert(player_id.clone(), player_name.clone());
        drop(playerid_playername_map_guard);

        let mut playername_rate_limit_map_guard = playername_rate_limit_map.lock().await;
        playername_rate_limit_map_guard.insert(player_name.clone(), 0);
        drop(playername_rate_limit_map_guard);

        matching_engine.lock().await.add_new_player_with_inventory(player_name.clone(), Inventory { spades: 3, clubs: 3, diamonds: 3, hearts: 3 });
        matching_engine.lock().await.print_all_players();

        let response = HTTPResponse { status: "SUCCESS".to_string(), message: format!("Temp player name: {}. Testnet will always send out 3 cards of each suit to test with", player_name) };
        let serialized_response = serde_json::to_string(&response).unwrap();
        return HttpResponse::Ok().json(serialized_response);

    } else {
        println!("{}[!] Required headers not found, please send 'playerid' header with your request{}", CL::Orange.get(), CL::End.get());
        let response = HTTPResponse { status: "MISSING_HEADER".to_string(), message: "Required headers not found. Please send 'playerid' in your Headers with a random ID. We'll register this playerid into the testnet and send you back a temporary PlayerName".to_string()};
        let serialized_response = serde_json::to_string(&response).unwrap();
        return HttpResponse::Ok().json(serialized_response);
    }
}



fn main() {
    println!("=-= Starting Figgie Testnet Exchange =-=");

    // =-------------------------------------------------------------------------------------------------------= //

    let playerid_playername_map: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new())); // playerid -> playername
    let playername_rate_limit_map: Arc<Mutex<HashMap<String, u8>>> = Arc::new(Mutex::new(HashMap::new())); // playername -> rate_limit


    let player_ws_map: Arc<Mutex<HashMap<String, SplitSink<WebSocketStream<TcpStream>, Message>>>> = Arc::new(Mutex::new(HashMap::new()));
    let player_ws_map_hotpath = Arc::clone(&player_ws_map);


    let started: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let matching_engine: Arc<Mutex<MatchingEngine>> = Arc::new(Mutex::new(MatchingEngine::new(STARTING_BALANCE, player_ws_map_hotpath)));
    let matching_engine_hotpath = Arc::clone(&matching_engine);


    let (sender, receiver) = kanal::unbounded_async::<(Order, OneshotSender<HTTPResponse>)>();
    let sender_arc = Arc::new(sender);


    // =-------------------------------------------------------------------------------------------------------= //


    let network_thread = std::thread::Builder::new()
        .spawn(move || {
        let res = core_affinity::set_for_current(core_affinity::CoreId { id: 0 });
        if res {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("build runtime");
            rt.block_on(async {
                let mut handles = Vec::new();

                // =-= Rate Limit Monitoring =-= //
                let playername_rate_limit_map_monitoring = Arc::clone(&playername_rate_limit_map);
                let rate_limit_monitoring = tokio::task::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        let mut playername_rate_limit_map_guard = playername_rate_limit_map_monitoring.lock().await;
                        for (_, rate_limit) in playername_rate_limit_map_guard.iter_mut() {
                            *rate_limit = 0;
                        }
                    }
                });
                handles.push(rate_limit_monitoring);


                // =-= REST API =-= //
                let player_password_map_rest = Arc::clone(&playerid_playername_map);
                let rest_api = tokio::task::spawn(async move {
                    if let Err(e) = HttpServer::new(move || {
                        App::new()
                            .app_data(web::Data::new(Arc::clone(&player_password_map_rest)))
                            .app_data(web::Data::new(Arc::clone(&playername_rate_limit_map)))
                            .app_data(web::Data::new(Arc::clone(&started)))
                            .app_data(web::Data::new(Arc::clone(&matching_engine)))
                            .app_data(web::Data::new(Arc::clone(&sender_arc)))
                            .service(order_handler)
                            .service(inventory_handler)
                            .service(admin_handler)
                            .service(register_testnet_handler)
                    })
                    .bind(("127.0.0.1", 8080)).expect("[!] Failed to bind the address")
                    .run()
                    .await {
                        println!("[!] Error with the REST API server: {:?}", e);
                    }
                });
                handles.push(rest_api);
                
                
                // =-= Websocket Server =-= //
                let websocket = tokio::task::spawn(async move {
                    if let Ok(listener) = TcpListener::bind(&"127.0.0.1:8090").await {

                        while let Ok((stream, addr)) = listener.accept().await {
                            let player_ws_map_network_inside = Arc::clone(&player_ws_map);
                            let playerid_playername_map_websocket = Arc::clone(&playerid_playername_map);
                            tokio::spawn(async move {
                                println!("[+] Incoming TCP connection from: {:?}", addr);

                                match tokio_tungstenite::accept_async(stream).await {
                                    Ok(ws_stream) => {
                                        println!("{}[+] WebSocket connection established: {:?}{}", CL::Green.get(), addr, CL::End.get());

                                        let (mut sender, mut receiver) = ws_stream.split();
                                        while let Some(msg) = receiver.next().await {
                                            if let Ok(msg) = msg {
                                                println!("{}[-] Received a message: {:?}{}", CL::Dull.get(), msg, CL::End.get());
                                    
                                                match msg {
                                                    Message::Text(text) => {
                                                        if let Ok(message) = serde_json::from_str::<SubscribeMessage>(&text) {
                                                            if message.action == "subscribe" {
                                                                println!("{}[-] Attempting to subscribe to the exchange{}", CL::Dull.get(), CL::End.get());
                                        
                                                                match playerid_playername_map_websocket.read().await.get(&message.playerid) {
                                                                    Some(player_name) => {

                                                                        // =-= SUCCESS =-= //
                                                                        println!("{}[+] Successfully subscribed to the stream: {:?}{}", CL::DullTeal.get(), player_name, CL::End.get());
                                                                        let welcome_message = Message::Text(serde_json::to_string(&HTTPResponse {
                                                                            status: "SUCCESS".to_string(),
                                                                            message: format!("Welcome to the tesetnet, {}! You've been subscribed for further data updates", player_name)
                                                                        }).unwrap());
                                                                        sender.send(welcome_message).await.unwrap();
                                        

                                                                        player_ws_map_network_inside.lock().await.insert(player_name.clone(), sender);
                                                                        break;

                                                                    },
                                                                    None => {

                                                                        // =-= ACCOUNT_NOT_FOUND =-= //
                                                                        println!("{}[!] Account not found for the password given: {}{}", CL::Orange.get(), message.playerid, CL::End.get());
                                                                        let response_message = Message::Text(serde_json::to_string(&HTTPResponse {
                                                                            status: "UNKNOWN_PLAYER".to_string(),
                                                                            message: "Player name not found. Have you sent a post to /register_testnet?".to_string()
                                                                        }).unwrap());
                                                                        sender.send(response_message).await.unwrap();

                                                                    }
                                                                }
                                                            } else {

                                                                // =-= UNAUTHORIZED_ACTION =-= //
                                                                println!("{}[!] Unrecognized action: {:?} | Please send 'subscribe' with 'playerid'{}", CL::Orange.get(), message.action, CL::End.get());
                                                                let response_message = Message::Text(serde_json::to_string(&HTTPResponse {
                                                                    status: "UNAUTHORIZED_ACTION".to_string(),
                                                                    message: "Unauthorized action, please send 'subscribe' as the action".to_string()
                                                                }).unwrap());
                                                                sender.send(response_message).await.unwrap();

                                                            }
                                                        } else {

                                                            // =-= PARSE_ERROR =-= //
                                                            println!("{}[!] Failed to parse the WS message{}", CL::Orange.get(), CL::End.get());
                                                            let response_message = Message::Text(serde_json::to_string(&HTTPResponse {
                                                                status: "PARSE_ERROR".to_string(),
                                                                message: "Failed to parse the message. Please send a JSON message with fields 'subscribe' and 'playerid' that match up with your PlayerName (in the testnet, send a random playerid)".to_string()
                                                            }).unwrap());
                                                            sender.send(response_message).await.unwrap();

                                                        }
                                                    },
                                                    Message::Close(_) => {
                                                        println!("{}[!] Connection has been closed{}", CL::DullRed.get(), CL::End.get());
                                                        // cleanup is handled in the matching_engine
                                                        break;
                                                    },
                                                    _ => {}
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        println!("{}[!] Error accepting WS connection{}", CL::Red.get(), CL::End.get());
                                    }
                                }
                            });
                        }
                    }
                });
                handles.push(websocket);


                for handle in handles {
                    handle.await.unwrap();
                }
            
            });
        }
    }).unwrap();

    let hotpath_thread = std::thread::Builder::new()
        .spawn(move || {
        let res = core_affinity::set_for_current(core_affinity::CoreId { id: 1 });
        if res {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .worker_threads(1)
                .build()
                .expect("build runtime");
            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, async {
                loop {
                    match receiver.recv().await {
                        Ok((order_data, response_sender)) => {
                        
                            let response = matching_engine_hotpath.lock().await.process_order(order_data).await;
                            if let Err(e) = response_sender.send(response) {
                                println!("{}[!] Failed to send the response back to the RestAPI: {:?}{}", CL::Red.get(), e, CL::End.get()); // how to handle this? assume that the HTTP Connection was dropped?
                            }

                        },
                        Err(e) => {
                            //println!("{}[!] Failed to receive a response from the matching engine: {:?}{}", CL::Red.get(), e, CL::End.get());
                        }
                    }
                }
            });
        }
    }).unwrap();



    network_thread.join().unwrap();
    hotpath_thread.join().unwrap();

}