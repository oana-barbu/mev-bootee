use std::{prelude::v1::*, sync::Mutex};

use apps::AppEnv;
use base::trace::Alive;

use jsonrpc::{RpcServer, JsonrpcErrorObj, RpcServerConfig};
use std::sync::mpsc::{Sender, channel, Receiver, TryRecvError};
use eth_types::{Block, BlockHeader, Transaction, SH256};
use eth_tools::{ExecutionClient, MixRpcClient};

use std::sync::Arc;
use std::collections::BTreeMap;
use crate::{GetBidRequest, MevBooTeeMode, SignedHeader};

use crate::{MevBooTeeAPI, JsonRpcServerMsg, SubmitToBRequest};

pub struct MevBooTee {
    pub alive: Alive,
    el: Arc<ExecutionClient<Arc<MixRpcClient>>>,
    pub srv_receiver: Mutex<Receiver<JsonRpcServerMsg>>,
    pub srv_sender: Arc<Mutex<Sender<JsonRpcServerMsg>>>,
    state: Mutex<State>,
    pub mode: MevBooTeeMode,
    pub do_verification: bool
}

impl Default for MevBooTee {
    fn default() -> Self {
        let (sender, receiver) = channel();
        let alive = Alive::new();
        let mut client = MixRpcClient::new(None);
        client
            .add_endpoint(&alive, &["http://localhost:8545".to_owned()])
            .unwrap();
        let el: Arc<ExecutionClient<Arc<MixRpcClient>>> = Arc::new(ExecutionClient::new(Arc::new(client)));
        Self {
            alive,
            srv_receiver: Mutex::new(receiver),
            srv_sender: Arc::new(Mutex::new(sender)),
            state: Mutex::new(State::default()),
            mode: MevBooTeeMode::Assembler,
            do_verification: false,
            el,
        }
    }
}

impl MevBooTee {
    pub fn config(&mut self, mode: MevBooTeeMode, do_verification: bool) {
        self.mode = mode;
        self.do_verification = do_verification;
    }

    fn run(&self) {
        while self.alive.is_alive() {
            let msg = self.srv_receiver.lock().unwrap().try_recv();
            match msg {
                Ok(msg) => {
                    match msg {
                        JsonRpcServerMsg::SubmitToB(req, sender) => self.handle_submit_tob_request(req, sender),
                        JsonRpcServerMsg::RetractToB(req, sender) => self.handle_retract_tob_request(&req, sender),
                        JsonRpcServerMsg::GetBid(req, sender) => self.handle_get_bid_request(req, sender),
                        JsonRpcServerMsg::CommitHeader(signed_header, sender) => self.handle_commit_header_request(&signed_header, sender),
                    }
                },
                Err(e) =>
                    if e != TryRecvError::Empty {
                        glog::error!("Error reading message from server: {:?}", e)
                    },
            }
        }
    }

    pub fn start(&self) {
        glog::info!("running MEV-BooTEE");

        let rpc_srv_handle = base::thread::spawn("jsonrpc-server".into(), {
            let mut cfg = RpcServerConfig::default();
            cfg.listen_addr = "0.0.0.0:1234".into();
            let context = Arc::new(MevBooTeeAPI{sender: self.srv_sender.clone()});
            let mut srv = RpcServer::<MevBooTeeAPI>::new(self.alive.clone(), cfg, context).unwrap();
            match self.mode {
                MevBooTeeMode::ProposerAide => todo!(),
                MevBooTeeMode::BuilderAide => todo!(),
                MevBooTeeMode::Assembler => {
                    srv.jsonrpc("echo", MevBooTeeAPI::echo);
                    srv.jsonrpc("submit_tob", MevBooTeeAPI::submit_tob);
                    srv.jsonrpc("retract_tob", MevBooTeeAPI::retract_tob);
                    srv.jsonrpc("get_highest_bid", MevBooTeeAPI::get_highest_bid);
                    srv.jsonrpc("commit_header", MevBooTeeAPI::commit_header);
                },
                MevBooTeeMode::FullTeeBuilder => todo!(),
            }
            
            move || {
                srv.run();
            }
        });

        self.run();

        rpc_srv_handle.join().expect("failed to join RPC server");
    }

