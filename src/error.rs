use thiserror::Error;

#[derive(Debug, Error)]

/// Everything that can go wrong in wallet_lib.
pub enum Error {
    #[error("insufficient funds: needed {needed} sats, have {available} sats")]
    InsufficientFunds { needed: u64, available: u64 },

    #[error("no unused addresses available")]
    NoUnusedAddresses,

    #[error("invalid mnemonic: {0}")]
    Mnemonic(#[from] bip39::Error),

    #[error("invalid descriptor: {0}")]
    Descriptor(#[from] miniscript::Error),

    #[error("key derivation failed: {0}")]
    Derivation(#[from] bitcoin::bip32::Error),

    #[error("amount of {amount} sats is dust: the smallest payment this address accepts is {minimum} sats")]
    DustAmount { amount: u64, minimum: u64 },

    #[error("chain backend error: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, Error>;
