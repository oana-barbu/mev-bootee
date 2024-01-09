use std::prelude::v1::*;

use eth_types::{SH160, Transaction};
use serde::Deserialize;

pub const MAX_GAS_COST: u32 = 3_000_000;

#[derive(Clone)]
pub struct WrappedBundle {
    pub searcher: SH160,
    pub bid: Bid,
    pub txns: Vec<Transaction>,
    pub estimated_tip: u32,
    pub estimated_gas_cost: u32
}

impl WrappedBundle {
    // returns the value of the bundle (probably the estimated_tip in this case)
    pub fn value(&self) -> u32 {
        self.estimated_tip
    }

    pub fn cost(&self) -> u32 {
        self.estimated_gas_cost
    }

    pub fn contains_transaction(&self, t: &Transaction) -> bool {
        for txn in &self.txns {
            if txn == t {
                return true;
            }
        }
        return false
    }
}

#[derive(Clone, Deserialize)]
pub enum BidType {
    TopOfBlock,
    RestOfBlock
}

#[derive(Clone, Deserialize)]
pub struct Bid {
    pub ty: BidType,
    pub value: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum MevBooTEEError {
    #[error("bad signed header object: {0}")]
    BadSignedHeader(String)
}

#[derive(Clone, Deserialize)]
pub enum PartialBlockBuildingMode {
    BuilderProposes,
    ProposerProposes,
    ProposerChooses
}
