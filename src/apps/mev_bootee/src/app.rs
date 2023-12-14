use std::{prelude::v1::*, sync::Mutex};

use apps::AppEnv;
use base::trace::Alive;

use eth_types::SH160;

use jsonrpc::{RpcArgs, RpcServer, JsonrpcErrorObj, RpcServerConfig};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Sender, channel, Receiver};

use std::sync::Arc;
use std::collections::{BTreeMap, BTreeSet};

type Validator = String; // TODO

const MAX_GAS_COST: u32 = 3_000_000;

// state after executing a bundle
#[derive(Clone)]
pub struct ExecutionState {

}

pub trait OrderFlow {
    fn add_bundle(&mut self, bundle_id: String, bundle: Bundle);
    fn remove_bundle(&mut self, bundle_id: String);
    fn add_inclusion_list(&mut self, inclusion_list: Vec<Transaction>);
}

pub struct GreedyOrderFlow {
    pub starting_state: ExecutionState,
    pub ordered_bundle_ids: Vec<String>,
    pub inclusion_list: Vec<Transaction>,
    pub all_bundles: BTreeMap<String, Bundle>,
    pub block: Vec<String>
}

impl GreedyOrderFlow {
    // to be run after we add new bundles to ensure the block is still valid
    fn rebuild(&mut self) {
        self.block = Vec::new();
        if self.ordered_bundle_ids.len() == 0 {
            return
        }
        // the tob is fixed
        let tob = self.ordered_bundle_ids[0].clone();
        
        let mut state = self.starting_state.clone();
        self.execute_bundle(&tob, &mut state);
        self.block.push(tob);
        for b in self.ordered_bundle_ids[1..].into_iter() {
            if self.execute_bundle(b, &mut state) {
                self.block.push(b.clone());
            }
        }
        
    }

    // return true if bundle is valid with the current state
    // if bundle conflicts, it restores state to what it was
    fn execute_bundle(&self, bundle_id: &String, execution_state: &mut ExecutionState) -> bool {
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
    fn add_bundle(&mut self, bundle_id: String, bundle: Bundle) {
        self.add_bundle_id(&bundle_id, bundle.value());
        self.rebuild()
    }

    fn remove_bundle(&mut self, bundle_id: String) {
        self.all_bundles.remove(&bundle_id);
        for i in 0..self.ordered_bundle_ids.len() {
            if self.ordered_bundle_ids[i] == bundle_id {
                self.ordered_bundle_ids.remove(i);
                return
            }
        }
    }

    fn add_inclusion_list(&mut self, inclusion_list: Vec<Transaction>) {
        todo!()
    }
}

#[derive(Clone, Default)]
pub struct Transaction {
    pub raw: String,
    pub estimated_gas_cost: u32,
}

#[derive(Clone)]
pub struct Bundle {
    pub searcher: SH160,
    pub bid: Bid,
    pub txns: Vec<Transaction>,
    pub estimated_tip: u32,
    pub estimated_gas_cost: u32
}

impl Bundle {
    // returns the value of the bundle (probably the estimated_tip in this case)
    fn value(&self) -> u32 {
        todo!()
    }

