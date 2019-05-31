#![feature(async_await)]

use {
    chrono::prelude::*,
    core::ops::Deref,
    failure::{format_err, Fallible},
    futures::compat::*,
    jsonrpc_core::types::*,
    log::trace,
    monero::{cryptonote::hash::Hash as CryptoNoteHash, Address, PaymentId},
    serde::{de::IgnoredAny, Deserialize, Deserializer, Serialize, Serializer},
    serde_json::{json, Value},
    std::{
        collections::HashMap,
        convert::TryFrom,
        fmt::{self, Display},
        iter::{empty, once},
    },
    uuid::Uuid,
};

pub trait HashType: Sized {
    fn bytes(&self) -> &[u8];
    fn from_str(v: &str) -> Fallible<Self>;
}

macro_rules! hash_type_impl {
    ($name:ident) => {
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

hash_type_impl!(PaymentId);
hash_type_impl!(CryptoNoteHash);

macro_rules! hash_type {
    ($name:ident, $len:expr) => {
        fixed_hash::construct_fixed_hash! {
            pub struct $name($len);
        }

        hash_type_impl!($name);
    };
}

hash_type!(BlockHash, 32);
hash_type!(BlockHashingBlob, 76);

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

enum RpcParams {
    Array(Box<dyn Iterator<Item = Value> + Send>),
    Map(Box<dyn Iterator<Item = (&'static str, Value)> + Send>),
    None,
}

impl RpcParams {
    fn array<A>(v: A) -> Self
    where
        A: Iterator<Item = Value> + Send + 'static,
    {
        RpcParams::Array(Box::new(v))
    }

    fn map<M>(v: M) -> Self
    where
        M: Iterator<Item = (&'static str, Value)> + Send + 'static,
    {
        RpcParams::Map(Box::new(v))
    }
}

impl From<RpcParams> for Params {
    fn from(value: RpcParams) -> Self {
        match value {
            RpcParams::Map(v) => Params::Map(v.map(|(k, v)| (k.to_string(), v)).collect()),
            RpcParams::Array(v) => Params::Array(v.collect()),
            RpcParams::None => Params::None,
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

    async fn request<T>(&self, method: &'static str, params: RpcParams) -> Fallible<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let params = params.into();

        let method = method.to_string();

        let addr = format!("{}/json_rpc", &self.addr);

        let body = serde_json::to_string(&MethodCall {
            jsonrpc: Some(Version::V2),
            method: method.to_string(),
            params,
            id: Id::Str(Uuid::new_v4().to_string()),
        })
        .unwrap();

        trace!("Sending {} to {}", body, addr);

        let rsp = self
            .client
            .post(&addr)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .compat()
            .await?
            .json::<Value>()
            .compat()
            .await?;

        trace!("Received response {} from addr {}", rsp, addr);

        let rsp = serde_json::from_value::<response::Output>(rsp)?;

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
            .request::<MoneroResult<BlockCount>>("get_block_count", RpcParams::array(empty()))
            .await?
            .into_inner()
            .count)
    }

    pub async fn on_get_block_hash(&self, height: u64) -> Fallible<BlockHash> {
        self.inner
            .request::<HashString<BlockHash>>(
                "on_get_block_hash",
                RpcParams::array(once(height.into())),
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
                RpcParams::array(
                    empty()
                        .chain(once(serde_json::to_value(wallet_address).unwrap()))
                        .chain(once(reserve_size.into())),
                ),
            )
            .await?
            .into_inner())
    }

    pub async fn submit_block(&self, block_blob_data: String) -> Fallible<String> {
        self.inner
            .request(
                "submit_block",
                RpcParams::array(once(block_blob_data.into())),
            )
            .await
    }

    pub async fn get_last_block_header(&self) -> Fallible<LastBlockHeaderResponse> {
        #[derive(Deserialize)]
        struct Rsp {
            block_header: LastBlockHeaderResponse,
        }

        self.inner
            .request::<Rsp>("get_last_block_header", RpcParams::None)
            .await
            .map(|rsp| rsp.block_header)
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
        let params = empty()
            .chain(once((
                "amount_of_blocks",
                serde_json::to_value(amount_of_blocks).unwrap(),
            )))
            .chain(once((
                "wallet_address",
                serde_json::to_value(wallet_address).unwrap(),
            )));

        Ok(self
            .inner
            .request::<MoneroResult<GenerateBlocksResponse>>(
                "generateblocks",
                RpcParams::map(params),
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

#[derive(Clone, Debug)]
pub struct SignedTransferOutput {
    pub signed_txset: Vec<u8>,
    pub tx_hash_list: Vec<CryptoNoteHash>,
    pub tx_raw_list: Vec<Vec<u8>>,
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
        let params = empty()
            .chain(once(account.into()))
            .chain(addresses.map(Value::from));

        self.inner
            .request("get_balance", RpcParams::array(params))
            .await
    }

    pub async fn get_address(
        &self,
        account: u64,
        addresses: Option<Vec<u64>>,
    ) -> Fallible<AddressData> {
        let params = empty()
            .chain(once(("account_index", account.into())))
            .chain(addresses.map(|v| {
                (
                    "address_index".into(),
                    v.into_iter().map(Value::from).collect::<Vec<_>>().into(),
                )
            }));

        self.inner
            .request("get_address", RpcParams::map(params))
            .await
    }

    pub async fn get_address_index(&self, address: Address) -> Fallible<(u64, u64)> {
        #[derive(Deserialize)]
        struct Rsp {
            index: SubaddressIndex,
        }

        let params = once(("address", address.to_string().into()));

        let rsp = self
            .inner
            .request::<Rsp>("get_address_index", RpcParams::map(params))
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

        let params = empty()
            .chain(once(("account_index", Value::Number(account_index.into()))))
            .chain(label.map(|v| ("label", Value::String(v))));

        let rsp = self
            .inner
            .request::<Rsp>("create_address", RpcParams::map(params))
            .await?;

        Ok((rsp.address, rsp.address_index))
    }

    pub async fn label_address(
        &self,
        account_index: u64,
        address_index: u64,
        label: String,
    ) -> Fallible<()> {
        let params = empty()
            .chain(once((
                "index".into(),
                json!(SubaddressIndex {
                    major: account_index,
                    minor: address_index,
                }),
            )))
            .chain(once(("label", label.into())));

        self.inner
            .request::<IgnoredAny>("label_address", RpcParams::map(params))
            .await?;

        Ok(())
    }

    pub async fn get_payments(&self, payment_id: PaymentId) -> Fallible<Vec<Payment>> {
        let params = empty().chain(once((
            "payment_id",
            HashString(payment_id).to_string().into(),
        )));

        self.inner
            .request::<Vec<Payment>>("get_payments", RpcParams::map(params))
            .await
    }

    pub async fn get_bulk_payments(
        &self,
        payment_ids: Vec<PaymentId>,
        min_block_height: u64,
    ) -> Fallible<Vec<Payment>> {
        #[derive(Deserialize)]
        struct Rsp {
            #[serde(default = "Default::default")]
            payments: Vec<Payment>,
        }

        let params = empty()
            .chain(once((
                "payment_ids",
                payment_ids
                    .into_iter()
                    .map(|s| HashString(s).to_string())
                    .collect::<Vec<_>>()
                    .into(),
            )))
            .chain(once(("min_block_height", min_block_height.into())));

        self.inner
            .request::<Rsp>("get_bulk_payments", RpcParams::map(params))
            .await
            .map(|rsp| rsp.payments)
    }

    pub async fn query_view_key(&self) -> Fallible<monero::PrivateKey> {
        hash_type!(PK, 32);

        #[derive(Deserialize)]
        struct Rsp {
            key: HashString<PK>,
        }

        let params = empty().chain(once(("key_type", Value::from("view_key"))));

        let rsp = self
            .inner
            .request::<Rsp>("query_key", RpcParams::map(params))
            .await?;

        Ok(monero::PrivateKey::from_slice(&rsp.key.0.as_bytes())?)
    }

    pub async fn transfer(
        &self,
        destinations: HashMap<Address, u128>,
        priority: TransferPriority,
        options: TransferOptions,
    ) -> Fallible<TransferData> {
        let params = empty()
            .chain(once((
                "destinations",
                destinations
                    .into_iter()
                    .map(|(address, amount)| json!({"address": address, "amount": amount}))
                    .collect::<Vec<Value>>()
                    .into(),
            )))
            .chain(once(("priority", serde_json::to_value(priority)?)))
            .chain(options.account_index.map(|v| ("account_index", v.into())))
            .chain(options.subaddr_indices.map(|v| {
                (
                    "subaddr_indices",
                    v.into_iter().map(From::from).collect::<Vec<Value>>().into(),
                )
            }))
            .chain(options.mixin.map(|v| ("mixin", v.into())))
            .chain(options.ring_size.map(|v| ("ring_size", v.into())))
            .chain(options.unlock_time.map(|v| ("unlock_time", v.into())))
            .chain(
                options
                    .payment_id
                    .map(|v| ("payment_id", serde_json::to_value(HashString(v)).unwrap())),
            )
            .chain(options.do_not_relay.map(|v| ("do_not_relay", v.into())))
            .chain(once(("get_tx_key", true.into())))
            .chain(once(("get_tx_hex", true.into())))
            .chain(once(("get_tx_metadata", true.into())));

        self.inner.request("transfer", RpcParams::map(params)).await
    }

    pub async fn sign_transfer(&self, unsigned_txset: Vec<u8>) -> Fallible<SignedTransferOutput> {
        #[derive(Deserialize)]
        struct Rsp {
            signed_txset: HashString<Vec<u8>>,
            tx_hash_list: Vec<HashString<CryptoNoteHash>>,
            tx_raw_list: Vec<HashString<Vec<u8>>>,
        }

        impl From<Rsp> for SignedTransferOutput {
            fn from(value: Rsp) -> Self {
                Self {
                    signed_txset: value.signed_txset.0,
                    tx_hash_list: value.tx_hash_list.into_iter().map(|v| v.0).collect(),
                    tx_raw_list: value.tx_raw_list.into_iter().map(|v| v.0).collect(),
                }
            }
        }

        let params = empty()
            .chain(once((
                "unsigned_txset",
                serde_json::to_value(HashString(unsigned_txset)).unwrap(),
            )))
            .chain(once(("export_raw", true.into())));

        self.inner
            .request::<Rsp>("sign_transfer", RpcParams::map(params))
            .await
            .map(From::from)
    }

    pub async fn submit_transfer(&self, tx_data_hex: Vec<u8>) -> Fallible<Vec<CryptoNoteHash>> {
        #[derive(Deserialize)]
        struct Rsp {
            tx_hash_list: Vec<HashString<CryptoNoteHash>>,
        }

        let params = empty().chain(once((
            "tx_data_hex",
            HashString(tx_data_hex).to_string().into(),
        )));

        self.inner
            .request::<Rsp>("sign_transfer", RpcParams::map(params))
            .await
            .map(|v| v.tx_hash_list.into_iter().map(|v| v.0).collect())
    }

    pub async fn get_version(&self) -> Fallible<(u16, u16)> {
        #[derive(Deserialize)]
        struct Version {
            version: u32,
        }

        let version: Version = self.inner.request("get_version", RpcParams::None).await?;

        let major = version.version >> 16;
        let minor = version.version - (major << 16);

        Ok((u16::try_from(major)?, u16::try_from(minor)?))
    }
}
