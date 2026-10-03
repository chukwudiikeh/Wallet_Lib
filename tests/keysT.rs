//! Integration tests for [`wallet_lib::keys`].

use wallet_lib::bip39::Mnemonic;
use wallet_lib::bitcoin::Network;
use wallet_lib::keys::{derive_account_xpriv, derive_address, generate_mnemonic};

const WORDS: &str = "abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon about";

#[test]
fn generates_a_valid_mnemonic() {
    let words = generate_mnemonic().unwrap();

    // What we generated should itself parse back as valid.
    let parsed = Mnemonic::parse(&words);
    assert!(parsed.is_ok());

    // Twelve words, as asked.
    assert_eq!(words.split_whitespace().count(), 12);
}

#[test]
fn derives_a_testnet_address() {
    let account = derive_account_xpriv(WORDS, Network::Testnet).unwrap();
    let address = derive_address(&account, 0, Network::Testnet).unwrap();

    assert!(address.to_string().starts_with("tb1q"));
}

#[test]
fn same_words_give_same_address() {
    let a = derive_account_xpriv(WORDS, Network::Testnet).unwrap();
    let b = derive_account_xpriv(WORDS, Network::Testnet).unwrap();

    assert_eq!(
        derive_address(&a, 0, Network::Testnet).unwrap(),
        derive_address(&b, 0, Network::Testnet).unwrap(),
    );
}
