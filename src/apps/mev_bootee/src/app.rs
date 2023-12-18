use std::{prelude::v1::*, sync::Mutex};

use apps::AppEnv;
use base::trace::Alive;

use jsonrpc::{RpcServer, JsonrpcErrorObj, RpcServerConfig};
use std::sync::mpsc::{Sender, channel, Receiver};

use std::sync::Arc;

use crate::{OrderFlow, MevBooTEEAPI, JsonRpcServerMsg, SubmitBundleRequest, BlockHeaderOffer, SignedHeader, GetBlockOfferRequest, PartialBlockBuildingMode};

type Validator = String; // TODO

pub struct MevBooTEE<T: OrderFlow> {
    pub alive: Alive,
    pub srv_receiver: Mutex<Receiver<JsonRpcServerMsg>>,
    pub srv_sender: Arc<Mutex<Sender<JsonRpcServerMsg>>>,
    pub order_flow: Mutex<T>,
    pub mode: PartialBlockBuildingMode
}

pub struct RoundEnv {
    pub base_fee: u32,
    pub proposer: Validator,
}

impl<T: OrderFlow> MevBooTEE<T> {
    pub fn new(mode: PartialBlockBuildingMode) -> Self {
        let (sender, receiver) = channel();
        let es = todo!();
        Self {
            alive: Alive::new(),
            srv_receiver: Mutex::new(receiver),
            srv_sender: Arc::new(Mutex::new(sender)),
            order_flow: Mutex::new(T::new(es)),
            mode: mode,
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
                        JsonRpcServerMsg::CancelBundle(bundle_id, sender) => self.handle_cancel_bundle_request(&bundle_id, sender),
                        JsonRpcServerMsg::GetBlockOffer(request, sender) => self.handle_get_block_offer(request, sender, &round_env),
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

    fn handle_submit_bundle_request(&self, bundle_request: SubmitBundleRequest, sender: Sender<Result<String, JsonrpcErrorObj>>, env: &RoundEnv) {
        if !bundle_request.verify(env) {
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad request: bundle param invalid".into()))) {
                glog::error!("unable to send on channel back: {:?}", e);
            }
            return;
        }
        let transactions = bundle_request.into_tarnsactions();
        let bundle = match self.order_flow.lock().unwrap().create_bundle(transactions) {
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
        self.order_flow.lock().unwrap().add_bundle(bundle_id.into(), bundle)
    }

    fn handle_cancel_bundle_request(&self, bundle_id: &String, sender: Sender<bool>) {
        let removed = self.order_flow.lock().unwrap().remove_bundle(bundle_id);
        if let Err(e) = sender.send(removed) {
            glog::error!("unable to send on channel back: {:?}", e);
        }
    }

    fn handle_get_block_offer(&self, request: GetBlockOfferRequest, sender: Sender<Result<BlockHeaderOffer, JsonrpcErrorObj>>, env: &RoundEnv) {
        if !request.verify(env) {
            if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad request: get block offer request invalid".into()))) {
                glog::error!("unable to send on channel back: {:?}", e);
            }
            return;
        }
        let transactions = request.into_transactions();
        match self.order_flow.lock().unwrap().create_bundle(transactions.clone()) {
            Ok(_) => {},
            Err(e) => {
                if let Err(e) = sender.send(Err(JsonrpcErrorObj::client("Bad request: invalid bundle".into()))) {
                    glog::error!("unable to send on channel back: {:?}", e);
                }
                return
            }
        };
        self.order_flow.lock().unwrap().add_inclusion_list(transactions);
        let header = self.order_flow.lock().unwrap().get_block_header();
        let offer = BlockHeaderOffer::new(&header);
        if let Err(e) = sender.send(Ok(offer)) {
            glog::error!("unable to send offer: {:?}", e);
        }
    }

    fn handle_submit_signed_header(&self, signed_header: SignedHeader, sender: Sender<bool>) {
        // here, we either return the block to the requester, or we release the block ourselves
        todo!()
    }
}

impl<T: OrderFlow> apps::App for MevBooTEE<T> {
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
