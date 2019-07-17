use {
    chrono::prelude::*,
    monero::{cryptonote::hash::Hash as CryptoNoteHash, Address, PaymentId},
    serde::{Deserialize, Deserializer, Serialize},
    std::{collections::HashMap, num::NonZeroU64},
};

use crate::util::*;

macro_rules! hash_type {
    ($name:ident, $len:expr) => {
        fixed_hash::construct_fixed_hash! {
            #[derive(Serialize, Deserialize)]
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
    #[serde(default)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferData {
    pub amount: u64,
    pub fee: u64,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GotAccount {
    pub account_index: u64,
    pub balance: u64,
    pub base_address: monero::Address,
    pub label: Option<String>,
    pub tag: Option<String>,
    pub unlocked_balance: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetAccountsData {
    pub subaddress_accounts: Vec<GotAccount>,
    pub total_balance: u64,
    pub total_unlocked_balance: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GetTransfersCategory {
    In,
    Out,
    Pending,
    Failed,
    Pool,
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
        }
    }
}

#[derive(Clone, Debug)]
pub struct GetTransfersSelector<T> {
    pub category_selector: HashMap<GetTransfersCategory, bool>,
    /// Filter transfers by block height.
    pub filter_by_height: Option<T>,
    /// Index of the account to query for transfers. (defaults to 0)
    pub account_index: Option<u64>,
    /// List of subaddress indices to query for transfers. (Defaults to empty - all indices)
    pub subaddr_indices: Option<Vec<u64>>,
}

impl<T> Default for GetTransfersSelector<T> {
    fn default() -> Self {
        Self {
            category_selector: Default::default(),
            filter_by_height: Default::default(),
            account_index: Default::default(),
            subaddr_indices: Default::default(),
        }
    }
}

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

#[derive(Clone, Debug, Deserialize)]
pub struct GotTransfer {
    /// Public address of the transfer.
    pub address: Address,
    /// Amount transferred.
    pub amount: u64,
    /// Number of block mined since the block containing this transaction (or block height at which the transaction should be added to a block if not yet confirmed).
    pub confirmations: u64,
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

#[derive(Clone, Debug)]
pub struct SignedTransferOutput {
    pub signed_txset: Vec<u8>,
    pub tx_hash_list: Vec<CryptoNoteHash>,
    pub tx_raw_list: Vec<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct SignedKeyImage {
    pub key_image: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyImageImportResponse {
    pub height: u64,
    /// Amount spent from key images.
    pub spent: u64,
    /// Amount still available from key images.
    pub unspent: u64,
}
