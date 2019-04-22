#![feature(async_await, await_macro, futures_api)]

use failure::{format_err, Fallible};
use futures::compat::*;
//use jsonrpc_core::Error;
//use jsonrpc_derive::rpc;
use log::trace;
use monero::Address;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
                let mut slice = [0u8; 2 + 2 * $len];
                ethereum_types_serialize::serialize(&mut slice, &self.0, serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                let mut bytes = [0u8; $len];
                ethereum_types_serialize::deserialize_check_len(
                    deserializer,
                    ethereum_types_serialize::ExpectedLen::Exact(&mut bytes),
                )?;
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

    async fn request<I, T>(&self, method: &'static str, params: I) -> Fallible<T>
    where
        I: IntoIterator<Item = Value>,
        for<'de> T: Deserialize<'de>,
    {
        use jsonrpc_core::types::*;

        let addr = format!("{}/json_rpc", &self.addr);

        let body = serde_json::to_string(&MethodCall {
            jsonrpc: Some(Version::V2),
            method: method.to_string(),
            params: Params::Array(params.into_iter().map(Value::from).collect()),
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
        await!(self.inner.request("get_block_count", vec![]))
    }

    pub async fn on_get_block_hash(&self, height: u64) -> Fallible<BlockHash> {
        await!(self.inner.request("on_get_block_hash", vec![height.into()]))
    }

    pub async fn get_block_template(
        &self,
        wallet_address: Address,
        reserve_size: u64,
    ) -> Fallible<BlockTemplate> {
        await!(self.inner.request(
            "get_block_template",
            vec![
                serde_json::to_value(wallet_address).unwrap(),
                reserve_size.into()
            ]
        ))
    }

    pub async fn submit_block(&self, block_blob_data: String) -> Fallible<String> {
        await!(self
            .inner
            .request("submit_block", vec![block_blob_data.into()]))
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferDestination {
    pub amount: u128,
    pub address: Address,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum TransferPriority {
    Default,
    Unimportant,
    Elevated,
    Priority,
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

        await!(self.inner.request("get_balance", args))
    }

    pub async fn get_address(&self, account: u64, addresses: Option<Vec<u64>>) -> Fallible<()> {
        let mut args = vec![];
        args.push(account.into());
        if let Some(addresses) = addresses {
            args.push(addresses.into());
        }

        await!(self.inner.request("get_address", args))
    }

    pub async fn transfer(
        &self,
        destinations: Vec<TransferDestination>,
        account_index: Option<u64>,
        subaddr_indices: Option<Vec<u64>>,
        priority: TransferPriority,
    ) -> Fallible<()> {
        unimplemented!()
    }
}
