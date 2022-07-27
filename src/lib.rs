// Copyright 2019-2022 Artem Vorotnikov and Monero Rust Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Monero daemon and wallet RPC library written in asynchronous Rust.
//!
//! ## Usage
//!
//! Create the base [`RpcClient`] and use the methods [`RpcClient::daemon`],
//! [`RpcClient::daemon_rpc`], or [`RpcClient::wallet`] to retrieve the specialized RPC client.
//!
//! On a [`DaemonJsonRpcClient`] you can call [`DaemonJsonRpcClient::regtest`] to get a [`RegtestDaemonJsonRpcClient`]
//! instance that enables RPC call specific to regtest such as
//! [`RegtestDaemonJsonRpcClient::generate_blocks`].
//!
//! ```rust
//! use monero_rpc::RpcClient;
//!
//! let client = RpcClient::new("http://node.monerooutreach.org:18081".to_string());
//! let daemon = client.daemon();
//! let regtest_daemon = daemon.regtest();
//! ```

#![forbid(unsafe_code)]

pub use monero;

#[macro_use]
mod util;
mod models;

pub use self::{models::*, util::*};

use jsonrpc_core::types::{Id, *};
use monero::{
    cryptonote::{hash::Hash as CryptoNoteHash, subaddress},
    util::address::PaymentId,
    Address,
};
use serde::{de::IgnoredAny, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::Debug,
    iter::{empty, once},
    num::NonZeroU64,
    ops::{Deref, RangeInclusive},
    sync::Arc,
};
use tracing::*;
use uuid::Uuid;

enum RpcParams {
    Array(Box<dyn Iterator<Item = Value> + Send + 'static>),
    Map(Box<dyn Iterator<Item = (String, Value)> + Send + 'static>),
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
        RpcParams::Map(Box::new(v.map(|(k, v)| (k.to_string(), v))))
    }
}

impl From<RpcParams> for Params {
    fn from(value: RpcParams) -> Self {
        match value {
            RpcParams::Map(v) => Params::Map(v.collect()),
            RpcParams::Array(v) => Params::Array(v.collect()),
            RpcParams::None => Params::None,
        }
    }
}

#[derive(Clone, Debug)]
struct RemoteCaller {
    http_client: reqwest::Client,
    addr: String,
}

impl RemoteCaller {
    async fn json_rpc_call(
        &self,
        method: &'static str,
        params: RpcParams,
    ) -> anyhow::Result<jsonrpc_core::Result<Value>> {
        let client = self.http_client.clone();
        let uri = format!("{}/json_rpc", &self.addr);

        let method_call = MethodCall {
            jsonrpc: Some(Version::V2),
            method: method.to_string(),
            params: params.into(),
            id: Id::Str(Uuid::new_v4().to_string()),
        };

        trace!("Sending JSON-RPC method call: {:?}", method_call);

        let rsp = client
            .post(&uri)
            .json(&method_call)
            .send()
            .await?
            .json::<response::Output>()
            .await?;

        trace!("Received JSON-RPC response: {:?}", rsp);
        let v = jsonrpc_core::Result::<Value>::from(rsp);
        Ok(v)
    }

    async fn daemon_rpc_call<T>(&self, method: &'static str, params: RpcParams) -> anyhow::Result<T>
    where
        T: for<'de> Deserialize<'de> + Send + 'static + Debug,
    {
        let client = self.http_client.clone();
        let uri = format!("{}/{}", &self.addr, method);

        let json_params: Params = params.into();

        trace!(
            "Sending daemon RPC call: {:?}, with params {:?}",
            method,
            json_params
        );

        let rsp = client
            .post(uri)
            .json(&json_params)
            .send()
            .await?
            .json::<T>()
            .await?;

        trace!("Received daemon RPC response: {:?}", rsp);

        Ok(rsp)
    }
}

#[derive(Clone, Debug)]
struct CallerWrapper(Arc<RemoteCaller>);

impl CallerWrapper {
    async fn request<T>(&self, method: &'static str, params: RpcParams) -> anyhow::Result<T>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let c = self.0.json_rpc_call(method, params);
        Ok(serde_json::from_value(c.await??)?)
    }

