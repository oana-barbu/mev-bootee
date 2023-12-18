use std::prelude::v1::*;

use std::sync::{Arc, mpsc::{channel, Sender}, Mutex};

use serde::{Deserialize, Serialize};
use jsonrpc::{JsonrpcErrorObj, RpcArgs};

use eth_types::{BlockHeader, Block};

use crate::{Bid, RoundEnv, Transaction};

#[derive(Deserialize)]
pub struct SubmitBundleRequest {
    pub txns: Vec<String>,
    pub bid: Bid
}

impl SubmitBundleRequest {
    pub fn verify(&self, env: &RoundEnv) -> bool {
        todo!()
    }

    pub fn into_tarnsactions(&self) -> Vec<Transaction> {
        todo!()
    }
}

#[derive(Deserialize)]
pub struct GetBlockOfferRequest {
    pub txns: Vec<String>,
}

impl GetBlockOfferRequest {
    pub fn verify(&self, env: &RoundEnv) -> bool {
        // verify that the requester is the block proposer for this round
        // + other possible verifications
        todo!()
    }

    pub fn into_transactions(&self) -> Vec<Transaction> {
        todo!()
    }
}

#[derive(Serialize)]
pub struct BlockHeaderOffer {

}

impl BlockHeaderOffer {
    pub fn new(header: &BlockHeader) -> Self {
        todo!()
    }
}

#[derive(Deserialize)]
pub struct SignedHeader {

}

impl SignedHeader {
    pub fn verify(&self, env: &RoundEnv) -> bool {
        // check signature belongs to proposer and is correct
        todo!()
    }
}

#[derive(Deserialize)]
pub struct SignedPartialBlockHeader {

}

impl SignedPartialBlockHeader {
    pub fn verify(&self, env: &RoundEnv) -> bool {
        // check signature belongs to proposer and is correct
        // check proposer is enrolled in EigenLayer and can be slashed
        todo!()
    }
}

pub enum JsonRpcServerMsg {
    SubmitBundle(SubmitBundleRequest, Sender<Result<String, JsonrpcErrorObj>>),
    CancelBundle(String, Sender<bool>),
    GetBlockOffer(GetBlockOfferRequest, Sender<Result<BlockHeaderOffer, JsonrpcErrorObj>>),
    SubmitSignedHeader(SignedHeader, Sender<Result<bool, JsonrpcErrorObj>>),
    SubmitSignedPartialBlockHeader(SignedPartialBlockHeader, Sender<Result<Block, JsonrpcErrorObj>>)
}

pub struct MevBooTEEAPI {
    pub sender: Arc<Mutex<Sender<JsonRpcServerMsg>>>
}

impl MevBooTEEAPI {
    pub fn submit_bundle(&self, args: RpcArgs<SubmitBundleRequest>) -> Result<String, JsonrpcErrorObj> {
        let req = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::SubmitBundle(req, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?
    }

    pub fn cancel_bundle(&self, args: RpcArgs<String>) -> Result<bool, JsonrpcErrorObj> {
        let bundle_id = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::CancelBundle(bundle_id, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        let removed = receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        Ok(removed)
    }

    pub fn get_block_offer(&self, args: RpcArgs<GetBlockOfferRequest>) -> Result<BlockHeaderOffer, JsonrpcErrorObj> {
        // TODO: here, the argument could be signed and we could verify if the submitter is indeed the block proposer
        let request = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::GetBlockOffer(request, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?
    }

    pub fn submit_signed_header(&self, args: RpcArgs<SignedHeader>) -> Result<bool, JsonrpcErrorObj> {
        let signed_header = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::SubmitSignedHeader(signed_header, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?
    }

    pub fn commit_to_partial_block(&self, args: RpcArgs<SignedPartialBlockHeader>) -> Result<Block, JsonrpcErrorObj> {
        let signed_pbh = args.params;
        let (sender, receiver) = channel();
        self.sender.lock().unwrap().send(JsonRpcServerMsg::SubmitSignedPartialBlockHeader(signed_pbh, sender)).map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?;
        receiver.recv().map_err(|_| JsonrpcErrorObj::unknown("unresponsive"))?
    }
}
