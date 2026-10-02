use bdk_coin_select::{
    Candidate, ChangePolicy, CoinSelector, DrainWeights, FeeRate, Target, TargetFee, TargetOutputs,
    metrics::LowestFee,
};

use crate::backend::Utxo;
use crate::error::{Error, Result};

const P2WPKH_INPUT_WEIGHT: u64 = 68 * 4;
const P2WPKH_OUTPUT_WEIGHT: u64 = 31 * 4;

/// Choose which UTXOs to spend to cover `target_sats`, aiming for the
/// lowest total fee via Branch and Bound.
pub fn select_coins(utxos: &[Utxo], target_sats: u64, fee_rate: f32) -> Result<Vec<Utxo>> {
    let candidates: Vec<Candidate> = utxos
        .iter()
        .map(|u| Candidate {
            value: u.amount.to_sat(),
            weight: P2WPKH_INPUT_WEIGHT,
            input_count: 1,
            is_segwit: true,
        })
        .collect();

    let target = Target {
        fee: TargetFee::from_feerate(FeeRate::from_sat_per_vb(fee_rate)),
        outputs: TargetOutputs::fund_outputs(std::iter::once((P2WPKH_OUTPUT_WEIGHT, target_sats))),
    };

    let drain_weights = DrainWeights::default();
    let long_term_feerate = FeeRate::from_sat_per_vb(fee_rate);
    let change_policy = ChangePolicy::min_value(drain_weights, 1_000);

    let mut selector = CoinSelector::new(&candidates);
    let metric = LowestFee {
        target,
        long_term_feerate,
        change_policy,
    };

    selector
        .run_bnb(metric, 100_000)
        .map_err(|_| Error::InsufficientFunds {
            needed: target_sats,
            available: utxos.iter().map(|u| u.amount.to_sat()).sum(),
        })?;

    let chosen = selector.apply_selection(utxos).cloned().collect();

    Ok(chosen)
}
