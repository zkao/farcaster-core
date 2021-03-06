use bitcoin::blockdata::transaction::TxOut;
use bitcoin::util::amount;
use bitcoin::util::psbt::PartiallySignedTransaction;
use strict_encoding::{StrictDecode, StrictEncode};

use farcaster_core::blockchain::{Fee, FeePolitic, FeeStrategy, FeeStrategyError};
use farcaster_core::consensus::{self, Decodable, Encodable};

use crate::bitcoin::transaction;
use crate::bitcoin::{Amount, Bitcoin};

use std::io;
use std::str::FromStr;

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, StrictDecode, StrictEncode)]
pub struct SatPerVByte(Amount);

impl SatPerVByte {
    pub fn from_sat(satoshi: u64) -> Self {
        SatPerVByte(Amount::from_sat(satoshi))
    }

    pub fn as_sat(&self) -> u64 {
        self.0.as_sat()
    }

    pub fn as_native_unit(&self) -> Amount {
        self.0
    }
}

impl Encodable for SatPerVByte {
    fn consensus_encode<W: io::Write>(&self, writer: &mut W) -> Result<usize, io::Error> {
        self.0.consensus_encode(writer)
    }
}

impl Decodable for SatPerVByte {
    fn consensus_decode<D: io::Read>(d: &mut D) -> Result<Self, consensus::Error> {
        let amount: Amount = Decodable::consensus_decode(d)?;
        Ok(SatPerVByte(amount))
    }
}

impl FromStr for SatPerVByte {
    type Err = consensus::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x = s
            .parse::<u64>()
            .map_err(|_| consensus::Error::ParseFailed("Failed to parse amount"))?;
        Ok(Self(Amount(amount::Amount::from_sat(x))))
    }
}

impl Fee for Bitcoin {
    type FeeUnit = SatPerVByte;

    /// Calculates and sets the fees on the given transaction and return the fees set
    fn set_fee(
        tx: &mut PartiallySignedTransaction,
        strategy: &FeeStrategy<SatPerVByte>,
        politic: FeePolitic,
    ) -> Result<Amount, FeeStrategyError> {
        // Get the available amount on the transaction
        let inputs: Result<Vec<TxOut>, FeeStrategyError> = tx
            .inputs
            .iter()
            .map(|psbt_in| {
                psbt_in
                    .witness_utxo
                    .clone()
                    .ok_or(FeeStrategyError::MissingInputsMetadata)
            })
            .collect();
        let input_sum = Amount::from_sat(inputs?.iter().map(|txout| txout.value).sum());

        // FIXME This does not account for witnesses
        // currently the fees are wrong
        // Get the transaction weight
        let weight = tx.global.unsigned_tx.get_weight() as u64;

        // Compute the fee amount to set in total
        let fee_amount = match strategy {
            FeeStrategy::Fixed(sat_per_vbyte) => sat_per_vbyte.as_native_unit().checked_mul(weight),
            FeeStrategy::Range(range) => match politic {
                FeePolitic::Aggressive => range.start.as_native_unit().checked_mul(weight),
                FeePolitic::Conservative => range.end.as_native_unit().checked_mul(weight),
            },
        }
        .ok_or_else(|| FeeStrategyError::AmountOfFeeTooHigh)?;

        if tx.global.unsigned_tx.output.len() != 1 {
            return Err(FeeStrategyError::new(
                transaction::Error::MultiUTXOUnsuported,
            ));
        }

        // Apply the fee on the first output
        tx.global.unsigned_tx.output[0].value = input_sum
            .checked_sub(fee_amount)
            .ok_or_else(|| FeeStrategyError::NotEnoughAssets)?
            .as_sat();

        // Return the fee amount set in native blockchain asset unit
        Ok(fee_amount)
    }

    /// Validates that the fees for the given transaction are set accordingly to the strategy
    fn validate_fee(
        _tx: &PartiallySignedTransaction,
        _strategy: &FeeStrategy<SatPerVByte>,
    ) -> Result<bool, FeeStrategyError> {
        todo!()
    }
}