    async fn daemon_rpc_request<T>(
        &self,
        method: &'static str,
        params: RpcParams,
    ) -> anyhow::Result<T>
    where
        T: for<'de> Deserialize<'de> + Send + 'static + Debug,
    {
        let c = self.0.daemon_rpc_call(method, params).await?;
        Ok(serde_json::from_value(c)?)
    }
}

/// Base RPC client. It is useless on its own, please see the attached methods to see how to
/// transform it into a specialized client.
#[derive(Clone, Debug)]
pub struct RpcClient {
    inner: CallerWrapper,
}

impl RpcClient {
    /// Create a new generic RPC client that can be transformed into specialized client.
    pub fn new(addr: String) -> Self {
        Self {
            inner: CallerWrapper(Arc::new(RemoteCaller {
                http_client: reqwest::ClientBuilder::new().build().unwrap(),
                addr,
            })),
        }
    }

    /// Transform the client into the specialized `DaemonJsonRpcClient` that interacts with JSON RPC
    /// methods on daemon.
    pub fn daemon(self) -> DaemonJsonRpcClient {
        let Self { inner } = self;
        DaemonJsonRpcClient { inner }
    }

    /// Transform the client into the specialized `DaemonRpcClient` that interacts with methods on
    /// daemon called with their own extensions.
    pub fn daemon_rpc(self) -> DaemonRpcClient {
        let Self { inner } = self;
        DaemonRpcClient { inner }
    }

    /// Transform the client into the specialized `WalletClient` that interacts with a Monero
    /// wallet RPC daemon.
    pub fn wallet(self) -> WalletClient {
        let Self { inner } = self;
        WalletClient { inner }
    }
}

/// Result of [`RpcClient::daemon`] to interact with JSON RPC Methods on daemon.
///
/// The majority of monerod RPC calls use the daemon's json_rpc interface to request various bits
/// of information. These methods all follow a similar structure.
///
/// ```rust
/// use monero_rpc::RpcClient;
///
/// let client = RpcClient::new("http://node.monerooutreach.org:18081".to_string());
/// let daemon = client.daemon();
/// let regtest_daemon = daemon.regtest();
/// ```
#[derive(Clone, Debug)]
pub struct DaemonJsonRpcClient {
    inner: CallerWrapper,
}

/// Result of [`DaemonJsonRpcClient::regtest`] to enable methods for daemons in regtest mode.
#[derive(Clone, Debug)]
pub struct RegtestDaemonJsonRpcClient(pub DaemonJsonRpcClient);

impl Deref for RegtestDaemonJsonRpcClient {
    type Target = DaemonJsonRpcClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Selector for daemon `get_block_header`.
pub enum GetBlockHeaderSelector {
    /// Select the last block.
    Last,
    /// Select the block by its hash.
    Hash(BlockHash),
    /// Select the block by its height.
    Height(u64),
}

impl DaemonJsonRpcClient {
    /// Look up how many blocks are in the longest chain known to the node.
    pub async fn get_block_count(&self) -> anyhow::Result<NonZeroU64> {
        #[derive(Deserialize)]
        struct Rsp {
            count: NonZeroU64,
        }

        Ok(self
            .inner
            .request::<MoneroResult<Rsp>>("get_block_count", RpcParams::array(empty()))
            .await?
            .into_inner()
            .count)
    }

    /// Look up a block's hash by its height.
    pub async fn on_get_block_hash(&self, height: u64) -> anyhow::Result<BlockHash> {
        self.inner
            .request::<HashString<BlockHash>>(
                "on_get_block_hash",
                RpcParams::array(once(height.into())),
            )
            .await
            .map(|v| v.0)
    }

    /// Get a block template on which mining a new block.
    pub async fn get_block_template(
        &self,
        wallet_address: Address,
        reserve_size: u64,
    ) -> anyhow::Result<BlockTemplate> {
        Ok(self
            .inner
            .request::<MoneroResult<BlockTemplate>>(
                "get_block_template",
                RpcParams::map(
                    empty()
                        .chain(once((
                            "wallet_address",
                            serde_json::to_value(wallet_address).unwrap(),
                        )))
                        .chain(once(("reserve_size", reserve_size.into()))),
                ),
            )
            .await?
            .into_inner())
    }

