# wallet_lib

A small, reusable Bitcoin wallet library in Rust. It covers BIP39 mnemonics, BIP84 native SegWit (P2WPKH) address derivation, coin selection, and PSBT building and signing. Chain access goes through a pluggable `ChainBackend` trait.

> **Status:** early-stage (v0.1.0). It has not been audited, so don't use it with real funds without your own review.

## Features

- Generate a new wallet or restore one from a 12-word BIP39 mnemonic
- Derive fresh BIP84 receive addresses (`bc1q…` / `tb1q…`) with no address reuse
- Sync UTXOs and report confirmed and unconfirmed balances
- Select coins by fee rate (via `bdk_coin_select`)
- Build PSBTs with automatic change output and a dust threshold
- Sign P2WPKH inputs and broadcast the finished transaction
- Bring your own chain source by implementing `ChainBackend`, or use the in-memory `FakeBackend` for tests

## Installation

Add it to your `Cargo.toml` (adjust the path or Git URL as needed):

```toml
[dependencies]
wallet_lib = { path = "../wallet_lib" }
```

The crate re-exports `bitcoin`, `bip39` and `miniscript`, so you can use the same versions it does through `wallet_lib::bitcoin` and so on.

## Quick Start

```rust
use std::str::FromStr;

use wallet_lib::backend::FakeBackend;
use wallet_lib::bitcoin::{Address, Amount, Network};
use wallet_lib::wallet::Wallet;

fn main() -> wallet_lib::error::Result<()> {
    // 1. Create a wallet. Show the words to the user once; they can't be recovered later.
    let (mut wallet, words) = Wallet::generate(Network::Testnet, FakeBackend::new())?;
    println!("Backup these words: {words}");

    // Or restore an existing one:
    // let mut wallet = Wallet::from_mnemonic(&words, Network::Testnet, backend)?;

    // 2. Get a fresh receive address.
    let address = wallet.new_address()?;
    println!("Receive at: {address}");

    // 3. Sync with the chain and check the balance.
    wallet.sync()?;
    let (confirmed, unconfirmed) = wallet.balance();
    println!("Confirmed: {confirmed}, unconfirmed: {unconfirmed}");

    // 4. Build, sign and broadcast a payment (needs spendable UTXOs).
    let recipient = Address::from_str("tb1q...")
        .unwrap()
        .require_network(Network::Testnet)
        .unwrap();
    let mut psbt = wallet.build_tx(&recipient, Amount::from_sat(50_000), 2.0)?; // 2 sat/vB
    wallet.sign(&mut psbt)?;
    let tx = psbt.unsigned_tx; // inputs now carry final witnesses
    wallet.broadcast(&tx)?;

    Ok(())
}
```

## API Overview

| Method | Description |
|---|---|
| `Wallet::generate(network, backend)` | New wallet with a fresh mnemonic. Returns `(wallet, words)`. |
| `Wallet::from_mnemonic(words, network, backend)` | Restore a wallet from 12 words. |
| `new_address()` | Derive the next unused receive address. |
| `sync()` | Fetch UTXOs for all generated addresses from the backend. |
| `list_utxos()` / `balance()` | Inspect coins and `(confirmed, unconfirmed)` totals as of the last sync. |
| `build_tx(recipient, amount, fee_rate)` | Build an unsigned PSBT. Fee rate is in sat/vB. Change goes to a fresh address. |
| `sign(&mut psbt)` | Sign all inputs with the wallet's keys. |
| `broadcast(&tx)` | Send a signed transaction through the backend. |

Errors are returned as `wallet_lib::error::Error`, for example `InsufficientFunds`, `NoUnusedAddresses` and `Mnemonic`.

## Implementing a Backend

Implement `ChainBackend` for your data source, such as Electrum, Esplora or Bitcoin Core RPC:

```rust
use wallet_lib::backend::{ChainBackend, Utxo};
use wallet_lib::bitcoin::{Address, Transaction};
use wallet_lib::error::Result;

struct MyBackend;

impl ChainBackend for MyBackend {
    fn utxos_for(&self, addresses: &[Address]) -> Result<Vec<Utxo>> { todo!() }
    fn has_history(&self, address: &Address) -> Result<bool> { todo!() }
    fn broadcast(&self, tx: &Transaction) -> Result<()> { todo!() }
}
```

## Project Layout

```
src/
├── lib.rs       # crate root and re-exports
├── wallet.rs    # Wallet<B>: addresses, sync, build_tx, sign, broadcast
├── keys.rs      # mnemonic, account key and address derivation
├── select.rs    # coin selection
├── backend.rs   # ChainBackend trait, Utxo, FakeBackend
└── error.rs     # Error and Result
test/            # integration tests (backendT, keysT, selectT, walletT)
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the transaction lifecycle and design notes.

## Development

```bash
cargo build
cargo test
```

Integration tests live in `test/` rather than Cargo's default `tests/`, and each target is declared in `Cargo.toml`.

## License

No license has been specified yet. Add one before publishing.
