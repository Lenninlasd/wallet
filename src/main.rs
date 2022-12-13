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
};

use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
// fn main() -> Result<(), bdk::Error> {
    let external_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/0/*)";
    let internal_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/1/*)";

    let wallet = Wallet::new(
        external_descriptor,
        Some(internal_descriptor),
        Network::Testnet,
        MemoryDatabase::new(),
    )?;

    println!("Address #0: {}", wallet.get_address(AddressIndex::Peek(10))?);
    // println!("Address #1: {}", wallet.get_address(AddressIndex::New)?);
    // println!("Address #2: {}", wallet.get_address(AddressIndex:::)?);

    let client = Client::new("ssl://electrum.blockstream.info:60002")?;
    let blockchain = ElectrumBlockchain::from(client);

    wallet.sync(&blockchain, SyncOptions::default())?;

    let balance = wallet.get_balance()?;
    println!("Wallet balance in SAT: {}", balance);


    let faucet_address = Address::from_str("mv4rnyY3Su5gjcDNzbMLKBQkBicCtHUtFB")?;

    let mut tx_builder = wallet.build_tx();
    
    tx_builder
        .add_recipient(faucet_address.script_pubkey(), 1000)
        .enable_rbf();
    
    let (mut psbt, tx_details) = tx_builder.finish()?;
    // let (_, tx_details) = tx_builder.finish()?;

    println!("Transaction details: {:#?}", tx_details);

    let finalized = wallet.sign(&mut psbt, SignOptions::default())?;
    assert!(finalized, "Tx has not been finalized");
    println!("Transaction Signed: {}", finalized);


    let raw_transaction = psbt.extract_tx();
    let txid = raw_transaction.txid();
    blockchain.broadcast(&raw_transaction)?;
    println!(
        "Transaction sent! TXID: {txid}.\nExplorer URL: https://blockstream.info/testnet/tx/{txid}",
        txid = txid
    );

    Ok(())
}
