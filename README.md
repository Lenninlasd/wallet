# wallet

The on-chain Bitcoin wallet service of Zener, a Colombian P2P exchange. It is a small Rust HTTP service built on [BDK](https://bitcoindevkit.org/) (Bitcoin Dev Kit) and [Rocket](https://rocket.rs/). The exchange backend, [exchange-engine](https://github.com/lenninlasd/exchange-engine), calls it to pay out customer withdrawals.

## How it works

- **Keys:** the wallet is a BIP84 (native SegWit) wallet derived from a BIP39 mnemonic passed in the `MNEMONIC_WORDS` environment variable. The mnemonic never lives in the code or the repo.
- **Chain data:** it syncs with an Electrum server (Blockstream's), on testnet by default or on mainnet with `BITCOIN_NETWORK=Mainnet`.
- **Withdrawals:** `POST /withdraw` takes a reference, an amount in sats, a destination address and a fee rate. It builds the transaction with RBF enabled, signs the PSBT and broadcasts it, and returns the transaction details. Insufficient funds come back as a typed error that the engine can act on.
- **Request integrity:** each withdrawal carries a SHA-256 checksum of its fields plus a secret shared with the engine (`WITHDRAWAL_INTEGRITY`). The wallet rejects a request whose checksum doesn't match, so a tampered amount or address is never signed.

## Endpoints

| Method | Path | What it does |
| --- | --- | --- |
| `POST` | `/withdraw` | Verifies the checksum, then builds, signs and broadcasts a withdrawal |
| `GET` | `/balance` | Syncs and returns the wallet balance |
| `GET` | `/address` | Returns a receive address |
| `GET` | `/extended` | Returns the external descriptor (xpub) |
| `GET` | `/transactiondetails` | Builds a transaction against a test wallet to preview its details and fee |
| `GET` | `/rand_mnemonic` | Generates a new 12-word mnemonic (setup helper) |

## Running it

```sh
export MNEMONIC_WORDS="<12 or 24 words>"
export WITHDRAWAL_INTEGRITY="<secret shared with the engine>"
export BITCOIN_NETWORK=Testnet   # or Mainnet
cargo run                         # serves on port 8000
```

Or with Docker:

```sh
docker build -t wallet .
docker run -p 8000:8000 -e MNEMONIC_WORDS -e WITHDRAWAL_INTEGRITY -e BITCOIN_NETWORK wallet
```

## CI/CD

GitHub Actions builds and tests every pull request to `main`. Publishing a release builds and tests the service, pushes a Docker image to DigitalOcean's container registry and deploys it. An issue or issue comment can redeploy an existing image to dev or prod.

## Stack

Rust, BDK 0.28, Rocket 0.5, Electrum, Docker, GitHub Actions, DigitalOcean.
