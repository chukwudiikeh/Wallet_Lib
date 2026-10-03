//! Integration tests for [`wallet_lib::select`].

use std::str::FromStr;

use wallet_lib::backend::Utxo;
use wallet_lib::bitcoin::{Address, Amount, Network, OutPoint, Txid};
use wallet_lib::select::select_coins;

fn dummy_utxo(sats: u64, vout: u32) -> Utxo {
    let address = Address::from_str("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx")
        .unwrap()
        .require_network(Network::Testnet)
        .unwrap();

    Utxo {
        outpoint: OutPoint::new(
            Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap(),
            vout,
        ),
        address,
        amount: Amount::from_sat(sats),
        confirmed: true,
    }
}

#[test]
fn selects_enough_to_cover_the_target() {
    let utxos = vec![dummy_utxo(100_000, 0), dummy_utxo(50_000, 1)];

    let chosen = select_coins(&utxos, 80_000, 2.0).unwrap();

    let total: u64 = chosen.iter().map(|u| u.amount.to_sat()).sum();

    println!("{}", total);

    assert!(total >= 80_000);
}

#[test]
fn errors_when_funds_are_insufficient() {
    let utxos = vec![dummy_utxo(1_000, 0)];

    let result = select_coins(&utxos, 80_000, 2.0);

    assert!(result.is_err());
}
