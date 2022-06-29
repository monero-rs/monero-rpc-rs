use crate::util::*;
use chrono::prelude::*;
use monero::{cryptonote::hash::Hash as CryptoNoteHash, util::address::PaymentId, Address};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, num::NonZeroU64};

macro_rules! hash_type {
    ($name:ident, $len:expr) => {
        ::fixed_hash::construct_fixed_hash! {
            /// Return type of daemon `on_get_block_hash`.
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            pub struct $name($len);
        }

        hash_type_impl!($name);
    };
}

hash_type!(BlockHash, 32);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Status {
    OK,
}

/// Helper type to unwrap RPC results.
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

/// Return type of daemon `get_block_template`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockTemplate {
    pub blockhashing_blob: HashString<Vec<u8>>,
    pub blocktemplate_blob: HashString<Vec<u8>>,
    pub difficulty: u64,
    pub expected_reward: u64,
    pub height: u64,
    pub prev_hash: HashString<BlockHash>,
    pub reserved_offset: u64,
    pub untrusted: bool,
}

#[derive(Deserialize)]
pub(crate) struct BlockHeaderResponseR {
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
    pub reward: u64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
}

impl From<BlockHeaderResponseR> for BlockHeaderResponse {
    fn from(value: BlockHeaderResponseR) -> Self {
        Self {
            block_size: value.block_size,
            depth: value.depth,
            difficulty: value.difficulty,
            hash: value.hash.0,
            height: value.height,
            major_version: value.major_version,
            minor_version: value.minor_version,
            nonce: value.nonce,
            num_txes: value.num_txes,
            orphan_status: value.orphan_status,
            prev_hash: value.prev_hash.0,
            reward: value.reward,
            timestamp: value.timestamp,
        }
    }
}

/// Return type of daemon `get_block_header` and `get_block_headers_range`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeaderResponse {
    pub block_size: u64,
    pub depth: u64,
    pub difficulty: u64,
    pub hash: BlockHash,
    pub height: u64,
    pub major_version: u64,
    pub minor_version: u64,
    pub nonce: u32,
    pub num_txes: u64,
    pub orphan_status: bool,
    pub prev_hash: BlockHash,
    pub reward: u64,
    pub timestamp: DateTime<Utc>,
}

/// Return type of daemon RPC `get_transactions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionsResponse {
    pub credits: u64,
    pub top_hash: String,
    pub status: String,
    pub missed_tx: Option<Vec<HashString<CryptoNoteHash>>>,
    pub txs: Option<Vec<Transaction>>,
    pub txs_as_hex: Option<Vec<String>>,
    pub txs_as_json: Option<Vec<String>>, // needs to be parsed as JsonTransaction, but is received as a string
    pub untrusted: bool,
}

/// Sub-type of [`TransactionsResponse`]'s return type of daemon RPC `get_transactions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub as_hex: String,
    pub as_json: Option<String>, // needs to be parsed as JsonTransaction, but is received as a string
    pub block_height: Option<u64>,
    pub block_timestamp: Option<u64>,
    pub double_spend_seen: bool,
    pub in_pool: bool,
    pub output_indices: Option<Vec<u64>>,
    pub tx_hash: HashString<CryptoNoteHash>,
}

/// Helper type to partially decode `as_json` string fields in other RPC return types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonTransaction {
    pub version: u64,
    pub unlock_time: u64,
    // TODO: these fields are skipped for now, their content changes often from hardfork to hardfork
    // vin, vout, extra, rct_signatures, rct_sig_prunable
}

/// Sub-type of [`BalanceData`]'s return type of wallet `get_balance`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubaddressBalanceData {
    pub address: Address,
    pub address_index: u64,
    pub balance: u64,
    pub label: String,
    pub num_unspent_outputs: u64,
    pub unlocked_balance: u64,
}

/// Return type of wallet `get_balance`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceData {
    /// Balance amount of account queried.
    pub balance: u64,
    /// If multisig import is needed.
    pub multisig_import_needed: bool,
    /// Balance data for each sub indicies queried.
    #[serde(default)]
    pub per_subaddress: Vec<SubaddressBalanceData>,
    /// Amount of unlocked balance in account queried.
    pub unlocked_balance: u64,
}