    /// Submit a mined block to the network.
    pub async fn submit_block(&self, block_blob_data: String) -> anyhow::Result<String> {
        self.inner
            .request(
                "submit_block",
                RpcParams::array(once(block_blob_data.into())),
            )
            .await
    }

    /// Retrieve block header information matching selected filter.
    pub async fn get_block_header(
        &self,
        selector: GetBlockHeaderSelector,
    ) -> anyhow::Result<BlockHeaderResponse> {
        #[derive(Deserialize)]
        struct Rsp {
            block_header: BlockHeaderResponseR,
        }

        let (request, params) = match selector {
            GetBlockHeaderSelector::Last => ("get_last_block_header", RpcParams::None),
            GetBlockHeaderSelector::Hash(hash) => (
                "get_block_header_by_hash",
                RpcParams::map(
                    Some(("hash", serde_json::to_value(HashString(hash)).unwrap())).into_iter(),
                ),
            ),
            GetBlockHeaderSelector::Height(height) => (
                "get_block_header_by_height",
                RpcParams::map(Some(("height", height.into())).into_iter()),
            ),
        };

        Ok(self
            .inner
            .request::<Rsp>(request, params)
            .await?
            .block_header
            .into())
    }

    /// Similar to [`Self::get_block_header`] above, but for a range of blocks. This method
    /// includes a starting block height and an ending block height as parameters to retrieve basic
    /// information about the range of blocks.
    pub async fn get_block_headers_range(
        &self,
        range: RangeInclusive<u64>,
    ) -> anyhow::Result<(Vec<BlockHeaderResponse>, bool)> {
        #[derive(Deserialize)]
        struct Rsp {
            headers: Vec<BlockHeaderResponseR>,
            untrusted: bool,
        }

        let params = empty()
            .chain(once(("start_height", (*range.start()).into())))
            .chain(once(("end_height", (*range.end()).into())));

        let Rsp { headers, untrusted } = self
            .inner
            .request::<MoneroResult<Rsp>>("get_block_headers_range", RpcParams::map(params))
            .await?
            .into_inner();

        Ok((headers.into_iter().map(From::from).collect(), untrusted))
    }

    /// Enable additional functions for daemons in regtest mode.
    pub fn regtest(self) -> RegtestDaemonJsonRpcClient {
        RegtestDaemonJsonRpcClient(self)
    }
}

/// Result of [`RpcClient::daemon_rpc`] to interact with methods on daemon called with their own
/// extensions.
///
/// Not all daemon RPC calls use the `JSON_RPC` interface. The data structure for these calls is
/// different. Whereas the JSON RPC methods were called using the `/json_rpc` extension and
/// specifying a method, these methods are called at their own extensions.
///
/// ```rust
/// use monero_rpc::RpcClient;
///
/// let client = RpcClient::new("http://node.monerooutreach.org:18081".to_string());
/// let daemon = client.daemon_rpc();
/// ```
#[derive(Clone, Debug)]
pub struct DaemonRpcClient {
    inner: CallerWrapper,
}

impl DaemonRpcClient {
    /// Look up one or more transactions by hash.
    pub async fn get_transactions(
        &self,
        txs_hashes: Vec<CryptoNoteHash>,
        decode_as_json: Option<bool>,
        prune: Option<bool>,
    ) -> anyhow::Result<TransactionsResponse> {
        let params = empty()
            .chain(once((
                "txs_hashes",
                txs_hashes
                    .into_iter()
                    .map(|s| HashString(s).to_string())
                    .collect::<Vec<_>>()
                    .into(),
            )))
            .chain(decode_as_json.map(|v| ("decode_as_json", v.into())))
            .chain(prune.map(|v| ("prune", v.into())));
        self.inner
            .daemon_rpc_request::<TransactionsResponse>("get_transactions", RpcParams::map(params))
            .await
    }
}

impl RegtestDaemonJsonRpcClient {
    /// Generate blocks and give mining rewards to specified address.
    pub async fn generate_blocks(
        &self,
        amount_of_blocks: u64,
        wallet_address: Address,
    ) -> anyhow::Result<u64> {
        #[derive(Deserialize)]
        struct Rsp {
            height: u64,
        }

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
            .request::<MoneroResult<Rsp>>("generateblocks", RpcParams::map(params))
            .await?
            .into_inner()
            .height)
    }
}

impl Serialize for TransferType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            TransferType::All => "all",
            TransferType::Available => "available",
            TransferType::Unavailable => "unavailable",
        })
    }
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

