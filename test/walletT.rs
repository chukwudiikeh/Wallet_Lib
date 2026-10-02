//! Integration tests for [`wallet_lib::wallet`].

use std::str::FromStr;

use wallet_lib::backend::{FakeBackend, Utxo};
use wallet_lib::bitcoin::hashes::Hash;
use wallet_lib::bitcoin::{Address, Amount, Network, OutPoint, Txid};
use wallet_lib::keys::{derive_account_xpriv, derive_address};
use wallet_lib::wallet::Wallet;

const WORDS: &str = "abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon about";

/// The all-zero txid, fine as a stand-in for a UTXO we never actually spend
/// against a real chain.
fn dummy_txid() -> Txid {
    Txid::from_str("0000000000000000000000000000000000000000000000000000000000000000").unwrap()
}

#[test]
fn generate_produces_a_usable_and_restorable_wallet() {
    let backend = FakeBackend::new();
    let (mut wallet, words) = Wallet::generate(Network::Testnet, backend).unwrap();

    // Twelve real words came back.
    assert_eq!(words.split_whitespace().count(), 12);

    // The wallet itself works, same as any other.
    let address = wallet.new_address().unwrap();
    assert!(address.to_string().starts_with("tb1q"));

    // Those same words, fed into from_mnemonic, recreate the identical
    // first address — proving generate() didn't do anything special
    // that from_mnemonic couldn't also do.
    let backend2 = FakeBackend::new();
    let mut restored = Wallet::from_mnemonic(&words, Network::Testnet, backend2).unwrap();
    let restored_address = restored.new_address().unwrap();

    assert_eq!(address, restored_address);
}

#[test]
fn syncs_balance_and_utxos_from_backend() {
    let account = derive_account_xpriv(WORDS, Network::Testnet).unwrap();
    let expected_address = derive_address(&account, 0, Network::Testnet).unwrap();

    let mut backend = FakeBackend::new();
    backend.add_utxo(Utxo {
        outpoint: OutPoint::new(dummy_txid(), 0),
        address: expected_address.clone(),
        amount: Amount::from_sat(500),
        confirmed: true,
    });

    let mut wallet = Wallet::from_mnemonic(WORDS, Network::Testnet, backend).unwrap();
    let address = wallet.new_address().unwrap();

    assert_eq!(address, expected_address);

    wallet.sync().unwrap();

    let (confirmed, unconfirmed) = wallet.balance();
    assert_eq!(confirmed, Amount::from_sat(500));
    assert_eq!(unconfirmed, Amount::ZERO);
    assert_eq!(wallet.list_utxos().len(), 1);
}

#[test]
fn builds_a_psbt_with_a_payment_and_change() {
    let account = derive_account_xpriv(WORDS, Network::Testnet).unwrap();
    let expected_address = derive_address(&account, 0, Network::Testnet).unwrap();

    let mut backend = FakeBackend::new();
    backend.add_utxo(Utxo {
        outpoint: OutPoint::new(dummy_txid(), 0),
        address: expected_address.clone(),
        amount: Amount::from_sat(100_000),
        confirmed: true,
    });

    let mut wallet = Wallet::from_mnemonic(WORDS, Network::Testnet, backend).unwrap();
    wallet.new_address().unwrap();
    wallet.sync().unwrap();

    let recipient = Address::from_str("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx")
        .unwrap()
        .require_network(Network::Testnet)
        .unwrap();

    let psbt = wallet
        .build_tx(&recipient, Amount::from_sat(50_000), 2.0)
        .unwrap();

    assert_eq!(psbt.unsigned_tx.input.len(), 1);
    assert_eq!(psbt.unsigned_tx.output.len(), 2);

    assert_eq!(
        psbt.inputs[0].witness_utxo.as_ref().unwrap().value,
        Amount::from_sat(100_000)
    );

    assert_eq!(psbt.unsigned_tx.output[0].value, Amount::from_sat(50_000));
}

#[test]
fn signs_a_valid_witness() {
    let account = derive_account_xpriv(WORDS, Network::Testnet).unwrap();
    let expected_address = derive_address(&account, 0, Network::Testnet).unwrap();

    let mut backend = FakeBackend::new();
    backend.add_utxo(Utxo {
        outpoint: OutPoint::new(dummy_txid(), 0),
        address: expected_address.clone(),
        amount: Amount::from_sat(100_000),
        confirmed: true,
    });

    let mut wallet = Wallet::from_mnemonic(WORDS, Network::Testnet, backend).unwrap();
    wallet.new_address().unwrap();
    wallet.sync().unwrap();

    let recipient = Address::from_str("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx")
        .unwrap()
        .require_network(Network::Testnet)
        .unwrap();

    let mut psbt = wallet
        .build_tx(&recipient, Amount::from_sat(50_000), 2.0)
        .unwrap();

    wallet.sign(&mut psbt).unwrap();

    // The witness should now have exactly two items: signature, then pubkey.
    let witness = &psbt.unsigned_tx.input[0].witness;
    assert_eq!(witness.len(), 2);

    let utxo = psbt.inputs[0].witness_utxo.as_ref().unwrap();

    // Manually verify the signature against the public key, proving it's
    // a genuine, valid signature for this exact spend.
    let sig_bytes = witness.nth(0).unwrap();
    let pk_bytes = witness.nth(1).unwrap();

    let signature = wallet_lib::bitcoin::ecdsa::Signature::from_slice(sig_bytes).unwrap();
    let public_key = wallet_lib::bitcoin::secp256k1::PublicKey::from_slice(pk_bytes).unwrap();

    let secp = wallet_lib::bitcoin::secp256k1::Secp256k1::new();
    let mut sighasher = wallet_lib::bitcoin::sighash::SighashCache::new(&psbt.unsigned_tx);
    let sighash = sighasher
        .p2wpkh_signature_hash(0, &utxo.script_pubkey, utxo.value, signature.sighash_type)
        .unwrap();

    let message = wallet_lib::bitcoin::secp256k1::Message::from_digest(sighash.to_byte_array());

    secp.verify_ecdsa(&message, &signature.signature, &public_key)
        .expect("signature should be valid");
}