    fn handle_submit_tob_request(&self, tob_request: SubmitToBRequest, sender: Sender<Result<String, JsonrpcErrorObj>>) {
        if self.do_verification {
            if !tob_request.verify() {
                if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad request: ToB is invalid".into()))) {
                    glog::error!("unable to send back on channel: {:?}", e);
                }
                return;
            }
        }

        let mut random = [0_u8; 32];
        crypto::read_rand(&mut random);
        let tob_id = std::str::from_utf8(&random[..]).unwrap();

        let mut state = self.state.lock().unwrap();
        state.tobs.insert(tob_id.into(), (tob_request.bid, tob_request.txns));
        if let Err(e) = sender.send(Ok(tob_id.into())) {
            glog::error!("unable to send tob_id back: {:?}", e);
        }
    }

    fn handle_retract_tob_request(&self, tob_id: &String, sender: Sender<bool>) {
        let removed = match self.state.lock().unwrap().tobs.remove(tob_id) {
            Some(_) => true,
            None => false,
        };
        if let Err(e) = sender.send(removed) {
            glog::error!("unable to send on channel back: {:?}", e);
        }
    }

    fn handle_get_bid_request(&self, get_bid_request: GetBidRequest, sender: Sender<Result<(u32, BlockHeader), JsonrpcErrorObj>>) {
        if !get_bid_request.validate_sender() {
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad sender".into()))) {
                glog::error!("unable to send back on channel: {:?}", e);
            }
            return;
        }

        let mut rob = get_bid_request.txn_list.clone();
        let tobs = self.state.lock().unwrap().tobs.clone();
        let (tob_id, (bid, tob_txns)) = tobs.iter().max_by_key(|f| f.1.0).unwrap();
        let mut block = tob_txns.clone();
        if self.do_verification {
            // prepare execution (although, tob is already verified so maybe we can optimize this step)
            todo!();
        }

        for txn in rob {
            if !block.contains(&txn) {
                if self.do_verification {
                    todo!();
                    // skip this txn if the execution fails
                    // if txn is correct, update bid to take into account this txn
                }
                block.push(txn);
            }
        }

        let (bid, header) = todo!();
        if let Err(e) = sender.send(Ok((bid, header))) {
            glog::error!("unable to send back on channel: {:?}", e);
        }
    }

    fn handle_commit_header_request(&self, signed_header: &SignedHeader, sender: Sender<Result<bool, JsonrpcErrorObj>>) {
        if !signed_header.validate_sender() {
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad sender".into()))) {
                glog::error!("unable to send back on channel: {:?}", e);
            }
            return;
        }

        let hash = signed_header.header.hash();

        let state = self.state.lock().unwrap();
        if let Some(block) = self.state.lock().unwrap().blocks.get(&hash) {
            let published = todo!();
            if let Err(e) = sender.send(Ok(published)) {
                glog::error!("unable to send back on channel: {:?}", e);
            }
        } else {
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Unknown header".into()))) {
                glog::error!("unable to send back on channel: {:?}", e);
            }
        }
    }
}

impl apps::App for MevBooTee {
    fn run(&self, args: AppEnv) -> Result<(), String> {
        self.start();
        Ok(())
    }

    fn terminate(&self) {
        glog::info!("terminate MevBooTEE");
        self.alive.shutdown();
    }
}

struct State {
    tobs: BTreeMap<String, (u32, Vec<String>)>,
    blocks: BTreeMap<SH256, Block>
}

impl Default for State {
    fn default() -> Self {
        Self { tobs: BTreeMap::new(), blocks: BTreeMap::new() }
    }
}
