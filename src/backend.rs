use bitcoin::{Address, Amount, OutPoint};
use bitcoin::Transaction;

use crate::error::Result;

/// One coin sitting at one of our addresses.
#[derive(Clone)]
pub struct Utxo {
    pub outpoint: OutPoint,
    pub address: Address,
    pub amount: Amount,
    pub confirmed: bool,
}
/// Anything that can tell us what's sitting on the chain, given a list of
/// addresses to check.
pub trait ChainBackend {
    fn utxos_for(&self, addresses: &[Address]) -> Result<Vec<Utxo>>;

    fn has_history(&self, address: &Address) -> Result<bool>;

    /// Broadcast a signed transaction to the network.
    fn broadcast(&self, tx: &Transaction) -> Result<()>;
}
pub struct FakeBackend {
    utxos: Vec<Utxo>,
    used: Vec<Address>,
}

impl FakeBackend {
    pub fn new() -> Self {
        FakeBackend {
            utxos: Vec::new(),
            used: Vec::new(),
        }
    }

    /// Add a UTXO as if it were sitting on the chain. Also marks its
    /// address as having history.
    pub fn add_utxo(&mut self, utxo: Utxo) {
        if !self.used.contains(&utxo.address) {
            self.used.push(utxo.address.clone());
        }
        self.utxos.push(utxo);
    }

    /// Mark an address as having history, with no current UTXO — for
    /// testing a used-but-fully-spent address.
    pub fn mark_used(&mut self, address: Address) {
        if !self.used.contains(&address) {
            self.used.push(address);
        }
    }
}

impl ChainBackend for FakeBackend {
    fn utxos_for(&self, addresses: &[Address]) -> Result<Vec<Utxo>> {
        Ok(self
            .utxos
            .iter()
            .filter(|u| addresses.contains(&u.address))
            .cloned()
            .collect())
    }

    fn has_history(&self, address: &Address) -> Result<bool> {
        Ok(self.used.contains(address))
    }

    fn broadcast(&self, _tx: &Transaction) -> Result<()> {
        // Nothing to actually broadcast to in tests — just succeed.
        Ok(())
    }
}