/// Result of [`RpcClient::wallet`] to interact with a Monero wallet RPC daemon.
///
/// ```rust
/// use monero_rpc::RpcClient;
///
/// let client = RpcClient::new("http://127.0.0.1:18083".to_string());
/// let daemon = client.wallet();
/// ```
#[derive(Clone, Debug)]
pub struct WalletClient {
    inner: CallerWrapper,
}

impl WalletClient {
    /// Generate a new wallet from viewkey, address and optionally a spend key.  Requires the rpc
    /// wallet to run with the `--wallet-dir` argument.
    pub async fn generate_from_keys(
        &self,
        args: GenerateFromKeysArgs,
    ) -> anyhow::Result<WalletCreation> {
        let params = empty()
            .chain(args.restore_height.map(|v| ("restore_height", v.into())))
            .chain(once(("filename", args.filename.into())))
            .chain(once(("address", args.address.to_string().into())))
            .chain(args.spendkey.map(|v| ("spendkey", v.to_string().into())))
            .chain(once(("viewkey", args.viewkey.to_string().into())))
            .chain(once(("password", args.password.into())))
            .chain(
                args.autosave_current
                    .map(|v| ("autosave_current", v.into())),
            );
        self.inner
            .request("generate_from_keys", RpcParams::map(params))
            .await
    }

    /// Create a new wallet. You need to have set the argument `--wallet-dir` when launching
    /// monero-wallet-rpc to make this work.
    pub async fn create_wallet(
        &self,
        filename: String,
        password: Option<String>,
        language: String,
    ) -> anyhow::Result<()> {
        let params = empty()
            .chain(once(("filename", filename.into())))
            .chain(password.map(|v| ("password", v.into())))
            .chain(once(("language", language.into())));
        self.inner
            .request::<IgnoredAny>("create_wallet", RpcParams::map(params))
            .await?;
        Ok(())
    }

    /// Open a wallet. You need to have set the argument `--wallet-dir` when launching
    /// monero-wallet-rpc to make this work.
    pub async fn open_wallet(
        &self,
        filename: String,
        password: Option<String>,
    ) -> anyhow::Result<()> {
        let params = empty()
            .chain(once(("filename", filename.into())))
            .chain(password.map(|v| ("password", v.into())));

        self.inner
            .request::<IgnoredAny>("open_wallet", RpcParams::map(params))
            .await?;
        Ok(())
    }

    /// Close the currently opened wallet, after trying to save it.
    pub async fn close_wallet(&self) -> anyhow::Result<()> {
        let params = empty();
        self.inner
            .request::<IgnoredAny>("close_wallet", RpcParams::map(params))
            .await?;
        Ok(())
    }

    /// Return the wallet's balance.
    pub async fn get_balance(
        &self,
        account_index: u32,
        address_indices: Option<Vec<u32>>,
    ) -> anyhow::Result<BalanceData> {
        let params = empty()
            .chain(once(("account_index", account_index.into())))
            .chain(address_indices.map(|v| {
                (
                    "address_indices",
                    v.into_iter().map(Value::from).collect::<Vec<_>>().into(),
                )
            }));

        self.inner
            .request("get_balance", RpcParams::map(params))
            .await
    }

    /// Return the wallet's addresses for an account. Optionally filter for specific set of
    /// subaddresses.
    pub async fn get_address(
        &self,
        account: u32,
        addresses: Option<Vec<u32>>,
    ) -> anyhow::Result<AddressData> {
        let params = empty()
            .chain(once(("account_index", account.into())))
            .chain(addresses.map(|v| {
                (
                    "address_index",
                    v.into_iter().map(Value::from).collect::<Vec<_>>().into(),
                )
            }));

        self.inner
            .request("get_address", RpcParams::map(params))
            .await
    }

    /// Get account and address indexes from a specific (sub)address.
    pub async fn get_address_index(&self, address: Address) -> anyhow::Result<subaddress::Index> {
        #[derive(Deserialize)]
        struct Rsp {
            index: subaddress::Index,
        }

        let params = once(("address", address.to_string().into()));

        let rsp = self
            .inner
            .request::<Rsp>("get_address_index", RpcParams::map(params))
            .await?;

        Ok(subaddress::Index {
            major: rsp.index.major,
            minor: rsp.index.minor,
        })
    }