/// Argument type of wallet `transfer`.
#[derive(Copy, Clone, Debug)]
pub enum TransferPriority {
    Default,
    Unimportant,
    Elevated,
    Priority,
}

/// Return type of wallet `transfer`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferData {
    pub amount: u64,
    pub fee: u64,
    pub tx_blob: HashString<Vec<u8>>,
    pub tx_hash: HashString<CryptoNoteHash>,
    pub tx_key: HashString<CryptoNoteHash>,
    pub tx_metadata: HashString<Vec<u8>>,
    pub unsigned_txset: HashString<Vec<u8>>,
}

/// Sub-type of [`AddressData`]'s return type of wallet `get_address`.
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

/// Return type of wallet `get_payments`.
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

/// Return type of wallet `generate_from_keys`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletCreation {
    /// Generated wallet address.
    pub address: Address,
    /// Info on generated wallet.
    pub info: String,
}

/// Return type of wallet `get_address`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressData {
    /// Address of the account queried.
    pub address: Address,
    /// A set of filtered subaddress.
    pub addresses: Vec<SubaddressData>,
}

/// Argument type of wallet `incoming_transfers`.
#[derive(Copy, Clone, Debug)]
pub enum TransferType {
    All,
    Available,
    Unavailable,
}

/// Return type of wallet `incoming_transfers`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncomingTransfers {
    pub transfers: Option<Vec<IncomingTransfer>>,
}

/// Sub-type of [`IncomingTransfers`]. Represent one incoming transfer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncomingTransfer {
    pub amount: u64,
    pub global_index: u64,
    pub key_image: Option<String>,
    pub spent: bool,
    pub subaddr_index: SubAddressIndex,
    pub tx_hash: HashString<CryptoNoteHash>,
    pub tx_size: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubAddressIndex {
    pub major: u64,
    pub minor: u64,
}

/// Argument type of wallet `sweep_all`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SweepAllArgs {
    pub address: Address,
    pub account_index: u64,
    pub subaddr_indices: Option<Vec<u64>>,
    pub priority: TransferPriority,
    pub mixin: u64,
    pub ring_size: u64,
    pub unlock_time: u64,
    pub get_tx_keys: Option<bool>,
    pub below_amount: Option<u64>,
    pub do_not_relay: Option<bool>,
    pub get_tx_hex: Option<bool>,
    pub get_tx_metadata: Option<bool>,
}

/// Return type of wallet `sweep_all`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SweepAllData {
    pub tx_hash_list: Vec<HashString<CryptoNoteHash>>,
    pub tx_key_list: Option<Vec<HashString<CryptoNoteHash>>>,
    pub amount_list: Vec<u64>,
    pub fee_list: Vec<u64>,
    pub tx_blob_list: Option<Vec<String>>,
    pub tx_metadata_list: Option<Vec<String>>,
    pub multisig_txset: String,
    pub unsigned_txset: String,
}

/// Argument type of wallet `transfer`.
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

/// Argument type of wallet `generate_from_keys`.
#[derive(Clone, Debug)]
pub struct GenerateFromKeysArgs {
    pub restore_height: Option<u64>,
    pub filename: String,
    pub address: Address,
    pub spendkey: Option<monero::PrivateKey>,
    pub viewkey: monero::PrivateKey,
    pub password: String,
    pub autosave_current: Option<bool>,
}

/// Return sub-type of wallet `get_accounts`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GotAccount {
    pub account_index: u64,
    pub balance: u64,
    pub base_address: monero::Address,
    pub label: Option<String>,
    pub tag: Option<String>,
    pub unlocked_balance: u64,
}

/// Return type of wallet `refresh`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshData {
    pub blocks_fetched: u64,
    pub received_money: bool,
}

/// Return type of wallet `get_accounts`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetAccountsData {
    pub subaddress_accounts: Vec<GotAccount>,
    pub total_balance: u64,
    pub total_unlocked_balance: u64,
}

/// Monero uses two type of private key in its cryptographic system: (1) a view key, and (2) a
/// spend key.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PrivateKeyType {
    /// Viewkey type.
    View,
    /// Spendkey type.
    Spend,
}

