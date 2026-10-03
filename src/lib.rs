//! A Reusable Bitcoin wallet library.
//!
//! Create a [`Wallet`](wallet::Wallet) from a mnemonic, derive addresses
//! with [`new_address`](wallet::Wallet::new_address), track coins with
//! [`sync`](wallet::Wallet::sync) and [`balance`](wallet::Wallet::balance),
//! then build and sign transactions with
//! [`build_tx`](wallet::Wallet::build_tx) and [`sign`](wallet::Wallet::sign).
//!
//! Built from first principles on `bitcoin`, `miniscript`, and `bip39`,
//! using BIP84 native SegWit addresses. Chain access is pluggable via the
//! [`ChainBackend`](backend::ChainBackend) trait.

pub use bip39;
pub use bitcoin;
pub use miniscript;

pub mod backend;
pub mod error;
pub mod keys;
pub mod select;
pub mod wallet;
