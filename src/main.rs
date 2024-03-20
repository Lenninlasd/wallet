mod wallet;

#[macro_use] extern crate rocket;
use rocket::{get, http::Status, serde::json::Json};
use serde::{Serialize, Deserialize};
use rocket::{Request, Response};
use rocket::response::status::Custom;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use sha256::digest;

use bdk::{
    wallet::get_funded_wallet,
    Balance,
    bitcoin::Address,
    wallet::AddressIndex,
    blockchain::Blockchain,
    SignOptions,
    TransactionDetails,
    FeeRate,
    Error,
};

use std::{
    str::FromStr,
    env,
};

use wallet::{
    create_wallet,
    sync_wallet,
    generate_mnemonic,
    extended_pub_key,
};

fn get_test_wpkh() -> &'static str {
    "wpkh(cVpPVruEDdmutPzisEsYvtST1usBR3ntr8pXSyt6D2YYqXRyPcFW)"
}

fn check_integrity(withdraw: &Json<Withdraw>) -> bool {
    let integrity = env::var("WITHDRAWAL_INTEGRITY").unwrap_or_else(|_| String::new());
    let str_token = format!("{}{}{}{}{}", withdraw.reference, withdraw.fee_rate, withdraw.amount, withdraw.address, integrity);
    let checksum = digest(str_token);
    return checksum == withdraw.checksum;
}

#[derive(Serialize, Deserialize)]
struct Withdraw {
    reference: String,
    amount: u64,
    address: String,
    fee_rate: f32,
    checksum: String,
}

#[get("/address")]
fn address() -> String {
    let wallet = create_wallet().unwrap();
    return wallet.get_address(AddressIndex::Peek(10)).unwrap().to_string();
}

#[get("/rand_mnemonic")]
fn mnemonic() -> String {
    return generate_mnemonic();
}

#[get("/extended")]
fn extended() -> String {
    return extended_pub_key();
}

#[derive(Serialize)]
pub struct GenericResponse {
    pub status: String,
    pub message: String,
    pub error: String,
}

#[get("/balance")]
fn balance() -> Result<Json<Balance>, Status> {
    let wallet = create_wallet().unwrap();
    sync_wallet(&wallet);

    let balance = wallet.get_balance().unwrap();
    println!("Wallet balance in SAT: {}", balance);
    
    Ok(Json(balance))
}

#[get("/transactiondetails?<address>&<amount>&<fee_rate>")]
fn transaction_details(address: String, amount: u64, fee_rate: f32) -> Result<Json<TransactionDetails>, Status> {    
    let (wallet, _, _) = get_funded_wallet(get_test_wpkh());

    let faucet_address = Address::from_str(&address).unwrap();

    let mut tx_builder = wallet.build_tx();
    
    tx_builder
        .add_recipient(faucet_address.script_pubkey(), amount)
        .fee_rate(FeeRate::from_sat_per_vb(fee_rate))
        .enable_rbf();

    let ( _, tx_details) = tx_builder.finish().unwrap();

    Ok(Json(tx_details))
}

#[post("/withdraw", format = "json", data = "<withdraw>")]
fn withdraw(withdraw: Json<Withdraw>) -> Result<Json<TransactionDetails>, Custom<Json<GenericResponse>>> {
    let is_checksum_valid = check_integrity(&withdraw);
    if !is_checksum_valid {
        let error_response = GenericResponse {
            status: "fail".to_string(),
            message: "Wrong checksum".to_string(),
            error: "WrongChecksum".to_string(),
        };
        return Err(Custom(Status::BadRequest, Json(error_response)));
    }

    let wallet = create_wallet().unwrap();
    let  blockchain = sync_wallet(&wallet);

    // mv4rnyY3Su5gjcDNzbMLKBQkBicCtHUtFB
    let faucet_address = Address::from_str(&withdraw.address).unwrap();

    let mut tx_builder = wallet.build_tx();
    
    tx_builder
        .add_recipient(faucet_address.script_pubkey(), withdraw.amount)
        .fee_rate(FeeRate::from_sat_per_vb(withdraw.fee_rate))
        .enable_rbf();

    let (mut psbt, tx_details) = match tx_builder.finish() {
        Ok(tx_builder_result) => {
            tx_builder_result
        }
        Err(error) => {
            let error_response = match error {
                Error::InsufficientFunds { needed: _, available: _ } => {
                    GenericResponse {
                        status: "fail".to_string(),
                        message: format!("{}",error),
                        error: "InsufficientFunds".to_string(),
                    }
                }
                rest_error => {
                    GenericResponse {
                        status: "fail".to_string(),
                        message: rest_error.to_string(),
                        error: "Generic".to_string(),
                    }
                }
            };

            return Err(Custom(Status::InternalServerError, Json(error_response)));
        }
    };
    
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
    rocket::build().attach(Cors).mount("/", routes![index, address, withdraw, transaction_details, balance, mnemonic, extended])
}