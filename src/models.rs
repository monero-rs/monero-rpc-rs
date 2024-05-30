// Copyright 2019-2023 Artem Vorotnikov and Monero Rust Contributors
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

// TODO: remove this when clippy error goes away...
#![allow(clippy::non_canonical_clone_impl)]

use crate::util::*;
use chrono::prelude::*;
use monero::{
    cryptonote::{hash::Hash as CryptoNoteHash, subaddress},
    util::{
        address::PaymentId,
        amount::{self, Amount},
    },
    Address,
};
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockTemplate {
    pub blockhashing_blob: HashString<Vec<u8>>,
    pub blocktemplate_blob: HashString<Vec<u8>>,
    pub difficulty: u64,
    #[serde(with = "amount::serde::as_pico")]
    pub expected_reward: Amount,
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
    #[serde(with = "amount::serde::as_pico")]
    pub reward: Amount,
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    #[serde(with = "amount::serde::as_pico")]
    pub reward: Amount,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GenerateBlocksResponseR {
    pub height: u64,
    pub blocks: Option<Vec<HashString<BlockHash>>>,
}

/// Return type of regtest daemon RPC `generate_blocks`
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GenerateBlocksResponse {
    pub height: u64,
    pub blocks: Option<Vec<BlockHash>>,
}

impl From<GenerateBlocksResponseR> for GenerateBlocksResponse {
    fn from(v: GenerateBlocksResponseR) -> Self {
        let GenerateBlocksResponseR { height, blocks } = v;

        Self {
            height,
            blocks: blocks.map(|vec| vec.into_iter().map(|b| b.0).collect()),
        }
    }
}

/// Return type of daemon RPC `get_transactions`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubaddressBalanceData {
    pub address: Address,
    pub address_index: u32,
    #[serde(with = "amount::serde::as_pico")]
    pub balance: Amount,
    pub label: String,
    pub num_unspent_outputs: u64,
    #[serde(with = "amount::serde::as_pico")]
    pub unlocked_balance: Amount,
}

/// Return type of wallet `get_balance`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BalanceData {
    /// Balance amount of account queried.
    #[serde(with = "amount::serde::as_pico")]
    pub balance: Amount,
    /// If multisig import is needed.
    pub multisig_import_needed: bool,
    /// Balance data for each sub indicies queried.
    #[serde(default)]
    pub per_subaddress: Vec<SubaddressBalanceData>,
    /// Amount of unlocked balance in account queried.
    #[serde(with = "amount::serde::as_pico")]
    pub unlocked_balance: Amount,
}

/// Argument type of wallet `transfer`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TransferPriority {
    Default,
    Unimportant,
    Elevated,
    Priority,
}

/// Return type of wallet `transfer`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferData {
    #[serde(with = "amount::serde::as_pico")]
    pub amount: Amount,
    #[serde(with = "amount::serde::as_pico")]
    pub fee: Amount,
    pub tx_blob: HashString<Vec<u8>>,
    pub tx_hash: HashString<CryptoNoteHash>,
    pub tx_key: HashString<Vec<u8>>,
    pub tx_metadata: HashString<Vec<u8>>,
    pub unsigned_txset: HashString<Vec<u8>>,
}

/// Sub-type of [`AddressData`]'s return type of wallet `get_address`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubaddressData {
    pub address: Address,
    pub address_index: u32,
    pub label: String,
    pub used: bool,
}

/// Return type of wallet `get_payments`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Payment {
    pub payment_id: HashString<PaymentId>,
    pub tx_hash: HashString<CryptoNoteHash>,
    #[serde(with = "amount::serde::as_pico")]
    pub amount: Amount,
    pub block_height: u64,
    pub unlock_time: u64,
    pub subaddr_index: subaddress::Index,
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IncomingTransfers {
    pub transfers: Option<Vec<IncomingTransfer>>,
}

/// Sub-type of [`IncomingTransfers`]. Represent one incoming transfer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IncomingTransfer {
    #[serde(with = "amount::serde::as_pico")]
    pub amount: Amount,
    pub global_index: u64,
    pub key_image: Option<String>,
    pub spent: bool,
    pub subaddr_index: subaddress::Index,
    pub tx_hash: HashString<CryptoNoteHash>,
    pub tx_size: Option<u64>,
}

