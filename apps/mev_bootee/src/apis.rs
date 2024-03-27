use std::prelude::v1::*;

use std::sync::{Arc, mpsc::{channel, Sender}, Mutex};

use serde::{Deserialize, Serialize};
use jsonrpc::{JsonrpcErrorObj, RpcArgs};

use eth_types::{BlockHeader, Block, Transaction};

#[derive(Deserialize)]
pub struct SubmitToBRequest {
    pub txns: Vec<String>,
    pub bid: u32,
    pub block_number: u32
}

impl SubmitToBRequest {
    pub fn verify(&self) -> bool {
        todo!()
    }

    pub fn into_transactions(&self) -> Vec<Transaction> {
        todo!()
    }
}

#[derive(Deserialize)]
pub struct GetBidRequest {
    pub txn_list: Vec<String>,
    pub block_number: u32,
    pub signature: Vec<u8>
}

impl GetBidRequest {
    // check if sender is current proposer
    pub fn validate_sender(&self) -> bool {
        todo!()
    }

    pub fn into_transactions(&self) -> Vec<Transaction> {
        todo!()
    }
}

#[derive(Deserialize)]
pub struct SignedHeader {
    pub header: BlockHeader,
    pub signature: Vec<u8>
}

impl SignedHeader {
    // check if sender is current proposer
    pub fn validate_sender(&self) -> bool {
        todo!()
    }
}

pub enum JsonRpcServerMsg {
    SubmitToB(SubmitToBRequest, Sender<Result<String, JsonrpcErrorObj>>),
    RetractToB(String, Sender<bool>),
    GetBid(GetBidRequest, Sender<Result<(u32, BlockHeader), JsonrpcErrorObj>>),
    CommitHeader(SignedHeader, Sender<Result<bool, JsonrpcErrorObj>>)
}

pub struct MevBooTeeAPI {
    pub sender: Arc<Mutex<Sender<JsonRpcServerMsg>>>
}

impl MevBooTeeAPI {
    pub fn echo(&self, args: RpcArgs<String>) -> Result<String, JsonrpcErrorObj> {
        let req = args.params;
        Ok(req)
    }

    pub fn submit_tob(&self, args: RpcArgs<SubmitToBRequest>) -> Result<String, JsonrpcErrorObj> {
        let req = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::SubmitToB(req, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?
    }

    pub fn retract_tob(&self, args: RpcArgs<String>) -> Result<bool, JsonrpcErrorObj> {
        let tob_id = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::RetractToB(tob_id, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        let removed = receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        Ok(removed)
    }

    pub fn get_highest_bid(&self, args: RpcArgs<GetBidRequest>) -> Result<(u32, BlockHeader), JsonrpcErrorObj> {
        let req = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::GetBid(req, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?
    }


    pub fn commit_header(&self, args: RpcArgs<SignedHeader>) -> Result<bool, JsonrpcErrorObj> {
        let signed_header = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::CommitHeader(signed_header, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?
    }
}
