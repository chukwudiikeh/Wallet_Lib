use std::str::FromStr;
use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, CompressedPublicKey, Network};

use crate::error::{Error, Result};

/// Generate a brand-new, random twelve-word BIP39 mnemonic.
pub fn generate_mnemonic() -> Result<String> {
    let mnemonic = Mnemonic::generate(12).map_err(Error::from)?;
    Ok(mnemonic.to_string())
}

/// Derive the account-level key (`m/84'/coin_type'/0'`) from a twelve-word
/// mnemonic.
pub fn derive_account_xpriv(words: &str, network: Network) -> Result<Xpriv> {
    let mnemonic = Mnemonic::parse(words)?;
    let seed = mnemonic.to_seed("");

    let master = Xpriv::new_master(network, &seed)?;

    let coin_type = if network == Network::Bitcoin { 0 } else { 1 };
    let secp = Secp256k1::new();
    let path = DerivationPath::from_str(&format!("m/84'/{coin_type}'/0'"))?;
    let account = master.derive_priv(&secp, &path)?;

    Ok(account)
}

pub fn derive_address(account_xpriv: &Xpriv, index: u32, network: Network) -> Result<Address> {
    let secp = Secp256k1::new();

    let path = DerivationPath::from_str(&format!("m/0/{index}"))?;
    let child = account_xpriv.derive_priv(&secp, &path)?;

    let public_key = CompressedPublicKey::from_private_key(&secp, &child.to_priv())
        .map_err(|e| Error::Backend(e.to_string()))?;

    Ok(Address::p2wpkh(&public_key, network))
}
