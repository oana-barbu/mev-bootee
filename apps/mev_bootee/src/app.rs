use std::{prelude::v1::*, sync::Mutex};

use apps::AppEnv;
use base::trace::Alive;

use jsonrpc::{RpcServer, JsonrpcErrorObj, RpcServerConfig};
use std::sync::mpsc::{Sender, channel, Receiver};
use eth_types::Block;
use eth_tools::{ExecutionClient, MixRpcClient};

use std::sync::Arc;

use crate::{BlockBuildingStrategy, MevBooTEEAPI, JsonRpcServerMsg, SubmitBundleRequest, BlockHeaderOffer, SignedHeader, GetBlockOfferRequest, PartialBlockBuildingMode, SignedPartialBlockHeader};

type Validator = String; // TODO

pub struct MevBooTEE<T: BlockBuildingStrategy> {
    pub alive: Alive,
    el: Arc<ExecutionClient<Arc<MixRpcClient>>>,
    pub srv_receiver: Mutex<Receiver<JsonRpcServerMsg>>,
    pub srv_sender: Arc<Mutex<Sender<JsonRpcServerMsg>>>,
    pub mode: PartialBlockBuildingMode,
    phantom: Option<std::marker::PhantomData<T>>

}

pub struct RoundEnv<T: BlockBuildingStrategy> {
    pub base_fee: u32,
    pub proposer: Validator,
    pub builder: T
}

impl<T: BlockBuildingStrategy> MevBooTEE<T> {
    pub fn new(mode: PartialBlockBuildingMode) -> Self {
        let (sender, receiver) = channel();
        let alive = Alive::new();
        let el = generate_el(&alive);
        Self {
            el,
            alive: alive,
            srv_receiver: Mutex::new(receiver),
            srv_sender: Arc::new(Mutex::new(sender)),
            mode: mode,
            phantom: None
        }
    }

    fn init_round(&self, block_number: u64) -> RoundEnv<T> {
        let builder = BlockBuildingStrategy::new(self.el.clone(), block_number);
        RoundEnv {
            base_fee: todo!(),
            proposer: todo!(),
            builder,
        }
    }

    fn build_round(&self, alive: Alive) {
        let mut round_env = self.init_round(123456);
        while alive.is_alive() {
            let msg = self.srv_receiver.lock().unwrap().recv();
            match msg {
                Ok(msg) => {
                    match msg {
                        JsonRpcServerMsg::SubmitBundle(bundle, sender) => self.handle_submit_bundle_request(bundle, sender, &mut round_env),
                        JsonRpcServerMsg::CancelBundle(bundle_id, sender) => self.handle_cancel_bundle_request(&bundle_id, sender, &mut round_env),
                        JsonRpcServerMsg::GetBlockOffer(request, sender) => self.handle_get_block_offer(request, sender, &mut round_env),
                        JsonRpcServerMsg::SubmitSignedHeader(signed_header, sender) => self.handle_submit_signed_header(signed_header, sender, &round_env),
                        JsonRpcServerMsg::SubmitSignedPartialBlockHeader(signed_partial_block_header, sender) => self.handle_submit_signed_partial_block_header(signed_partial_block_header, sender, &round_env),
                    }
                },
                Err(e) => glog::error!("Error reading message from server: {:?}", e),
            }
        }
    }

    fn get_round_deadline(&self) -> Alive {
        todo!()
    }

