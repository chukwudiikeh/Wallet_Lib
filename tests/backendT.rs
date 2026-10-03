//! Integration tests for [`wallet_lib::backend`].
use std::str::FromStr;

use wallet_lib::backend::{ChainBackend, FakeBackend, Utxo};
use wallet_lib::bitcoin::{Address, Amount, Network, OutPoint, Txid};

#[test]
fn has_history_tracks_used_addresses() {
    use wallet_lib::keys::{derive_account_xpriv, derive_address};

    const WORDS: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    let account = derive_account_xpriv(WORDS, Network::Testnet).unwrap();
    let address = derive_address(&account, 0, Network::Testnet).unwrap();
    let unused_address = derive_address(&account, 1, Network::Testnet).unwrap();

    let mut backend = FakeBackend::new();

    // Neither address has history yet.
    assert!(!backend.has_history(&address).unwrap());
    assert!(!backend.has_history(&unused_address).unwrap());

    // Adding a UTXO marks its address as having history.
    backend.add_utxo(Utxo {
        outpoint: OutPoint::new(
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap(),
            0,
        ),
        address: address.clone(),
        amount: Amount::from_sat(500),
        confirmed: true,
    });
    assert!(backend.has_history(&address).unwrap());

    // Marking an address used directly, with no UTXO, covers a
    // used-but-now-fully-spent address.
    backend.mark_used(unused_address.clone());
    assert!(backend.has_history(&unused_address).unwrap());

    // And it genuinely has no current UTXO, proving the two are
    // tracked separately.
    let utxos = backend.utxos_for(&[unused_address]).unwrap();
    assert!(utxos.is_empty());
}

#[test]
fn returns_utxos_for_a_known_address() {
    let address = Address::from_str("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx")
        .unwrap()
        .require_network(Network::Testnet)
        .unwrap();

    let outpoint = OutPoint::new(
        Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000").unwrap(),
        0,
    );

    let mut backend = FakeBackend::new();
    backend.add_utxo(Utxo {
        outpoint,
        address: address.clone(),
        amount: Amount::from_sat(500),
        confirmed: true,
    });

    let found = backend.utxos_for(&[address]).unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].amount, Amount::from_sat(500));
}
