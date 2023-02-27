#[macro_use] extern crate rocket;
use rocket::{get, http::Status, serde::json::Json};
use serde::{Serialize, Deserialize};

use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;

use bdk::{
    Wallet,
    bitcoin::{
        Network,
        Address,
    },
    database::MemoryDatabase,
    wallet::AddressIndex,
    blockchain::{
        ElectrumBlockchain,
        Blockchain,
    },
    electrum_client::Client,
    SyncOptions,
    SignOptions,
    TransactionDetails,
};

use std::str::FromStr;

fn create_wallet() -> Wallet<MemoryDatabase>{
    let external_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/0/*)";
    let internal_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/1/*)";

    let wallet = Wallet::new(
        external_descriptor,
        Some(internal_descriptor),
        Network::Testnet,
        MemoryDatabase::new(),
    ).unwrap();

    return wallet;
}

#[derive(Serialize, Deserialize)]
struct Withdraw {
    amount: u64,
    address: String,
}

#[get("/address")]
fn address() -> String {
    let wallet = create_wallet();
    return wallet.get_address(AddressIndex::Peek(10)).unwrap().to_string();
}

// #[get("/balance")]
#[post("/withdraw", format = "json", data = "<withdraw>")]
fn withdraw(withdraw: Json<Withdraw>) -> Result<Json<TransactionDetails>, Status> {
    let wallet = create_wallet();

    let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
    let blockchain = ElectrumBlockchain::from(client);
    wallet.sync(&blockchain, SyncOptions::default()).unwrap();

    // mv4rnyY3Su5gjcDNzbMLKBQkBicCtHUtFB
    let faucet_address = Address::from_str(&withdraw.address).unwrap();

    let mut tx_builder = wallet.build_tx();
        
    tx_builder
        .add_recipient(faucet_address.script_pubkey(), withdraw.amount)
        .enable_rbf();
        
    let (mut psbt, tx_details) = tx_builder.finish().unwrap();
    
    println!("Transaction details: {:#?}", tx_details);

    let finalized = wallet.sign(&mut psbt, SignOptions::default()).unwrap();
    assert!(finalized, "Tx has not been finalized");

    let raw_transaction = psbt.extract_tx();

    let txid = raw_transaction.txid();
    blockchain.broadcast(&raw_transaction).unwrap();

    println!(
        "Transaction sent! TXID: {txid}.\nExplorer URL: https://blockstream.info/testnet/tx/{txid}",
        txid = txid
    );
    
    Ok(Json(tx_details))
}

#[get("/")]
fn index() -> &'static str {
    "Hello, Rust!"
}

struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "CORS header",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "http://engine:8080"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "Content-Type"));
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().attach(Cors).mount("/", routes![index, address, withdraw])
}