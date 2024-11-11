use bdk_wallet::{bitcoin::Network, KeychainKind, Wallet};
use bdk_wallet::rusqlite::Connection;
use bdk_wallet::keys::{
    bip39::{Mnemonic, Language},
    ExtendedKey, DerivableKey,
};
use bdk_wallet::descriptor::template::Bip84;

use std::env;

fn main() {
    // Open or create a new file store for wallet data.
    let mut conn = Connection::open_in_memory().expect("must open connection");

    // Create a wallet with initial wallet data read from the file store.
    let network = Network::Testnet;

    let mnemonic_words = env::var("MNEMONIC_WORDS").expect("Error: MNEMONIC NOT SET");

     // Parse a mnemonic
     let mnemonic = Mnemonic::parse_in(Language::English, &mnemonic_words).unwrap();
     // Generate the extended key
     let xkey: ExtendedKey = mnemonic.into_extended_key().unwrap();
     // Get xprv from the extended key
     let xprv = xkey.into_xprv(network).unwrap();

    let wallet_opt = Wallet::load()
        .descriptor(KeychainKind::External, Some(Bip84(xprv, KeychainKind::External)))
        .descriptor(KeychainKind::Internal, Some(Bip84(xprv, KeychainKind::Internal)))
        .extract_keys()
        .check_network(network)
        .load_wallet(&mut conn)
        .expect("wallet");

    let mut wallet = match wallet_opt {
        Some(wallet) => wallet,
        None => Wallet::create(Bip84(xprv, KeychainKind::External), Bip84(xprv, KeychainKind::Internal))
            .network(network)
            .create_wallet(&mut conn)
            .expect("wallet"),
    };

    // Get a new address to receive bitcoin.
    let receive_address = wallet.reveal_next_address(KeychainKind::External);
    // Persist staged wallet data changes to the file store.
    wallet.persist(&mut conn).expect("persist");
    println!("Your new receive address is: {}", receive_address.address);

}