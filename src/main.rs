mod rpc;

use warp::Filter;
use serde_json::json;
use rpc::{RpcRequest, RpcResponse, RpcError, ErrorObject};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use clap::{App, Arg, SubCommand};
use reqwest::Client;
use std::time::{Instant, Duration};
use tokio::time::{interval, sleep_until};
use std::thread::sleep;

#[derive(Serialize, Deserialize, Debug)]
struct CreateAccountPayload {
    id: String,
    balance: String,
}

#[tokio::main]
async fn main() {
    let matches = App::new("Mock Blockchain Node CLI")
        .version("1.0")
        .author("Jasper Li")
        .about("cli")
        .subcommand(SubCommand::with_name("start-node")
            .about("Starts the local node"))
        .subcommand(SubCommand::with_name("create-account")
            .about("Create account")
            .arg(Arg::with_name("ID")
                .help("The id of the account")
                .required(true)
                .index(1))
            .arg(Arg::with_name("BALANCE")
                .help("The starting balance")
                .required(true)
                .index(2)))
        .subcommand(SubCommand::with_name("balance")
            .about("Get balance")
            .arg(Arg::with_name("ID")
                .help("The id of the account")
                .required(true)
                .index(1)))
        .subcommand(SubCommand::with_name("transfer")
            .about("Transfer")
            .arg(Arg::with_name("FROM")
                .help("Transfer from")
                .required(true)
                .index(1))
            .arg(Arg::with_name("TO")
                .help("Transfer to")
                .required(true)
                .index(2))
            .arg(Arg::with_name("AMOUNT")
                .help("Amount to transfer")
                .required(true)
                .index(3)))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("start-node") {
        start_node().await;
    }

    if let Some(matches) = matches.subcommand_matches("create-account") {
        let id = matches.value_of("ID").unwrap();
        let balance = matches.value_of("BALANCE").unwrap();
        create_account(id, balance).await;
    }

    if let Some(matches) = matches.subcommand_matches("balance") {
        let id = matches.value_of("ID").unwrap();
        get_balance(id).await;
    }

    if let Some(matches) = matches.subcommand_matches("transfer") {
        let from = matches.value_of("FROM").unwrap();
        let to = matches.value_of("TO").unwrap();
        let amount = matches.value_of("AMOUNT").unwrap();
        transfer(from, to, amount).await;
    }
}

async fn start_node() {
    // Create a shared account map wrapped in Arc and Mutex
    let accounts = Arc::new(Mutex::new(HashMap::new()));

    // Clone the accounts map for the warp filter
    let accounts_filter = warp::any().map(move || Arc::clone(&accounts));

    let genesis_time = Instant::now();
    let genesis_time_filter = warp::any().map(move || genesis_time);

    // Create a new interval that ticks every 10 seconds
    let mut interval = interval(Duration::from_secs(10));

    // Spawn a new task that prints a message every 10 seconds
    tokio::spawn(async move {
        let mut block_count = 0;
        loop {
            interval.tick().await;
            block_count += 1;
            println!("Block {} mined", block_count);
        }
    });

    let rpc = warp::path::end()
        .and(warp::post())
        .and(warp::body::json())
        .and(accounts_filter)
        .and(genesis_time_filter)
        .map(handle_rpc);

    println!("Node running on http://127.0.0.1:3030");
    
    warp::serve(rpc).run(([127, 0, 0, 1], 3030)).await;
}

