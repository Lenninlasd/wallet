use bdk_wallet::{bitcoin::Network, KeychainKind, Wallet};
use bdk_wallet::rusqlite::Connection;

fn main() {
    // Open or create a new file store for wallet data.
    // let mut db =
    //     bdk_file_store::Store::<ChangeSet>::open_or_create_new(b"magic_bytes", "/tmp/my_wallet.db")
    //         .expect("create store");

    let mut conn = Connection::open_in_memory().expect("must open connection");

    // Create a wallet with initial wallet data read from the file store.
    let network = Network::Testnet;
    let descriptor = "wpkh(tprv8ZgxMBicQKsPdcAqYBpzAFwU5yxBUo88ggoBqu1qPcHUfSbKK1sKMLmC7EAk438btHQrSdu3jGGQa6PA71nvH5nkDexhLteJqkM4dQmWF9g/84'/1'/0'/0/*)";
    let change_descriptor = "wpkh(tprv8ZgxMBicQKsPdcAqYBpzAFwU5yxBUo88ggoBqu1qPcHUfSbKK1sKMLmC7EAk438btHQrSdu3jGGQa6PA71nvH5nkDexhLteJqkM4dQmWF9g/84'/1'/0'/1/*)";
    let wallet_opt = Wallet::load()
        .descriptor(KeychainKind::External, Some(descriptor))
        .descriptor(KeychainKind::Internal, Some(change_descriptor))
        .extract_keys()
        .check_network(network)
        .load_wallet(&mut conn)
        .expect("wallet");
    let mut wallet = match wallet_opt {
        Some(wallet) => wallet,
        None => Wallet::create(descriptor, change_descriptor)
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