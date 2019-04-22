#![feature(async_await, await_macro, futures_api)]

use failure::{format_err, Fallible};
use futures::compat::*;
//use jsonrpc_core::Error;
//use jsonrpc_derive::rpc;
use jsonrpc_core::types::*;
use log::trace;
use monero::{Address, PaymentId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};
use std::collections::HashMap;

macro_rules! hash_type {
    ($name:ident, $len:expr) => {
        fixed_hash::construct_fixed_hash! {
            pub struct $name($len);
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                serializer.serialize_str(&hex::encode(self.as_bytes()))
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                let mut bytes = [0u8; $len];
                let s = String::deserialize(deserializer)?;
                hex::decode_to_slice(s, &mut bytes).map_err(serde::de::Error::custom)?;
                Ok($name(bytes))
            }
        }
    };
}

hash_type!(BlockHash, 32);
hash_type!(BlockHashingBlob, 76);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Status {
    OK,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockCount {
    pub count: u128,
    pub status: Status,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockTemplate {
    pub blockhashing_blob: BlockHashingBlob,
    pub blocktemplate_blob: String,
    pub difficulty: u64,
    pub expected_reward: u64,
    pub height: u64,
    pub prev_hash: BlockHash,
    pub reserved_offset: u64,
    pub status: Status,
    pub untrusted: bool,
}

//#[rpc]
//pub trait Daemon {
//    #[rpc(name = "get_block_count", returns = "BlockCount")]
//    fn get_block_count(&self) -> Result<BlockCount, Error>;
//    #[rpc(name = "on_get_block_hash", returns = "H256")]
//    fn on_get_block_hash(&self, height: u64) -> Result<H256, Error>;
//}

#[derive(Debug)]
pub struct RpcClient {
    client: reqwest::r#async::Client,
    addr: String,
}

impl RpcClient {
    pub fn new(addr: String) -> Self {
        Self {
            client: reqwest::r#async::Client::new(),
            addr,
        }
    }

    async fn request<T>(&self, method: &'static str, params: Params) -> Fallible<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        let addr = format!("{}/json_rpc", &self.addr);

        let body = serde_json::to_string(&MethodCall {
            jsonrpc: Some(Version::V2),
            method: method.to_string(),
            params,
            id: Id::Null,
        })
        .unwrap();

        trace!("Sending {} to {}", body, &addr);

        let rsp = await!(await!(self
            .client
            .post(&addr)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .compat())?
        .json::<response::Output>()
        .compat())?;

        let v = jsonrpc_core::Result::<Value>::from(rsp)
            .map_err(|e| format_err!("Code: {:?}, Message: {}", e.code, e.message))?;

        Ok(serde_json::from_value(v)?)
    }

    pub fn daemon(self) -> DaemonClient {
        DaemonClient { inner: self }
    }

    pub fn wallet(self) -> WalletClient {
        WalletClient { inner: self }
    }
}

#[derive(Debug)]
pub struct DaemonClient {
    inner: RpcClient,
}

impl DaemonClient {
    pub async fn get_block_count(&self) -> Fallible<BlockCount> {
        await!(self.inner.request("get_block_count", Params::Array(vec![])))
    }

    pub async fn on_get_block_hash(&self, height: u64) -> Fallible<BlockHash> {
        await!(self
            .inner
            .request("on_get_block_hash", Params::Array(vec![height.into()])))
    }

    pub async fn get_block_template(
        &self,
        wallet_address: Address,
        reserve_size: u64,
    ) -> Fallible<BlockTemplate> {
        await!(self.inner.request(
            "get_block_template",
            Params::Array(vec![
                serde_json::to_value(wallet_address).unwrap(),
                reserve_size.into()
            ])
        ))
    }

    pub async fn submit_block(&self, block_blob_data: String) -> Fallible<String> {
        await!(self
            .inner
            .request("submit_block", Params::Array(vec![block_blob_data.into()])))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubaddressBalanceData {
    pub address: Address,
    pub address_index: u64,
    pub balance: u64,
    pub label: String,
    pub num_unspent_outputs: u64,
    pub unlocked_balance: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceData {
    pub balance: u64,
    pub multisig_import_needed: bool,
    pub per_subaddress: Vec<SubaddressBalanceData>,
    pub unlocked_balance: u64,
}

#[derive(Copy, Clone, Debug)]
pub enum TransferPriority {
    Default,
    Unimportant,
    Elevated,
    Priority,
}

impl Serialize for TransferPriority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(match self {
            TransferPriority::Default => 0,
            TransferPriority::Unimportant => 1,
            TransferPriority::Elevated => 2,
            TransferPriority::Priority => 3,
        })
    }
}

impl<'de> Deserialize<'de> for TransferPriority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = u8::deserialize(deserializer)?;
        Ok(match v {
            0 => TransferPriority::Default,
            1 => TransferPriority::Unimportant,
            2 => TransferPriority::Elevated,
            3 => TransferPriority::Priority,
            other => return Err(serde::de::Error::custom("Invalid variant, expected 0-3")),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferData {
    pub amount: u128,
    pub fee: u128,
    pub multisig_txset: Vec<()>,
    pub tx_blob: String,
    pub tx_hash: String,
    pub tx_key: String,
    pub tx_metadata: String,
    pub unsigned_txset: String,
}

#[derive(Debug)]
pub struct WalletClient {
    inner: RpcClient,
}

impl WalletClient {
    pub async fn get_balance(
        &self,
        account: u64,
        addresses: Option<Vec<u64>>,
    ) -> Fallible<BalanceData> {
        let mut args = vec![];
        args.push(account.into());
        if let Some(addresses) = addresses {
            args.push(addresses.into());
        }

        await!(self.inner.request("get_balance", Params::Array(args)))
    }

    pub async fn get_address(&self, account: u64, addresses: Option<Vec<u64>>) -> Fallible<()> {
        let mut args = vec![];
        args.push(account.into());
        if let Some(addresses) = addresses {
            args.push(addresses.into());
        }

        await!(self.inner.request("get_address", Params::Array(args)))
    }

    pub async fn transfer(
        &self,
        destinations: HashMap<Address, u128>,
        account_index: Option<u64>,
        subaddr_indices: Option<Vec<u64>>,
        priority: TransferPriority,
        mixin: Option<u64>,
        ring_size: Option<u64>,
        unlock_time: Option<u64>,
        payment_id: Option<PaymentId>,
        do_not_relay: Option<bool>,
    ) -> Fallible<TransferData> {
        let mut args = serde_json::Map::default();
        args["destinations"] = destinations
            .into_iter()
            .map(|(address, amount)| json!({"address": address, "amount": amount}))
            .collect::<Vec<Value>>()
            .into();
        args["priority"] = serde_json::to_value(priority).unwrap();

        if let Some(account_index) = account_index {
            args["account_index"] = account_index.into();
        }

        if let Some(subaddr_indices) = subaddr_indices {
            args["subaddr_indices"] = subaddr_indices
                .into_iter()
                .map(|v| v.into())
                .collect::<Vec<Value>>()
                .into();
        }

        if let Some(mixin) = mixin {
            args["mixin"] = mixin.into();
        }

        if let Some(ring_size) = ring_size {
            args["ring_size"] = ring_size.into();
        }

        if let Some(unlock_time) = unlock_time {
            args["unlock_time"] = unlock_time.into();
        }

        if let Some(payment_id) = payment_id {
            args["payment_id"] = Value::String(hex::encode(payment_id.as_bytes()));
        }

        if let Some(do_not_relay) = do_not_relay {
            args["do_not_relay"] = do_not_relay.into();
        }

        await!(self.inner.request("transfer", Params::Map(args)))
    }
}
