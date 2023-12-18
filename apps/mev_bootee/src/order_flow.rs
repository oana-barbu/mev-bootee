use std::prelude::v1::*;

use std::collections::BTreeMap;

use crate::{Transaction, Bundle, ExecutionState, MevBooTEEError};

pub trait OrderFlow {
    fn new(es: ExecutionState) -> Self;
    fn add_bundle(&mut self, bundle_id: String, bundle: Bundle);
    fn remove_bundle(&mut self, bundle_id: &String) -> bool;
    fn add_inclusion_list(&mut self, inclusion_list: Vec<Transaction>);
    fn get_block(&self) -> eth_types::Block;
    fn get_block_header(&self) -> eth_types::BlockHeader;
    fn create_bundle(&self, transactions: Vec<Transaction>) -> Result<Bundle, MevBooTEEError>; // create and verify the bundle against the starting state of the order flow
}

pub struct GreedyOrderFlow {
    pub starting_state: ExecutionState,
    pub all_bundles: BTreeMap<String, Bundle>, // all bundles
    pub ordered_bundle_ids: Vec<String>, // ordered bundle_ids (based on the value of the bundle)
    pub proposer_requested_txns: Vec<Transaction>, // transactions the proposer wants included
    
    pub block: Vec<String>, // bundle ids as they appear in the final block
    pub inclusion_list: Vec<Transaction> // last part of the block: subset of proposer_requested_txns (a txn may be included in a bundle and excluded from this subset)
}

impl GreedyOrderFlow {
    // to be run:
    // 1. after we add new bundles to ensure the block is still maximized
    // 2. after an inclusion list is added to ensure the block is still valid
    fn rebuild(&mut self) {
        self.block = Vec::new();

        let mut curr_state = self.starting_state.clone();
        let mut inclusion_list = self.proposer_requested_txns.clone();

        for bundle_id in &self.ordered_bundle_ids.clone() {
            let bundle = &self.all_bundles[bundle_id];
            // for each bundle, we execute the txns in the bundle, followed by the txns in the inclusion list (after we removed any common txns)
            let before_bundle_state = curr_state.clone(); // if the bundle and inclusion list conflict, we need to return to the state before applying the bundle
            if self.execute_bundle(&bundle.txns, &mut curr_state) {
                if inclusion_list.len() > 0 { // apply the remainder of the inclusion list to ensure the current bundle does not conflict with it
                    let inclusion_list_backup = inclusion_list.clone();
                    remove_common_txns(bundle, &mut inclusion_list);
                    let after_bundle_state = curr_state.clone(); // if there is no conflict, we want to return to the state before applying the inclusion list
                    if self.execute_bundle(&self.inclusion_list, &mut curr_state) {
                        // all good, we keep the bundle
                        self.block.push(bundle_id.clone());
                        curr_state = after_bundle_state;
                    }
                    else {
                        // bundle and inclusion list conflict, undo the transactions in the bundle
                        curr_state = before_bundle_state;
                        // restore inclusion_list
                        inclusion_list = inclusion_list_backup;
                    }
                }    
            }
        }
        self.inclusion_list = inclusion_list;
        // at the end, the block is formed from the transactions in self.block followed by the transactions in self.inclusion_list
    }

    // return true if bundle is valid with the current state
    // if bundle conflicts, it restores state to what it was
    fn execute_bundle(&self, txns: &Vec<Transaction>, execution_state: &mut ExecutionState) -> bool {
        todo!()
    }

    fn add_bundle_id(&mut self, bundle_id: &String, bundle_value: u32) {
        for i in 0..self.ordered_bundle_ids.len() {
            if self.all_bundles[&self.ordered_bundle_ids[i]].value() < bundle_value {
                self.ordered_bundle_ids.insert(i, bundle_id.into());
                return
            }
        }
        self.ordered_bundle_ids.push(bundle_id.into())
    }
}

impl OrderFlow for GreedyOrderFlow {
    fn new(ex: ExecutionState) -> GreedyOrderFlow {
        todo!()
    }

    fn add_bundle(&mut self, bundle_id: String, bundle: Bundle) {
        self.add_bundle_id(&bundle_id, bundle.value());
        self.rebuild()
    }

    fn remove_bundle(&mut self, bundle_id: &String) -> bool {
        self.all_bundles.remove(bundle_id);
        for i in 0..self.ordered_bundle_ids.len() {
            if self.ordered_bundle_ids[i] == bundle_id.to_string() {
                self.ordered_bundle_ids.remove(i);
                return true
            }
        }
        return false
    }

    fn add_inclusion_list(&mut self, inclusion_list: Vec<Transaction>) {
        self.proposer_requested_txns = inclusion_list;
        self.rebuild()
    }

    fn get_block(&self) -> eth_types::Block {
        todo!()
    }

    fn get_block_header(&self) -> eth_types::BlockHeader {
        todo!()
    }

    fn create_bundle(&self, transactions: Vec<Transaction>) -> Result<Bundle, MevBooTEEError> {
        todo!()
    }
}

fn remove_common_txns(bundle: &Bundle, inclusion_list: &mut Vec<Transaction>) {
    let mut to_remove_idx = Vec::new();
    for i in 0..inclusion_list.len() {
        if bundle.contains_transaction(&inclusion_list[i]) {
            to_remove_idx.push(i);
        }
    }
    to_remove_idx.reverse();
    for idx in to_remove_idx {
        inclusion_list.remove(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_bundle() {
        let order_flow = GreedyOrderFlow::new();
    }
}
