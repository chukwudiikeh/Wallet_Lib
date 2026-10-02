use bitcoin::absolute::LockTime;
use bitcoin::bip32::Xpriv;
use bitcoin::psbt::Psbt;
use bitcoin::transaction::Version;
use bitcoin::{Address, Amount, Network, Sequence, Transaction, TxIn, TxOut, Witness};
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{bip32::DerivationPath, ecdsa, CompressedPublicKey};
use bitcoin::hashes::Hash;
use std::str::FromStr;

use crate::backend::{ChainBackend, Utxo};
use crate::error::{Error, Result};
use crate::keys::{derive_account_xpriv, derive_address, generate_mnemonic};
use crate::select::select_coins;

/// A Bitcoin wallet: derives addresses, tracks coins, and builds and signs
/// transactions, using BIP84 native SegWit.
pub struct Wallet<B: ChainBackend> {
    account_xpriv: Xpriv,
    network: Network,
    next_index: u32,
    addresses: Vec<Address>,
    backend: B,
    utxos: Vec<Utxo>,
}

impl<B: ChainBackend> Wallet<B> {
    /// Create a wallet from a twelve-word BIP39 mnemonic.
    ///
    /// `backend` is anything implementing [`ChainBackend`], used to look up
    /// UTXOs on [`sync`](Wallet::sync).
    pub fn from_mnemonic(words: &str, network: Network, backend: B) -> Result<Self> {
        let account_xpriv = derive_account_xpriv(words, network)?;

        Ok(Wallet {
            account_xpriv,
            network,
            next_index: 0,
            addresses: Vec::new(),
            backend,
            utxos: Vec::new(),
        })
    }

    /// Create a brand-new wallet with a freshly generated mnemonic.
    ///
    /// Returns the wallet, plus the twelve words — show these to the user
    /// once, so they can write them down. They cannot be recovered later.
    pub fn generate(network: Network, backend: B) -> Result<(Self, String)> {
        let words = generate_mnemonic()?;
        let wallet = Self::from_mnemonic(&words, network, backend)?;
        Ok((wallet, words))
    }

    /// Derive and return the next unused receive address.
    ///
    /// Each call returns a fresh address; addresses are never reused.
    pub fn new_address(&mut self) -> Result<Address> {
        let address = derive_address(&self.account_xpriv, self.next_index, self.network)?;
        self.next_index += 1;
        self.addresses.push(address.clone());
        Ok(address)
    }

    /// Ask the backend for the latest UTXOs on every address this wallet
    /// has generated so far, replacing any previously known UTXOs.
    pub fn sync(&mut self) -> Result<()> {
        self.utxos = self.backend.utxos_for(&self.addresses)?;
        Ok(())
    }

    /// Every UTXO known as of the last [`sync`](Wallet::sync).
    pub fn list_utxos(&self) -> &[Utxo] {
        &self.utxos
    }

    /// Confirmed and unconfirmed balance, as `(confirmed, unconfirmed)`,
    /// as of the last [`sync`](Wallet::sync).
    pub fn balance(&self) -> (Amount, Amount) {
        let mut confirmed = Amount::ZERO;
        let mut unconfirmed = Amount::ZERO;

        for utxo in &self.utxos {
            if utxo.confirmed {
                confirmed += utxo.amount;
            } else {
                unconfirmed += utxo.amount;
            }
        }

        (confirmed, unconfirmed)
    }