/// Argument type of wallet `sweep_all`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SweepAllArgs {
    pub address: Address,
    pub account_index: u32,
    pub subaddr_indices: Option<Vec<u32>>,
    pub priority: TransferPriority,
    pub mixin: u64,
    pub ring_size: u64,
    pub unlock_time: u64,
    pub get_tx_keys: Option<bool>,
    #[serde(default, with = "amount::serde::as_pico::opt")]
    pub below_amount: Option<Amount>,
    pub do_not_relay: Option<bool>,
    pub get_tx_hex: Option<bool>,
    pub get_tx_metadata: Option<bool>,
}

/// Return type of wallet `sweep_all`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SweepAllData {
    pub tx_hash_list: Vec<HashString<CryptoNoteHash>>,
    pub tx_key_list: Option<Vec<HashString<CryptoNoteHash>>>,
    #[serde(
        default,
        serialize_with = "amount::serde::as_pico::slice::serialize",
        deserialize_with = "amount::serde::as_pico::vec::deserialize_amount"
    )]
    pub amount_list: Vec<Amount>,
    #[serde(
        default,
        serialize_with = "amount::serde::as_pico::slice::serialize",
        deserialize_with = "amount::serde::as_pico::vec::deserialize_amount"
    )]
    pub fee_list: Vec<Amount>,
    pub tx_blob_list: Option<Vec<String>>,
    pub tx_metadata_list: Option<Vec<String>>,
    pub multisig_txset: String,
    pub unsigned_txset: String,
}

/// Argument type of wallet `transfer`.
#[derive(Clone, Debug, Default)]
pub struct TransferOptions {
    pub account_index: Option<u32>,
    pub subaddr_indices: Option<Vec<u32>>,
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
    // TODO it seems this argument is really optional, although the doc at
    // `https://www.getmonero.org/resources/developer-guides/wallet-rpc.html#generate_from_keys` does not mention it
    pub password: String,
    pub autosave_current: Option<bool>,
}

/// Return sub-type of wallet `get_accounts`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GotAccount {
    pub account_index: u32,
    #[serde(with = "amount::serde::as_pico")]
    pub balance: Amount,
    pub base_address: monero::Address,
    pub label: Option<String>,
    pub tag: Option<String>,
    #[serde(with = "amount::serde::as_pico")]
    pub unlocked_balance: Amount,
}

/// Return type of wallet `refresh`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshData {
    pub blocks_fetched: u64,
    pub received_money: bool,
}

/// Return type of wallet `get_accounts`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetAccountsData {
    pub subaddress_accounts: Vec<GotAccount>,
    #[serde(with = "amount::serde::as_pico")]
    pub total_balance: Amount,
    #[serde(with = "amount::serde::as_pico")]
    pub total_unlocked_balance: Amount,
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
    pub account_index: Option<u32>,
    /// List of subaddress indices to query for transfers. (Defaults to empty - all indices)
    pub subaddr_indices: Option<Vec<u32>>,
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
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct GotTransfer {
    /// Public address of the transfer.
    pub address: Address,
    /// Amount transferred.
    #[serde(with = "amount::serde::as_pico")]
    pub amount: Amount,
    /// Number of block mined since the block containing this transaction (or block height at which the transaction should be added to a block if not yet confirmed).
    pub confirmations: Option<u64>,
    /// True if the key image(s) for the transfer have been seen before.
    pub double_spend_seen: bool,
    /// Transaction fee for this transfer.
    #[serde(with = "amount::serde::as_pico")]
    pub fee: Amount,
    /// Height of the first block that confirmed this transfer (0 if not mined yet).
    pub height: TransferHeight,
    /// Note about this transfer.
    pub note: String,
    /// Payment ID for this transfer.
    pub payment_id: HashString<PaymentId>,
    /// JSON object containing the major & minor subaddress index.
    pub subaddr_index: subaddress::Index,
    /// Estimation of the confirmations needed for the transaction to be included in a block.
    pub suggested_confirmations_threshold: Option<u64>,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedKeyImage {
    /// The key image.
    pub key_image: Vec<u8>,
    /// Signature of the key image.
    pub signature: Vec<u8>,
}

/// Return type of wallet `import_key_images`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeyImageImportResponse {
    pub height: u64,
    /// Amount spent from key images.
    #[serde(with = "amount::serde::as_pico")]
    pub spent: Amount,
    /// Amount still available from key images.
    #[serde(with = "amount::serde::as_pico")]
    pub unspent: Amount,
}

