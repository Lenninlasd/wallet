use bdk::bitcoin::Network;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;
use bdk::keys::{
    bip39::{Language, Mnemonic, WordCount},
    DerivableKey, ExtendedKey, GeneratableKey, GeneratedKey,
};
use bdk::template::Bip84;
use bdk::SyncOptions;
use bdk::{miniscript, KeychainKind, Wallet};

use std::env;

pub fn create_wallet() -> Result<Wallet<MemoryDatabase>, String> {
    let mut network = Network::Testnet;

    if let Ok(environment) = env::var("ENVIRONMENT") {
        if environment == "prod" {
            network = Network::Bitcoin;
        }
    }

    let mnemonic_words = match env::var("MNEMONIC_WORDS") {
        Ok(value) => value,
        Err(_) => return Err("Error: MNEMONIC NOT SET".to_string()),
    };

    // Parse a mnemonic
    let mnemonic = Mnemonic::parse(&mnemonic_words).unwrap();
    // Generate the extended key
    let xkey: ExtendedKey = mnemonic.into_extended_key().unwrap();
    // Get xprv from the extended key
    let xprv = xkey.into_xprv(network).unwrap();

    // Create a BDK wallet structure using BIP 84 descriptor ("m/84h/1h/0h/0" and "m/84h/1h/0h/1")
    let wallet = Wallet::new(
        Bip84(xprv, KeychainKind::External),
        Some(Bip84(xprv, KeychainKind::Internal)),
        network,
        MemoryDatabase::new(),
    )
    .unwrap();

    return Ok(wallet);
}

pub fn sync_wallet(wallet: &Wallet<MemoryDatabase>) -> ElectrumBlockchain {
    let client = Client::new("ssl://electrum.blockstream.info:60002").unwrap();
    let blockchain = ElectrumBlockchain::from(client);
    wallet.sync(&blockchain, SyncOptions::default()).unwrap();
    return blockchain;
}

pub fn generate_mnemonic() -> String {
    let mnemonic: GeneratedKey<_, miniscript::Segwitv0> =
        Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();

    // Convert mnemonic to string
    let mnemonic_words = mnemonic.to_string();
    return mnemonic_words;
}

pub fn extended_pub_key() -> String {
    let wallet = create_wallet().unwrap();
    return wallet.get_descriptor_for_keychain(KeychainKind::External).to_string();
}