    fn cost(&self) -> u32 {
        return self.estimated_gas_cost
    }
}

pub struct Block {
    pub tob: Bundle,
    pub rob: Vec<Bundle>,
    pub inclusion_list: Vec<Transaction>
}

pub enum JsonRpcServerMsg {
    SubmitBundle(SubmitBundleRequest, Sender<String>),
    CancelBundle(String, Sender<bool>),
    GetBlockOffer(Vec<String>, Sender<BlockHeaderOffer>),
    SubmitSignedHeader(SignedHeader, Sender<bool>)
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

#[derive(Deserialize)]
pub struct SubmitBundleRequest {
    pub txns: Vec<String>,
    pub bid: Bid
}

pub struct MevBooTEEAPI {
    pub sender: Arc<Mutex<Sender<JsonRpcServerMsg>>>
}

#[derive(Serialize)]
pub struct BlockHeaderOffer {

}

#[derive(Deserialize)]
pub struct SignedHeader {

}

impl MevBooTEEAPI {
    pub fn submit_bundle(&self, args: RpcArgs<SubmitBundleRequest>) -> Result<String, JsonrpcErrorObj> {
        let req = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::SubmitBundle(req, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        let bundle_id = receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        Ok(bundle_id)
    }

    pub fn cancel_bundle(&self, args: RpcArgs<String>) -> Result<bool, JsonrpcErrorObj> {
        let bundle_id = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::CancelBundle(bundle_id, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        let removed = receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        Ok(removed)
    }

    pub fn get_block_offer(&self, args: RpcArgs<Vec<String>>) -> Result<BlockHeaderOffer, JsonrpcErrorObj> {
        // TODO: here, the argument could be signed and we could verify if the submitter is indeed the block proposer
        let transactions = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::GetBlockOffer(transactions, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        let header = receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        Ok(header)
    }

    pub fn submit_signed_header(&self, args: RpcArgs<SignedHeader>) -> Result<bool, JsonrpcErrorObj> {
        let signed_header = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::SubmitSignedHeader(signed_header, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        let header = receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        Ok(header)
    }
}

pub struct MevBooTEE {
    pub alive: Alive,
    pub srv_receiver: Mutex<Receiver<JsonRpcServerMsg>>,
    pub srv_sender: Arc<Mutex<Sender<JsonRpcServerMsg>>>
}

pub struct RoundEnv {
    pub base_fee: u32,
    pub proposer: Validator,
}

pub struct RoundStore {
    pub bundles: Vec<Bundle>,
    pub inclusion_list: Vec<Transaction>,
    pub block: Block,
}

impl MevBooTEE {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            alive: Alive::new(),
            srv_receiver: Mutex::new(receiver),
            srv_sender: Arc::new(Mutex::new(sender)),
        }
    }

    fn init_round(&self) -> RoundEnv {
        todo!()
    }

    fn build_round(&self, alive: Alive) {
        let round_env = self.init_round();
        while alive.is_alive() {
            let msg = self.srv_receiver.lock().unwrap().recv();
            match msg {
                Ok(msg) => {
                    match msg {
                        JsonRpcServerMsg::SubmitBundle(bundle, sender) => self.handle_submit_bundle_request(bundle, sender, &round_env),
                        JsonRpcServerMsg::CancelBundle(bundle_id, sender) => self.handle_cancel_bundle_request(bundle_id, sender),
                        JsonRpcServerMsg::GetBlockOffer(txns, sender) => self.handle_get_block_offer(txns, sender),
                        JsonRpcServerMsg::SubmitSignedHeader(signed_header, sender) => self.handle_submit_signed_header(signed_header, sender),
                    }
                },
                Err(e) => glog::error!("Error reading message from server: {:?}", e),
            }
        }
    }

    fn get_round_deadline(&self) -> Alive {
        todo!()
    }

    pub fn run(&self) {
        glog::info!("running MEV-BooTEE");

        let rpc_srv_handle = base::thread::spawn("jsonrpc-server".into(), {
            let mut cfg = RpcServerConfig::default();
            cfg.listen_addr = "0.0.0.0:1234".into();
            let context = Arc::new(MevBooTEEAPI{sender: self.srv_sender.clone()});
            let mut srv = RpcServer::<MevBooTEEAPI>::new(self.alive.clone(), cfg, context).unwrap();
            srv.jsonrpc("submit_bundle", MevBooTEEAPI::submit_bundle);
            srv.jsonrpc("cancel_bundle", MevBooTEEAPI::cancel_bundle);
            srv.jsonrpc("get_block_offer", MevBooTEEAPI::get_block_offer);
            srv.jsonrpc("submit_signed_header", MevBooTEEAPI::submit_signed_header);
            move || {
                srv.run();
            }
        });

        while self.alive.is_alive() {
            let alive = self.get_round_deadline();
            self.build_round(alive);
        }

        rpc_srv_handle.join().expect("failed to join RPC server");
    }

    fn handle_submit_bundle_request(&self, bundle: SubmitBundleRequest, sender: Sender<String>, env: &RoundEnv) {
        let mut random = [0_u8; 32];
        crypto::read_rand(&mut random);
        let bundle_id = std::str::from_utf8(&random[..]).unwrap();
        if let Err(e) = sender.send(bundle_id.into()) {
            glog::error!("unable to send bundle_id back: {:?}", e);
            return; // the request cannot be completed so it's pointless to add the bundle
        }
        // TODO: add the bundle and build the block
    }

    fn handle_cancel_bundle_request(&self, bundle_id: String, sender: Sender<bool>) {
        todo!()
        // TODO: remove bundle from the current block
    }

    fn handle_get_block_offer(&self, proposer_txns: Vec<String>, sender: Sender<BlockHeaderOffer>) {
        todo!()
    }

    fn handle_submit_signed_header(&self, signed_header: SignedHeader, sender: Sender<bool>) {
        todo!()
    }
}

impl apps::App for MevBooTEE {
    fn run(&self, args: AppEnv) -> Result<(), String> {
        glog::info!("running app");
        self.run();
        Ok(())
    }

    fn terminate(&self) {
        glog::info!("terminate MevBooTEE");
        self.alive.shutdown();
    }
}