    /// Create a new address for an account. Optionally, label the new address.
    pub async fn create_address(
        &self,
        account_index: u32,
        label: Option<String>,
    ) -> anyhow::Result<(Address, u32)> {
        #[derive(Deserialize)]
        struct Rsp {
            address: Address,
            address_index: u32,
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

    /// Label an address.
    pub async fn label_address(
        &self,
        index: subaddress::Index,
        label: String,
    ) -> anyhow::Result<()> {
        let params = empty()
            .chain(once(("index", json!(index))))
            .chain(once(("label", label.into())));

        self.inner
            .request::<IgnoredAny>("label_address", RpcParams::map(params))
            .await?;

        Ok(())
    }

    /// Refresh a wallet after openning.
    pub async fn refresh(&self, start_height: Option<u64>) -> anyhow::Result<RefreshData> {
        let params = empty().chain(start_height.map(|v| ("start_height", v.into())));

        self.inner.request("refresh", RpcParams::map(params)).await
    }

    /// Get all accounts for a wallet. Optionally filter accounts by tag.
    pub async fn get_accounts(&self, tag: Option<String>) -> anyhow::Result<GetAccountsData> {
        let params = empty().chain(tag.map(|v| ("tag", v.into())));

        self.inner
            .request("get_accounts", RpcParams::map(params))
            .await
    }

    /// Get a list of incoming payments using a given payment id.
    pub async fn get_payments(&self, payment_id: PaymentId) -> anyhow::Result<Vec<Payment>> {
        let params = empty().chain(once((
            "payment_id",
            HashString(payment_id).to_string().into(),
        )));

        self.inner
            .request("get_payments", RpcParams::map(params))
            .await
    }

    /// Get a list of incoming payments using a given payment id, or a list of payments ids, from a
    /// given height. This method is the preferred method over [`Self::get_payments`] because it
    /// has the same functionality but is more extendable. Either is fine for looking up
    /// transactions by a single payment ID.
    pub async fn get_bulk_payments(
        &self,
        payment_ids: Vec<PaymentId>,
        // It seems that the `min_block_height` argument is really optional, but the docs on the Monero website do not mention it
        min_block_height: u64,
    ) -> anyhow::Result<Vec<Payment>> {
        #[derive(Deserialize)]
        struct Rsp {
            #[serde(default)]
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

    /// Return the spend or view private key.
    pub async fn query_key(
        &self,
        key_selector: PrivateKeyType,
    ) -> anyhow::Result<monero::PrivateKey> {
        #[derive(Deserialize)]
        struct Rsp {
            key: HashString<Vec<u8>>,
        }

        let params = empty().chain({
            match key_selector {
                PrivateKeyType::View => empty().chain(once(("key_type", "view_key".into()))),
                PrivateKeyType::Spend => empty().chain(once(("key_type", "spend_key".into()))),
            }
        });
        let rsp = self
            .inner
            .request::<Rsp>("query_key", RpcParams::map(params))
            .await?;

        Ok(monero::PrivateKey::from_slice(&rsp.key.0)?)
    }

    /// Returns the wallet's current block height.
    pub async fn get_height(&self) -> anyhow::Result<NonZeroU64> {
        #[derive(Deserialize)]
        struct Rsp {
            height: NonZeroU64,
        }

        Ok(self
            .inner
            .request::<Rsp>("get_height", RpcParams::None)
            .await?
            .height)
    }

    /// Send all unlocked balance to an address.
    pub async fn sweep_all(&self, args: SweepAllArgs) -> anyhow::Result<SweepAllData> {
        let params = empty()
            .chain(once(("address", args.address.to_string().into())))
            .chain(once(("account_index", args.account_index.into())))
            .chain(args.subaddr_indices.map(|v| ("subaddr_indices", v.into())))
            .chain(once(("priority", serde_json::to_value(args.priority)?)))
            .chain(once(("mixin", args.mixin.into())))
            .chain(once(("ring_size", args.ring_size.into())))
            .chain(once(("unlock_time", args.unlock_time.into())))
            .chain(args.get_tx_keys.map(|v| ("get_tx_keys", v.into())))
            .chain(
                args.below_amount
                    .map(|v| ("below_amount", v.as_pico().into())),
            )
            .chain(args.do_not_relay.map(|v| ("do_not_relay", v.into())))
            .chain(args.get_tx_hex.map(|v| ("get_tx_hex", v.into())))
            .chain(args.get_tx_metadata.map(|v| ("get_tx_metadata", v.into())));
        self.inner
            .request("sweep_all", RpcParams::map(params))
            .await
    }

    /// Relay a transaction previously created with `"do_not_relay":true`.
    pub async fn relay_tx(&self, tx_metadata_hex: String) -> anyhow::Result<CryptoNoteHash> {
        #[derive(Deserialize)]
        struct Rsp {
            tx_hash: HashString<CryptoNoteHash>,
        }
        let params = empty().chain(once(("hex", tx_metadata_hex.into())));
        Ok(self
            .inner
            .request::<Rsp>("relay_tx", RpcParams::map(params))
            .await?
            .tx_hash
            .0)
    }

    /// Send monero to a number of recipients.
    pub async fn transfer(
        &self,
        destinations: HashMap<Address, monero::Amount>,
        priority: TransferPriority,
        options: TransferOptions,
    ) -> anyhow::Result<TransferData> {
        let params = empty()
            .chain(once((
                "destinations",
                destinations
                    .into_iter()
                    .map(
                        |(address, amount)| json!({"address": address, "amount": amount.as_pico()}),
                    )
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

    /// Sign a transaction created on a read-only wallet (in cold-signing process).
    pub async fn sign_transfer(
        &self,
        unsigned_txset: Vec<u8>,
    ) -> anyhow::Result<SignedTransferOutput> {
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

    /// Submit a previously signed transaction on a read-only wallet (in cold-signing process).
    pub async fn submit_transfer(
        &self,
        tx_data_hex: Vec<u8>,
    ) -> anyhow::Result<Vec<CryptoNoteHash>> {
        #[derive(Deserialize)]
        struct Rsp {
            tx_hash_list: Vec<HashString<CryptoNoteHash>>,
        }

        let params = empty().chain(once((
            "tx_data_hex",
            HashString(tx_data_hex).to_string().into(),
        )));

        self.inner
            .request::<Rsp>("submit_transfer", RpcParams::map(params))
            .await
            .map(|v| v.tx_hash_list.into_iter().map(|v| v.0).collect())
    }

    /// Return a list of incoming transfers to the wallet.
    pub async fn incoming_transfers(
        &self,
        transfer_type: TransferType,
        account_index: Option<u32>,
        subaddr_indices: Option<Vec<u32>>,
    ) -> anyhow::Result<IncomingTransfers> {
        let params = empty()
            .chain(once((
                "transfer_type",
                serde_json::to_value(transfer_type)?,
            )))
            .chain(account_index.map(|v| ("account_index", v.into())))
            .chain(subaddr_indices.map(|v| ("subaddr_indices", v.into())));

        self.inner
            .request("incoming_transfers", RpcParams::map(params))
            .await
    }

    /// Returns a list of transfers.
    pub async fn get_transfers(
        &self,
        selector: GetTransfersSelector,
    ) -> anyhow::Result<HashMap<GetTransfersCategory, Vec<GotTransfer>>> {
        let GetTransfersSelector {
            category_selector,
            account_index,
            subaddr_indices,
            block_height_filter,
        } = selector;

        let mut min_height = None;
        let mut max_height = None;

        if let Some(block_filter) = block_height_filter.clone() {
            min_height = match block_filter.min_height {
                Some(x) => Some(x),
                None => Some(0),
            };
            max_height = block_filter.max_height;
        }

        let params = empty()
            .chain(
                category_selector
                    .into_iter()
                    .map(|(cat, b)| (cat.into(), b.into())),
            )
            .chain(account_index.map(|v| ("account_index", v.into())))
            .chain(subaddr_indices.map(|v| ("subaddr_indices", v.into())))
            .chain(
                block_height_filter
                    .clone()
                    .map(|_| ("filter_by_height", true.into())),
            )
            .chain(min_height.map(|b| ("min_height", b.into())))
            .chain(max_height.map(|b| ("max_height", b.into())));

        self.inner
            .request("get_transfers", RpcParams::map(params))
            .await
    }

    /// Show information about a transfer to/from this address. **Calls `get_transfer_by_txid` in
    /// RPC.**
    pub async fn get_transfer(
        &self,
        txid: CryptoNoteHash,
        account_index: Option<u32>,
    ) -> anyhow::Result<Option<GotTransfer>> {
        #[derive(Deserialize)]
        struct Rsp {
            transfer: GotTransfer,
        }

        let params = empty()
            .chain(Some(("txid", HashString(txid).to_string().into())))
            .chain(account_index.map(|v| ("account_index", v.into())));

        let rsp = match self
            .inner
            .0
            .json_rpc_call("get_transfer_by_txid", RpcParams::map(params))
            .await?
        {
            Ok(v) => serde_json::from_value::<Rsp>(v)?,
            Err(e) => {
                if e.code == jsonrpc_core::ErrorCode::ServerError(-8) {
                    return Ok(None);
                } else {
                    return Err(e.into());
                }
            }
        };

        Ok(Some(rsp.transfer))
    }

    /// Export a signed set of key images.
    pub async fn export_key_images(
        &self,
        all: Option<bool>,
    ) -> anyhow::Result<Vec<SignedKeyImage>> {
        #[derive(Deserialize)]
        struct R {
            key_image: HashString<Vec<u8>>,
            signature: HashString<Vec<u8>>,
        }

        #[derive(Deserialize)]
        struct Rsp {
            #[serde(default)]
            signed_key_images: Vec<R>,
        }

        impl From<Rsp> for Vec<SignedKeyImage> {
            fn from(rsp: Rsp) -> Self {
                rsp.signed_key_images
                    .into_iter()
                    .map(
                        |R {
                             key_image,
                             signature,
                         }| SignedKeyImage {
                            key_image: key_image.0,
                            signature: signature.0,
                        },
                    )
                    .collect()
            }
        }

        let params = empty().chain(all.map(|v| ("all", v.into())));

        self.inner
            .request::<Rsp>("export_key_images", RpcParams::map(params))
            .await
            .map(From::from)
    }

    /// Import signed key images list and verify their spent status.
    pub async fn import_key_images(
        &self,
        signed_key_images: Vec<SignedKeyImage>,
    ) -> anyhow::Result<KeyImageImportResponse> {
        let params = empty().chain(once((
            "signed_key_images",
            signed_key_images
                .into_iter()
                .map(
                    |SignedKeyImage {
                         key_image,
                         signature,
                     }| {
                        json!({
                            "key_image": HashString(key_image),
                            "signature": HashString(signature),
                        })
                    },
                )
                .collect::<Vec<_>>()
                .into(),
        )));

        self.inner
            .request("import_key_images", RpcParams::map(params))
            .await
    }

    /// Check a transaction in the blockchain with its secret key.
    pub async fn check_tx_key(
        &self,
        txid: CryptoNoteHash,
        tx_key: CryptoNoteHash,
        address: Address,
    ) -> anyhow::Result<(NonZeroU64, bool, NonZeroU64)> {
        #[derive(Deserialize)]
        struct Rsp {
            confirmations: NonZeroU64,
            in_pool: bool,
            received: NonZeroU64,
        }

        let params = empty()
            .chain(once(("txid", HashString(txid).to_string().into())))
            .chain(once(("tx_key", HashString(tx_key).to_string().into())))
            .chain(once(("address", address.to_string().into())));

        let rsp = self
            .inner
            .request::<Rsp>("check_tx_key", RpcParams::map(params))
            .await?;

        Ok((rsp.confirmations, rsp.in_pool, rsp.received))
    }

    /// Get RPC version Major & Minor integer-format, where Major is the first 16 bits and Minor
    /// the last 16 bits.
    pub async fn get_version(&self) -> anyhow::Result<(u16, u16)> {
        #[derive(Deserialize)]
        struct Rsp {
            version: u32,
        }

        let version = self
            .inner
            .request::<Rsp>("get_version", RpcParams::None)
            .await?;

        let major = version.version >> 16;
        let minor = version.version - (major << 16);

        Ok((u16::try_from(major)?, u16::try_from(minor)?))
    }
}
