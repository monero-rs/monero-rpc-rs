#![feature(async_await)]

use chrono::prelude::*;
use core::ops::Deref;
use failure::{format_err, Fallible};
use futures::compat::*;
use jsonrpc_core::types::*;
use log::trace;
use monero::{cryptonote::hash::Hash as CryptoNoteHash, Address, PaymentId};
use serde::{de::IgnoredAny, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{self, Display};
use uuid::Uuid;

pub trait HashType: Sized {
    fn bytes(&self) -> &[u8];
    fn from_str(v: &str) -> Fallible<Self>;
}

macro_rules! hash_type {
    ($name:ident, $len:expr) => {
        fixed_hash::construct_fixed_hash! {
            pub struct $name($len);
        }

        impl HashType for $name {
            fn bytes(&self) -> &[u8] {
                self.as_bytes()
            }
            fn from_str(v: &str) -> Fallible<Self> {
                Ok(v.parse()?)
            }
        }
    };
}

hash_type!(BlockHash, 32);
hash_type!(BlockHashingBlob, 76);

impl HashType for PaymentId {
    fn bytes(&self) -> &[u8] {
        self.as_bytes()
    }
    fn from_str(v: &str) -> Fallible<Self> {
        Ok(v.parse()?)
    }
}

impl HashType for CryptoNoteHash {
    fn bytes(&self) -> &[u8] {
        self.as_bytes()
    }
    fn from_str(v: &str) -> Fallible<Self> {
        Ok(v.parse()?)
    }
}

impl HashType for Vec<u8> {
    fn bytes(&self) -> &[u8] {
        &*self
    }
    fn from_str(v: &str) -> Fallible<Self> {
        Ok(hex::decode(v)?)
    }
}

#[derive(Clone, Debug)]
pub struct HashString<T>(pub T);

impl<T> Display for HashString<T>
where
    T: HashType,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.bytes()))
    }
}

impl<'a, T> Serialize for HashString<T>
where
    T: HashType,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, T> Deserialize<'de> for HashString<T>