/// Part of return type of wallet `get_transfers`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GetTransfersCategory {
    /// Incoming transfers.
    In,
    /// Outgoing transfers.
    Out,
    /// Pending transfers.
    Pending,
    /// Failed transfers.
    Failed,
    /// Pool transfers.
    Pool,
    /// Block transfers.
    Block,
}

impl From<GetTransfersCategory> for &'static str {
    fn from(value: GetTransfersCategory) -> Self {
        use GetTransfersCategory::*;

        match value {
            In => "in",
            Out => "out",
            Pending => "pending",
            Failed => "failed",
            Pool => "pool",
            Block => "block",
        }
    }
}

/// Argument type of wallet `get_transfers`.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct GetTransfersSelector {
    pub category_selector: HashMap<GetTransfersCategory, bool>,
    /// Index of the account to query for transfers. (defaults to 0)
    pub account_index: Option<u64>,
    /// List of subaddress indices to query for transfers. (Defaults to empty - all indices)
    pub subaddr_indices: Option<Vec<u64>>,
    /// Filter transfers by block height.
    pub block_height_filter: Option<BlockHeightFilter>,
}

/// Configuration filter for [`GetTransfersSelector`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeightFilter {
    /// Optional minimal height. (**Excluded bound**)
    pub min_height: Option<u64>,
    /// Optional maximum height. (**Included bound**)
    pub max_height: Option<u64>,
}

/// Sub-type of [`GotTransfer`].
#[derive(Clone, Debug)]
pub enum TransferHeight {
    Confirmed(NonZeroU64),
    InPool,
}

impl<'de> Deserialize<'de> for TransferHeight {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let h = u64::deserialize(deserializer)?;

        Ok({
            if let Some(h) = NonZeroU64::new(h) {
                TransferHeight::Confirmed(h)
            } else {
                TransferHeight::InPool
            }
        })
    }
}

/// Return type of wallet `get_transfer` and `get_transfers`.
#[derive(Clone, Debug, Deserialize)]
pub struct GotTransfer {
    /// Public address of the transfer.
    pub address: Address,
    /// Amount transferred.
    pub amount: u64,
    /// Number of block mined since the block containing this transaction (or block height at which the transaction should be added to a block if not yet confirmed).
    pub confirmations: Option<u64>,
    /// True if the key image(s) for the transfer have been seen before.
    pub double_spend_seen: bool,
    /// Transaction fee for this transfer.
    pub fee: u64,
    /// Height of the first block that confirmed this transfer (0 if not mined yet).
    pub height: TransferHeight,
    /// Note about this transfer.
    pub note: String,
    /// Payment ID for this transfer.
    pub payment_id: HashString<PaymentId>,
    /// JSON object containing the major & minor subaddress index.
    pub subaddr_index: SubaddressIndex,
    /// Estimation of the confirmations needed for the transaction to be included in a block.
    pub suggested_confirmations_threshold: u64,
    /// POSIX timestamp for when this transfer was first confirmed in a block (or timestamp submission if not mined yet).
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
    /// Transaction ID for this transfer.
    pub txid: HashString<Vec<u8>>,
    /// Type of transfer.
    #[serde(rename = "type")]
    pub transfer_type: GetTransfersCategory,
    /// Number of blocks until transfer is safely spendable.
    pub unlock_time: u64,
}

/// Return type of wallet `sign_transfer`.
#[derive(Clone, Debug)]
pub struct SignedTransferOutput {
    pub signed_txset: Vec<u8>,
    pub tx_hash_list: Vec<CryptoNoteHash>,
    pub tx_raw_list: Vec<Vec<u8>>,
}

/// Used to export and import signed key images. Return type of wallet `export_key_images` and
/// argument type of wallet `import_key_images`.
#[derive(Clone, Debug)]
pub struct SignedKeyImage {
    /// The key image.
    pub key_image: Vec<u8>,
    /// Signature of the key image.
    pub signature: Vec<u8>,
}

/// Return type of wallet `import_key_images`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyImageImportResponse {
    pub height: u64,
    /// Amount spent from key images.
    pub spent: u64,
    /// Amount still available from key images.
    pub unspent: u64,
}