    pub fn start(&self) {
        glog::info!("running MEV-BooTEE");

        let rpc_srv_handle = base::thread::spawn("jsonrpc-server".into(), {
            let mut cfg = RpcServerConfig::default();
            cfg.listen_addr = "0.0.0.0:1234".into();
            let context = Arc::new(MevBooTEEAPI{sender: self.srv_sender.clone()});
            let mut srv = RpcServer::<MevBooTEEAPI>::new(self.alive.clone(), cfg, context).unwrap();
            srv.jsonrpc("submit_bundle", MevBooTEEAPI::submit_bundle);
            srv.jsonrpc("cancel_bundle", MevBooTEEAPI::cancel_bundle);
            srv.jsonrpc("get_block_offer", MevBooTEEAPI::get_block_offer);
            match self.mode {
                PartialBlockBuildingMode::BuilderProposes => {
                    // proposer signs the header and lets the builder propose the block
                    srv.jsonrpc("submit_signed_header", MevBooTEEAPI::submit_signed_header);
                },
                PartialBlockBuildingMode::ProposerProposes => {
                    // proposer commits to the partial block built by BooTEE
                    srv.jsonrpc("commit_to_partial_block", MevBooTEEAPI::commit_to_partial_block);
                },
                PartialBlockBuildingMode::ProposerChooses => {
                    // the proposer can choose whether to sign the header and let the builder propose
                    // or commit to the partial block built by the builder
                    srv.jsonrpc("submit_signed_header", MevBooTEEAPI::submit_signed_header);
                    srv.jsonrpc("commit_to_partial_block", MevBooTEEAPI::commit_to_partial_block);
                },
            }
            
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

    fn handle_submit_bundle_request(&self, bundle_request: SubmitBundleRequest, sender: Sender<Result<String, JsonrpcErrorObj>>, env: &mut RoundEnv<T>) {
        if !bundle_request.verify(env) {
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad request: bundle param invalid".into()))) {
                glog::error!("unable to send on channel back: {:?}", e);
            }
            return;
        }
        let transactions = bundle_request.into_transactions();
        let bundle = match env.builder.create_bundle(transactions) {
            Ok(bundle) => bundle,
            Err(e) => {
                if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad request: invalid bundle".into()))) {
                    glog::error!("unable to send on channel back: {:?}", e);
                }
                return
            }
        };
        let mut random = [0_u8; 32];
        crypto::read_rand(&mut random);
        let bundle_id = std::str::from_utf8(&random[..]).unwrap();
        if let Err(e) = sender.send(Ok(bundle_id.into())) {
            glog::error!("unable to send bundle_id back: {:?}", e);
            return; // the request cannot be completed so it's pointless to add the bundle
        }
        env.builder.add_bundle(bundle_id.into(), bundle)
    }

    fn handle_cancel_bundle_request(&self, bundle_id: &String, sender: Sender<bool>, env: &mut RoundEnv<T>) {
        let removed = env.builder.remove_bundle(bundle_id);
        if let Err(e) = sender.send(removed) {
            glog::error!("unable to send on channel back: {:?}", e);
        }
    }

    fn handle_get_block_offer(&self, request: GetBlockOfferRequest, sender: Sender<Result<BlockHeaderOffer, JsonrpcErrorObj>>, env: &mut RoundEnv<T>) {
        if !request.verify(env) {
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad request: get block offer request invalid".into()))) {
                glog::error!("unable to send on channel back: {:?}", e);
            }
            return;
        }
        let transactions = request.into_transactions();
        match env.builder.create_bundle(transactions.clone()) {
            Ok(_) => {},
            Err(e) => {
                if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad request: invalid bundle".into()))) {
                    glog::error!("unable to send on channel back: {:?}", e);
                }
                return
            }
        };
        env.builder.add_inclusion_list(transactions);
        let header = env.builder.get_block_header();
        let offer = BlockHeaderOffer::new(&header);
        if let Err(e) = sender.send(Ok(offer)) {
            glog::error!("unable to send offer: {:?}", e);
        }
    }

    fn handle_submit_signed_header(&self, signed_header: SignedHeader, sender: Sender<Result<bool, JsonrpcErrorObj>>, env: &RoundEnv<T>) {
        // we release the block ourselves
        if let Err(e) = signed_header.verify(env) {
            let msg = format!("{:?}", e);
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client(msg))) {
                glog::error!("unable to send response on channel: {:?}", e);
            }
            return;
        }
        let ret = self.release_block();
        if let Err(e) = sender.send(Ok(ret)) {
            glog::error!("unable to send on channel: {:?}", e);
        }
    }

    fn handle_submit_signed_partial_block_header(&self, signed_partial_block_header: SignedPartialBlockHeader, sender: Sender<Result<Block, JsonrpcErrorObj>>, env: &RoundEnv<T>) {
        // we return block to proposer
        if let Err(e) = signed_partial_block_header.verify(env) {
            let msg = format!("{:?}", e);
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client(msg))) {
                glog::error!("unable to send on channel: {:?}", e);
            }
            return;
        }
        let block = self.get_block();
        if let Err(e) = sender.send(Ok(block)) {
            glog::error!("unable to send on channel: {:?}", e);
        }
    }

    fn release_block(&self) -> bool {
        todo!()
    }

    fn get_block(&self) -> Block {
        todo!()
    }
}

impl<T: BlockBuildingStrategy> apps::App for MevBooTEE<T> {
    fn run(&self, args: AppEnv) -> Result<(), String> {
        glog::info!("running app");
        self.start();
        Ok(())
    }

    fn terminate(&self) {
        glog::info!("terminate MevBooTEE");
        self.alive.shutdown();
    }
}

fn generate_el(alive: &Alive) -> Arc<ExecutionClient<Arc<MixRpcClient>>> {
    let mut client = MixRpcClient::new(None);
        client
            .add_endpoint(alive, &["http://localhost:8545".to_owned()])
            .unwrap();
        Arc::new(ExecutionClient::new(Arc::new(client)))
}