fn handle_rpc(
    req: RpcRequest, 
    accounts: Arc<Mutex<HashMap<String, String>>>, 
    genesis_time: Instant
) -> warp::reply::Json {
    let response = match req.method.as_str() {
        "create_account" => {
            let params = req.params.clone();
            let accounts = Arc::clone(&accounts);
            let id = req.id.clone();
            if let Some(params) = params.as_object() {
                if let (Some(id), Some(balance)) = (params.get("id").and_then(|v| v.as_str()), params.get("balance").and_then(|v| v.as_str())) {
                    let mut accounts = accounts.lock().unwrap();
                    accounts.insert(id.to_string(), balance.to_string());

                    let now = Instant::now();
                    let elapsed = now.duration_since(genesis_time);
                    let next = ((elapsed.as_secs() / 10) + 1) * 10;
                    let sleep_time = next - elapsed.as_secs();
                    
                    println!("T={}. Waiting to be confirmed...", elapsed.as_secs());
                    sleep(Duration::from_secs(sleep_time));

                    warp::reply::json(&RpcResponse {
                        jsonrpc: "2.0".into(),
                        result: json!(format!("Account created: id={}, balance={}", id, balance)),
                        id: id.into(),
                    })
                } else {
                    warp::reply::json(&make_error(-32602, "Invalid params".into(), id))
                }
            } else {
                warp::reply::json(&make_error(-32602, "Invalid params".into(), id))
            }
        }
        "balance" => {
            let params = req.params.clone();
            let accounts = Arc::clone(&accounts);
            let id = req.id.clone();
            if let Some(params) = params.as_object() {
                if let Some(id) = params.get("id").and_then(|v| v.as_str()) {
                    if let Some(balance) = accounts.lock().unwrap().get(id) {
                        warp::reply::json(&RpcResponse {
                            jsonrpc: "2.0".into(),
                            result: json!(format!("{}", balance)),
                            id: id.into(),
                        })
                    } else {
                        warp::reply::json(&RpcResponse {
                            jsonrpc: "2.0".into(),
                            result: json!(format!("{}", 0)),
                            id: id.into(),
                        })
                    }
                } else {
                    warp::reply::json(&make_error(-32602, "Invalid params".into(), id))
                }
            } else {
                warp::reply::json(&make_error(-32602, "Invalid params".into(), id))
            }
        }
        "transfer" => {
            let params = req.params.clone();
            let accounts = Arc::clone(&accounts);
            let id = req.id.clone();
            if let Some(params) = params.as_object() {
                if let (
                    Some(from), 
                    Some(to),
                    Some(amount)
                ) = (
                    params.get("from").and_then(|v| v.as_str()), 
                    params.get("to").and_then(|v| v.as_str()),
                    params.get("amount").and_then(|v| v.as_str()),
                ) {
                    let amount_u64 = match amount.parse::<u64>() {
                        Ok(value) => value,
                        Err(_) => return warp::reply::json(&make_error(-32602, "Invalid amount".into(), id)),
                    };
                
                    let mut accounts = accounts.lock().unwrap();
                
                    if !accounts.contains_key(from) {
                        return warp::reply::json(&make_error(-32603, "Account from does not exist".into(), id));
                    }
                    if !accounts.contains_key(to) {
                        return warp::reply::json(&make_error(-32603, "Account to does not exist".into(), id));
                    }
                
                    {
                        let balance_from_str = accounts.get_mut(from).unwrap();
                        let balance_from = match balance_from_str.parse::<u64>() {
                            Ok(value) => value,
                            Err(_) => return warp::reply::json(&make_error(-32602, "Invalid balance for account from".into(), id)),
                        };

                        if balance_from < amount_u64 {
                            return warp::reply::json(&make_error(-32603, "Insufficient funds".into(), id));
                        }

                        let new_balance_from = balance_from - amount_u64;
                        *balance_from_str = new_balance_from.to_string();
                    }

                    {
                        let balance_to_str = accounts.get_mut(to).unwrap();
                        let balance_to = match balance_to_str.parse::<u64>() {
                            Ok(value) => value,
                            Err(_) => return warp::reply::json(&make_error(-32602, "Invalid balance for account to".into(), id)),
                        };

                        let new_balance_to = balance_to + amount_u64;
                        *balance_to_str = new_balance_to.to_string();
                    }

                    let now = Instant::now();
                    let elapsed = now.duration_since(genesis_time);
                    let next = ((elapsed.as_secs() / 10) + 1) * 10;
                    let sleep_time = next - elapsed.as_secs();
                    
                    println!("T={}. Waiting to be confirmed...", elapsed.as_secs());
                    sleep(Duration::from_secs(sleep_time));
                
                    warp::reply::json(&RpcResponse {
                        jsonrpc: "2.0".into(),
                        result: json!(format!("Transfer successful: {} to {} amount {}", from, to, amount)),
                        id: id,
                    })
                } else {
                    warp::reply::json(&make_error(-32602, "Invalid params".into(), id))
                }
            } else {
                warp::reply::json(&make_error(-32602, "Invalid params".into(), id))
            }
        }
        _ => warp::reply::json(&make_error(-32601, "Method not found".into(), req.id)),
    };

    response
}

fn make_error(code: i32, message: String, id: serde_json::Value) -> RpcError {
    RpcError {
        jsonrpc: "2.0".into(),
        error: ErrorObject {
            code,
            message,
            data: None,
        },
        id,
    }
}

async fn create_account(id: &str, balance: &str) {
    let client = Client::new();
    let request = json!({
        "jsonrpc": "2.0",
        "method": "create_account",
        "params": {
            "id": id,
            "balance": balance
        },
        "id": 1
    });

    match client.post("http://127.0.0.1:3030")
        .json(&request)
        .send()
        .await {
        Ok(response) => {
            if response.status().is_success() {
                let resp_json: serde_json::Value = response.json().await.unwrap();
                println!("Response: {}", resp_json);
            } else {
                println!("Failed to send request: {}", response.status());
            }
        },
        Err(e) => println!("Error sending request: {}", e),
    }
}

async fn get_balance(id: &str) {
    let client = Client::new();
    let request = json!({
        "jsonrpc": "2.0",
        "method": "balance",
        "params": {
            "id": id
        },
        "id": 1
    });

    match client.post("http://127.0.0.1:3030")
        .json(&request)
        .send()
        .await {
        Ok(response) => {
            if response.status().is_success() {
                let resp_json: serde_json::Value = response.json().await.unwrap();
                println!("Response: {}", resp_json);
            } else {
                println!("Failed to send request: {}", response.status());
            }
        },
        Err(e) => println!("Error sending request: {}", e),
    }
}

async fn transfer(from: &str, to: &str, amount: &str) {
    let client = Client::new();
    let request = json!({
        "jsonrpc": "2.0",
        "method": "transfer",
        "params": {
            "from": from,
            "to": to,
            "amount": amount
        },
        "id": 1
    });

    match client.post("http://127.0.0.1:3030")
        .json(&request)
        .send()
        .await {
        Ok(response) => {
            if response.status().is_success() {
                let resp_json: serde_json::Value = response.json().await.unwrap();
                println!("Response: {}", resp_json);
            } else {
                println!("Failed to send request: {}", response.status());
            }
        },
        Err(e) => println!("Error sending request: {}", e),
    }
}