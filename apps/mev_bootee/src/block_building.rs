use std::prelude::v1::*;

use std::{collections::BTreeMap, sync::{Arc, Mutex}};

use eth_tools::{ExecutionClient, MixRpcClient};
use mpt::{Database, TrieState, BlockStateFetcher};
use evm_executor::{BlockBuilder, ConsensusBlockInfo, Engine, Ethereum, BlockHashGetter};
use statedb::StateDB;

use crate::{WrappedBundle, MevBooTEEError};

use eth_types::{SH256, Transaction, EthereumEngineTypes};

pub trait BlockBuildingStrategy {
    fn new(el: Arc<ExecutionClient<Arc<MixRpcClient>>>, block_number: u64) -> Self;
    fn add_bundle(&mut self, bundle_id: String, bundle: WrappedBundle);
    fn remove_bundle(&mut self, bundle_id: &String) -> bool;
    fn add_inclusion_list(&mut self, inclusion_list: Vec<Transaction>);
    fn get_block(&self) -> eth_types::Block;
    fn get_block_header(&self) -> eth_types::BlockHeader;
    fn create_bundle(&self, transactions: Vec<Transaction>) -> Result<WrappedBundle, MevBooTEEError>; // create and verify the bundle against the starting state of the order flow
}

pub struct GreedyBlockBuildingStrategy {
    pub builder: BlockBuilder<
        Ethereum,
        TrieState<BlockStateFetcher<Arc<MixRpcClient>, EthereumEngineTypes, Arc<ExecutionClient<Arc<MixRpcClient>>>>, Database>,
        BuilderFetcher
    >,
    pub all_bundles: BTreeMap<String, WrappedBundle>, // all bundles
    pub ordered_bundle_ids: Vec<String>, // ordered bundle_ids (based on the value of the bundle)
    pub proposer_requested_txns: Vec<Transaction>, // transactions the proposer wants included
    state_root: SH256,
    pub block: Vec<String>, // bundle ids as they appear in the final block
    pub inclusion_list: Vec<Transaction> // last part of the block: subset of proposer_requested_txns (a txn may be included in a bundle and excluded from this subset)
}

impl GreedyBlockBuildingStrategy {
    // to be run:
    // 1. after we add new bundles or remove existing bundles to ensure the block is still maximized
    // 2. after an inclusion list is added to ensure the block is still valid
    fn rebuild(&mut self) {
        self.block = Vec::new();

        self.reset_builder();
        let mut inclusion_list = self.proposer_requested_txns.clone();

        for bundle_id in &self.ordered_bundle_ids.clone() {
            let bundle = &self.all_bundles[bundle_id];
            // for each bundle, we execute the txns in the bundle, followed by the txns in the inclusion list (after we removed any common txns)
            let before_bundle_state = self.builder.flush_state().unwrap(); // if the bundle and inclusion list conflict, we need to return to the state before applying the bundle
            let before_start_pos = self.builder.txs().len();
            if self.execute_bundle(&bundle.txns) {
                if inclusion_list.len() > 0 { // apply the remainder of the inclusion list to ensure the current bundle does not conflict with it
                    let inclusion_list_backup = inclusion_list.clone();
                    remove_common_txns(bundle, &mut inclusion_list);
                    let after_bundle_state = self.builder.flush_state().unwrap(); // if there is no conflict, we want to return to the state before applying the inclusion list
                    let after_start_pos = self.builder.txs().len();
                    if self.execute_bundle(&self.inclusion_list) {
                        // all good, we keep the bundle
                        self.block.push(bundle_id.clone());
                        self.builder.truncate_and_revert(after_start_pos, after_bundle_state);
                    }
                    else {
                        // bundle and inclusion list conflict, undo the transactions in the bundle
                        self.builder.truncate_and_revert(before_start_pos, before_bundle_state);
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
    fn execute_bundle(&self, txns: &Vec<Transaction>) -> bool {
        todo!()
    }

    fn reset_builder(&mut self) {
        self.builder.truncate_and_revert(0, self.state_root);
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

impl BlockBuildingStrategy for GreedyBlockBuildingStrategy {
    fn new(el: Arc<ExecutionClient<Arc<MixRpcClient>>>, block_number: u64) -> Self {
        let chain_id = el.chain_id().unwrap();
        let prev_block = el.get_block_header(block_number.into()).unwrap();

        let mut builder = {
            // use the ethereum engine
            let engine = Ethereum::new(chain_id.into());
            let header = engine.new_block_header(
                &prev_block,
                ConsensusBlockInfo {
                    gas_limit: todo!(),
                    timestamp: todo!(),
                    random: todo!(),
                    extra: todo!(),
                    coinbase: todo!(),
                },
            );
            // a memory database which store the mpt nodes and codes
            let db = Database::new(100000);
            // state fetcher, fetch the states on demand.
            let fetcher = mpt::BlockStateFetcher::new(el.clone(), prev_block.number.into());
            // world state trie, use prev_block's state_root
            let trie = mpt::TrieState::new(fetcher, prev_block.state_root, db);
            let hash_getter = BuilderFetcher::new(el.as_ref().clone());
            BlockBuilder::new(engine, trie, hash_getter, header).unwrap()
        };
        let starting_state = builder.flush_state().unwrap();
        GreedyBlockBuildingStrategy {
            builder,
            all_bundles: BTreeMap::new(),
            ordered_bundle_ids: Vec::new(),
            proposer_requested_txns: Vec::new(),
            block: Vec::new(),
            inclusion_list: Vec::new(),
            state_root: starting_state,
        }
    }

    fn add_bundle(&mut self, bundle_id: String, bundle: WrappedBundle) {
        self.add_bundle_id(&bundle_id, bundle.value());
        self.rebuild()
    }

    fn remove_bundle(&mut self, bundle_id: &String) -> bool {
        self.all_bundles.remove(bundle_id);
        for i in 0..self.ordered_bundle_ids.len() {
            if self.ordered_bundle_ids[i] == bundle_id.to_string() {
                self.ordered_bundle_ids.remove(i);
                self.rebuild(); // TODO: we can rebuild from the index of the removed bundle
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

    fn create_bundle(&self, transactions: Vec<Transaction>) -> Result<WrappedBundle, MevBooTEEError> {
        todo!()
    }
}

fn remove_common_txns(bundle: &WrappedBundle, inclusion_list: &mut Vec<Transaction>) {
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

#[derive(Clone)]
pub struct BuilderFetcher {
    client: ExecutionClient<Arc<MixRpcClient>>,
    cache: Arc<Mutex<BTreeMap<u64, SH256>>>,
}

impl BuilderFetcher {
    pub fn new(client: ExecutionClient<Arc<MixRpcClient>>) -> Self {
        Self {
            client,
            cache: Default::default(),
        }
    }
}

impl BlockHashGetter for BuilderFetcher {
    fn get_hash(&self, current: u64, target: u64) -> SH256 {
        if target >= current || target < current.saturating_sub(256) {
            return Default::default();
        }
        {
            let cache = self.cache.lock().unwrap();
            if let Some(hash) = cache.get(&target) {
                return *hash;
            }
        }
        match self.client.get_block_header(target.into()) {
            Ok(header) => {
                let hash = header.hash();
                let mut cache = self.cache.lock().unwrap();
                cache.insert(target, hash);
                hash
            }
            Err(err) => Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_bundle() {
        let builder = GreedyBlockBuildingStrategy::new();
    }
}
