#[macro_use] extern crate rocket;
use rocket::{get, http::Status, serde::json::Json};
use serde::{Serialize, Deserialize};

use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;

use bdk::{
    Wallet,
    wallet::get_funded_wallet,
    Balance,
    bitcoin::{
        Network,
        Address,
    },
    keys::{
        DerivableKey,
        ExtendedKey,
        bip39::Mnemonic
    },
    template::Bip84,
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
    FeeRate,
    KeychainKind,
};

use std::{
    str::FromStr,
    env,
};

fn get_test_wpkh() -> &'static str {
    "wpkh(cVpPVruEDdmutPzisEsYvtST1usBR3ntr8pXSyt6D2YYqXRyPcFW)"
}

fn create_wallet() -> Result<Wallet<MemoryDatabase>, String> {
    let  network = Network::Testnet;

    let mnemonic_words = match env::var("MNEMONIC_WORDS") {
        Ok(value) => value,
        Err(_) => return Err("Error: MNEMONIC NOT SET".to_string()),
    };

    // Parse a mnemonic
    let mnemonic  = Mnemonic::parse(&mnemonic_words).unwrap();
    // Generate the extended key
    let xkey: ExtendedKey = mnemonic.into_extended_key().unwrap();
    // Get xprv from the extended key
    let xprv = xkey.into_xprv(network).unwrap();

    let wallet = Wallet::new(
        Bip84(xprv, KeychainKind::External),
        Some(Bip84(xprv, KeychainKind::Internal)),
        network,
        MemoryDatabase::new(),
    ).unwrap();

    return Ok(wallet);
}

fn sync_wallet(wallet: &Wallet<MemoryDatabase>) -> ElectrumBlockchain {
    let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
    let blockchain = ElectrumBlockchain::from(client);
    wallet.sync(&blockchain, SyncOptions::default()).unwrap();
    return blockchain;
}

#[derive(Serialize, Deserialize)]
struct Withdraw {
    amount: u64,
    address: String,
    fee_rate: f32,
}

#[get("/address")]
fn address() -> String {
    let wallet = create_wallet().unwrap();
    return wallet.get_address(AddressIndex::Peek(10)).unwrap().to_string();
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

// #[get("/balance")]
#[post("/withdraw", format = "json", data = "<withdraw>")]
fn withdraw(withdraw: Json<Withdraw>) -> Result<Json<TransactionDetails>, Status> {
    let wallet = create_wallet().unwrap();
    let  blockchain = sync_wallet(&wallet);

    // mv4rnyY3Su5gjcDNzbMLKBQkBicCtHUtFB
    let faucet_address = Address::from_str(&withdraw.address).unwrap();

    let mut tx_builder = wallet.build_tx();
    
    // TODO: handle error for failed transaction
    tx_builder
        .add_recipient(faucet_address.script_pubkey(), withdraw.amount)
        .fee_rate(FeeRate::from_sat_per_vb(withdraw.fee_rate))
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
    rocket::build().attach(Cors).mount("/", routes![index, address, withdraw, transaction_details, balance])
}