/// Return type of `create_wallet`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountCreation {
    /// Index of the new account.
    pub account_index: u32,
    /// Generated wallet address.
    pub address: Address,
}

/// Return type of `check_tx_proof`.
#[derive(Clone, Debug, Deserialize)]
pub struct TxProofOutput {
    /// Number of block mined after the one with the transaction.
    pub confirmations: u32,
    /// States if the inputs proves the transaction.
    pub good: bool,
    /// States if the transaction is still in pool or has been added to a block.
    pub in_pool: bool,
    /// Amount of the transaction.
    #[serde(with = "amount::serde::as_pico")]
    pub received: Amount,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monero_result_to_inner() {
        let monero_res = MoneroResult::OK(123);
        assert_eq!(monero_res.into_inner(), 123);
    }

    #[test]
    fn block_header_response_from_block_header_response_r() {
        let bhrr = BlockHeaderResponseR {
            block_size: 123,
            depth: 1234,
            difficulty: 12345,
            hash: HashString(BlockHash::zero()),
            height: 123456,
            major_version: 1234567,
            minor_version: 12345678,
            nonce: 123456789,
            num_txes: 1,
            orphan_status: true,
            prev_hash: HashString(BlockHash::repeat_byte(12)),
            reward: Amount::from_pico(12),
            timestamp: DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(61, 0).unwrap(),
                Utc,
            ),
        };

        let expected_bhr = BlockHeaderResponse {
            block_size: 123,
            depth: 1234,
            difficulty: 12345,
            hash: BlockHash::zero(),
            height: 123456,
            major_version: 1234567,
            minor_version: 12345678,
            nonce: 123456789,
            num_txes: 1,
            orphan_status: true,
            prev_hash: BlockHash::repeat_byte(12),
            reward: Amount::from_pico(12),
            timestamp: DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(61, 0).unwrap(),
                Utc,
            ),
        };

        assert_eq!(BlockHeaderResponse::from(bhrr), expected_bhr);
    }

    #[test]
    fn str_from_get_transfers_category() {
        use GetTransfersCategory::*;

        let g_in: &str = In.into();
        let g_out: &str = Out.into();
        let g_pending: &str = Pending.into();
        let g_failed: &str = Failed.into();
        let g_pool: &str = Pool.into();
        let g_block: &str = Block.into();

        assert_eq!(g_in, "in");
        assert_eq!(g_out, "out");
        assert_eq!(g_pending, "pending");
        assert_eq!(g_failed, "failed");
        assert_eq!(g_pool, "pool");
        assert_eq!(g_block, "block");
    }

    #[test]
    fn deserialize_for_transfer_height() {
        use serde_test::{assert_de_tokens, Token};

        let confirmed = TransferHeight::Confirmed(NonZeroU64::new(10).unwrap());
        let in_pool = TransferHeight::InPool;

        assert_de_tokens(&confirmed, &[Token::U64(10)]);
        assert_de_tokens(&in_pool, &[Token::U64(0)]);
    }

    #[test]
    fn generate_blocks_response_from_generate_blocks_response_r() {
        let gbrr = GenerateBlocksResponseR {
            height: 10,
            blocks: None,
        };
        let expected_gbr = GenerateBlocksResponse {
            height: 10,
            blocks: None,
        };
        assert_eq!(GenerateBlocksResponse::from(gbrr), expected_gbr);

        let block_hash = BlockHash::zero();

        let gbrr = GenerateBlocksResponseR {
            height: 10,
            blocks: Some(vec![HashString(block_hash)]),
        };
        let expected_gbr = GenerateBlocksResponse {
            height: 10,
            blocks: Some(vec![block_hash]),
        };
        assert_eq!(GenerateBlocksResponse::from(gbrr), expected_gbr);
    }
}