where
    T: HashType,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Ok(Self(T::from_str(s).map_err(serde::de::Error::custom)?))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Status {
    OK,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockCount {
    pub count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockTemplate {
    pub blockhashing_blob: HashString<BlockHashingBlob>,
    pub blocktemplate_blob: String,
    pub difficulty: u64,
    pub expected_reward: u64,
    pub height: u64,
    pub prev_hash: HashString<BlockHash>,
    pub reserved_offset: u64,
    pub untrusted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum MoneroResult<T> {
    OK(T),
}

impl<T> MoneroResult<T> {
    pub fn into_inner(self) -> T {
        match self {
            MoneroResult::OK(v) => v,
        }
    }
}

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
        T: for<'de> Deserialize<'de>,
    {
        let method = method.to_string();

        let addr = format!("{}/json_rpc", &self.addr);

        let body = serde_json::to_string(&MethodCall {
            jsonrpc: Some(Version::V2),
            method: method.to_string(),
            params,
            id: Id::Str(Uuid::new_v4().to_string()),
        })
        .unwrap();

        trace!("Sending {} to {}", body, &addr);

        let rsp = self
            .client
            .post(&addr)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .compat()
            .await?
            .json::<response::Output>()
            .compat()
            .await?;

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

#[derive(Debug)]
pub struct RegtestDaemonClient(pub DaemonClient);

impl Deref for RegtestDaemonClient {
    type Target = DaemonClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LastBlockHeaderResponse {
    pub block_size: u64,
    pub depth: u64,
    pub difficulty: u64,
    pub hash: HashString<BlockHash>,
    pub height: u64,
    pub major_version: u64,
    pub minor_version: u64,
    pub nonce: u32,
    pub num_txes: u64,
    pub orphan_status: bool,
    pub prev_hash: HashString<BlockHash>,
    pub reward: u128,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
}

impl DaemonClient {
    pub async fn get_block_count(&self) -> Fallible<u64> {
        Ok(self
            .inner
            .request::<MoneroResult<BlockCount>>("get_block_count", Params::Array(vec![]))
            .await?
            .into_inner()
            .count)
    }

    pub async fn on_get_block_hash(&self, height: u64) -> Fallible<BlockHash> {
        self.inner
            .request::<HashString<BlockHash>>(
                "on_get_block_hash",
                Params::Array(vec![height.into()]),
            )
            .await
            .map(|v| v.0)
    }

    pub async fn get_block_template(
        &self,
        wallet_address: Address,
        reserve_size: u64,
    ) -> Fallible<BlockTemplate> {
        Ok(self
            .inner
            .request::<MoneroResult<BlockTemplate>>(
                "get_block_template",
                Params::Array(vec![
                    serde_json::to_value(wallet_address).unwrap(),
                    reserve_size.into(),
                ]),
            )
            .await?
            .into_inner())
    }

    pub async fn submit_block(&self, block_blob_data: String) -> Fallible<String> {
        self.inner
            .request("submit_block", Params::Array(vec![block_blob_data.into()]))
            .await
    }

    pub async fn get_last_block_header(&self) -> Fallible<LastBlockHeaderResponse> {
        self.inner
            .request("get_last_block_header", Params::None)
            .await
    }

    /// Enable additional functions for regtest mode
    pub fn regtest(self) -> RegtestDaemonClient {
        RegtestDaemonClient(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerateBlocksResponse {
    pub height: u64,
}

impl RegtestDaemonClient {
    pub async fn generate_blocks(
        &self,
        amount_of_blocks: u64,
        wallet_address: Address,
    ) -> Fallible<GenerateBlocksResponse> {
        Ok(self
            .inner
            .request::<MoneroResult<GenerateBlocksResponse>>(
                "generateblocks",
                Params::Map(
                    vec![
                        (
                            "amount_of_blocks".to_string(),
                            serde_json::to_value(amount_of_blocks).unwrap(),
                        ),
                        (
                            "wallet_address".to_string(),
                            serde_json::to_value(wallet_address).unwrap(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            )
            .await?
            .into_inner())
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
            other => {
                return Err(serde::de::Error::custom(format!(
                    "Invalid variant {}, expected 0-3",
                    other
                )))
            }
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferData {
    pub amount: u128,
    pub fee: u128,
    pub multisig_txset: Vec<()>,
    pub tx_blob: HashString<Vec<u8>>,
    pub tx_hash: HashString<CryptoNoteHash>,
    pub tx_key: HashString<Vec<u8>>,
    pub tx_metadata: HashString<Vec<u8>>,
    pub unsigned_txset: HashString<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubaddressData {
    pub address: Address,
    pub address_index: u64,
    pub label: String,
    pub used: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubaddressIndex {
    pub major: u64,
    pub minor: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payment {
    pub payment_id: HashString<PaymentId>,
    pub tx_hash: HashString<CryptoNoteHash>,
    pub amount: u64,
    pub block_height: u64,
    pub unlock_time: u64,
    pub subaddr_index: SubaddressIndex,
    pub address: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressData {
    pub address: Address,
    pub addresses: Vec<SubaddressData>,
}

#[derive(Clone, Debug, Default)]
pub struct TransferOptions {
    pub account_index: Option<u64>,
    pub subaddr_indices: Option<Vec<u64>>,
    pub mixin: Option<u64>,
    pub ring_size: Option<u64>,
    pub unlock_time: Option<u64>,
    pub payment_id: Option<PaymentId>,
    pub do_not_relay: Option<bool>,
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

        self.inner.request("get_balance", Params::Array(args)).await
    }

    pub async fn get_address(
        &self,
        account: u64,
        addresses: Option<Vec<u64>>,
    ) -> Fallible<AddressData> {
        let mut args = vec![];
        args.push(("account_index".into(), account.into()));
        if let Some(addresses) = addresses {
            args.push((
                "address_index".into(),
                addresses
                    .into_iter()
                    .map(Value::from)
                    .collect::<Vec<_>>()
                    .into(),
            ));
        }

        self.inner
            .request("get_address", Params::Map(args.into_iter().collect()))
            .await
    }

    pub async fn get_address_index(&self, address: Address) -> Fallible<(u64, u64)> {
        #[derive(Deserialize)]
        struct Rsp {
            index: SubaddressIndex,
        }

        let mut params = Map::new();
        params.insert("address".into(), address.to_string().into());

        let rsp = self
            .inner
            .request::<Rsp>("get_address_index", Params::Map(params.into()))
            .await?;

        Ok((rsp.index.major, rsp.index.minor))
    }

    pub async fn create_address(
        &self,
        account_index: u64,
        label: Option<String>,
    ) -> Fallible<(Address, u64)> {
        #[derive(Deserialize)]
        struct Rsp {
            address: Address,
            address_index: u64,
        }

        let mut params = Map::new();

        params.insert("account_index".into(), Value::Number(account_index.into()));

        if let Some(s) = label {
            params.insert("label".into(), Value::String(s));
        }

        let rsp = self
            .inner
            .request::<Rsp>("create_address", Params::Map(params.into()))
            .await?;

        Ok((rsp.address, rsp.address_index))
    }

    pub async fn label_address(
        &self,
        account_index: u64,
        address_index: u64,
        label: String,
    ) -> Fallible<()> {
        let mut params = Map::new();
        params.insert(
            "index".into(),
            json!(SubaddressIndex {
                major: account_index,
                minor: address_index,
            }),
        );
        params.insert("label".into(), label.into());

        self.inner
            .request::<IgnoredAny>("label_address", Params::Map(params))
            .await?;

        Ok(())
    }

    pub async fn get_payments(&self, payment_id: PaymentId) -> Fallible<Vec<Payment>> {
        let mut params = Map::new();
        params.insert("payment_id".into(), payment_id.to_string().into());

        self.inner
            .request::<Vec<Payment>>("get_payments", Params::Map(params))
            .await
    }

    pub async fn get_bulk_payments(
        &self,
        payment_ids: Vec<PaymentId>,
        min_block_height: u64,
    ) -> Fallible<Vec<Payment>> {
        let mut params = Map::new();
        params.insert(
            "payment_ids".into(),
            json!(payment_ids
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()),
        );
        params.insert("min_block_height".into(), min_block_height.into());

        self.inner
            .request::<Vec<Payment>>("get_bulk_payments", Params::Map(params))
            .await
    }

    pub async fn query_view_key(&self) -> Fallible<monero::PrivateKey> {
        hash_type!(PK, 32);

        #[derive(Deserialize)]
        struct Rsp {
            key: HashString<PK>,
        }

        let rsp = self
            .inner
            .request::<Rsp>(
                "query_key",
                Params::Map(
                    vec![("key_type".into(), Value::String("view_key".into()))]
                        .into_iter()
                        .collect(),
                ),
            )
            .await?;

        Ok(monero::PrivateKey::from_slice(&rsp.key.0.as_bytes())?)
    }

    pub async fn transfer(
        &self,
        destinations: HashMap<Address, u128>,
        priority: TransferPriority,
        options: TransferOptions,
    ) -> Fallible<TransferData> {
        let mut args = serde_json::Map::default();
        args["destinations"] = destinations
            .into_iter()
            .map(|(address, amount)| json!({"address": address, "amount": amount}))
            .collect::<Vec<Value>>()
            .into();
        args["priority"] = serde_json::to_value(priority)?;

        if let Some(account_index) = options.account_index {
            args["account_index"] = account_index.into();
        }

        if let Some(subaddr_indices) = options.subaddr_indices {
            args["subaddr_indices"] = subaddr_indices
                .into_iter()
                .map(From::from)
                .collect::<Vec<Value>>()
                .into();
        }

        if let Some(mixin) = options.mixin {
            args["mixin"] = mixin.into();
        }

        if let Some(ring_size) = options.ring_size {
            args["ring_size"] = ring_size.into();
        }

        if let Some(unlock_time) = options.unlock_time {
            args["unlock_time"] = unlock_time.into();
        }

        if let Some(payment_id) = options.payment_id {
            args["payment_id"] = serde_json::to_value(HashString(payment_id))?;
        }

        if let Some(do_not_relay) = options.do_not_relay {
            args["do_not_relay"] = do_not_relay.into();
        }

        args["get_tx_key"] = true.into();
        args["get_tx_hex"] = true.into();
        args["get_tx_metadata"] = true.into();

        self.inner.request("transfer", Params::Map(args)).await
    }

    pub async fn get_version(&self) -> Fallible<(u16, u16)> {
        #[derive(Deserialize)]
        struct Version {
            version: u32,
        }

        let version: Version = self.inner.request("get_version", Params::None).await?;

        let major = version.version >> 16;
        let minor = version.version - (major << 16);

        Ok((u16::try_from(major)?, u16::try_from(minor)?))
    }
}