    /// Build an unsigned PSBT paying `amount` to `recipient`, selecting
    /// coins to cover it and sending any leftover change to a fresh
    /// address of our own.
    pub fn build_tx(&mut self, recipient: &Address, amount: Amount, fee_rate: f32) -> Result<Psbt> {
        const BASE_TX_VB: u64 = 11;
        const P2WPKH_INPUT_VB: u64 = 68;
        const P2WPKH_OUTPUT_VB: u64 = 31;
        const DUST_LIMIT: u64 = 1_000;

        let amount_sats = amount.to_sat();

        let chosen = select_coins(&self.utxos, amount_sats, fee_rate)?;
        let total_in: u64 = chosen.iter().map(|u| u.amount.to_sat()).sum();

        let fee_no_change = ((BASE_TX_VB + chosen.len() as u64 * P2WPKH_INPUT_VB + P2WPKH_OUTPUT_VB)
            as f32
            * fee_rate) as u64;
        let leftover = total_in
            .saturating_sub(amount_sats)
            .saturating_sub(fee_no_change);

        let mut outputs = vec![TxOut {
            value: amount,
            script_pubkey: recipient.script_pubkey(),
        }];

        if leftover > DUST_LIMIT {
            let fee_with_change =
                ((BASE_TX_VB + chosen.len() as u64 * P2WPKH_INPUT_VB + 2 * P2WPKH_OUTPUT_VB) as f32
                    * fee_rate) as u64;
            let change_amount = total_in - amount_sats - fee_with_change;

            let change_address = self.new_address()?;
            outputs.push(TxOut {
                value: Amount::from_sat(change_amount),
                script_pubkey: change_address.script_pubkey(),
            });
        }

        let inputs: Vec<TxIn> = chosen
            .iter()
            .map(|u| TxIn {
                previous_output: u.outpoint,
                script_sig: Default::default(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::default(),
            })
            .collect();

        let tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: inputs,
            output: outputs,
        };

        let mut psbt = Psbt::from_unsigned_tx(tx).map_err(|e| Error::Backend(e.to_string()))?;

        for (i, utxo) in chosen.iter().enumerate() {
            psbt.inputs[i].witness_utxo = Some(TxOut {
                value: utxo.amount,
                script_pubkey: utxo.address.script_pubkey(),
            });
        }

        Ok(psbt)
    }
    /// Sign every input of `psbt` using this wallet's own keys.
    ///
    /// Since this wallet is always the sole signer, this fills in each
    /// input's final witness directly, producing an already-complete,
    /// broadcastable transaction.
    pub fn sign(&self, psbt: &mut Psbt) -> Result<()> {
        let secp = Secp256k1::new();

        // Gather what we need from each input before borrowing the transaction mutably.
        let mut utxos = Vec::with_capacity(psbt.inputs.len());
        for input in &psbt.inputs {
            let utxo = input
                .witness_utxo
                .clone()
                .ok_or_else(|| Error::Backend("missing witness_utxo".to_string()))?;
            utxos.push(utxo);
        }

        let mut sighasher = SighashCache::new(&mut psbt.unsigned_tx);

        for (i, utxo) in utxos.iter().enumerate() {
            // Which of our addresses does this input belong to?
            let index = self
                .addresses
                .iter()
                .position(|a| a.script_pubkey() == utxo.script_pubkey)
                .ok_or_else(|| Error::Backend("unknown input address".to_string()))?;

            let path = DerivationPath::from_str(&format!("m/0/{index}"))?;
            let child = self.account_xpriv.derive_priv(&secp, &path)?;
            let private_key = child.to_priv();
            let public_key = CompressedPublicKey::from_private_key(&secp, &private_key)
                .map_err(|e| Error::Backend(e.to_string()))?;

            let sighash = sighasher
                .p2wpkh_signature_hash(i, &utxo.script_pubkey, utxo.value, EcdsaSighashType::All)
                .map_err(|e| Error::Backend(e.to_string()))?;

            let message = Message::from_digest(sighash.to_byte_array());
            let signature = secp.sign_ecdsa(&message, &private_key.inner);
            let signature = ecdsa::Signature {
                signature,
                sighash_type: EcdsaSighashType::All,
            };

            *sighasher.witness_mut(i).unwrap() = Witness::p2wpkh(&signature, &public_key.0);
        }

        Ok(())
    }
    
    /// Broadcast a signed transaction via this wallet's backend.
    pub fn broadcast(&self, tx: &bitcoin::Transaction) -> Result<()> {
        self.backend.broadcast(tx)
    }

}
