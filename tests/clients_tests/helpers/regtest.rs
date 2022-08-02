use std::ops::RangeInclusive;

use chrono::{DateTime, NaiveDate, Utc};
use monero::{Address, Network};
use monero_rpc::{
    BlockHash, BlockHeaderResponse, BlockTemplate, GenerateBlocksResponse, HashString,
    RegtestDaemonJsonRpcClient,
};
use serde::Deserialize;

pub async fn get_block_count_assert_height(
    regtest: &RegtestDaemonJsonRpcClient,
    expected_height: u64,
) {
    let count = regtest.get_block_count().await.unwrap();
    assert_eq!(count.get(), expected_height);
}

pub async fn on_get_block_hash_assert_hash(
    regtest: &RegtestDaemonJsonRpcClient,
    height: u64,
    expected_hash: BlockHash,
) {
    let block_hash = regtest.on_get_block_hash(height).await.unwrap();
    assert_eq!(block_hash, expected_hash);
}

pub async fn on_get_block_hash_error_invalid_height(
    regtest: &RegtestDaemonJsonRpcClient,
    height: u64,
) {
    let block_hash = regtest.on_get_block_hash(height).await.unwrap_err();
    assert_eq!(
        block_hash.to_string(),
        format!("Invalid height {height} supplied.")
    );
}

fn get_expected_height_returned_by_generate_blocks(
    start_block_count: u64,
    amount_of_blocks: u64,
) -> u64 {
    let height = start_block_count - 1;
    height + amount_of_blocks
}

pub async fn generate_blocks_assert_ok(
    regtest: &RegtestDaemonJsonRpcClient,
    amount_of_blocks: u64,
    wallet_address: Address,
) -> GenerateBlocksResponse {
    let start_block_count = regtest.get_block_count().await.unwrap().get();

    let res = regtest
        .generate_blocks(amount_of_blocks, wallet_address)
        .await
        .unwrap();
    let expected_height =
        get_expected_height_returned_by_generate_blocks(start_block_count, amount_of_blocks);
    assert_eq!(res.height, expected_height);
    assert!(res.blocks.is_some());

    let final_block_count = regtest.get_block_count().await.unwrap().get();
    assert_eq!(start_block_count + amount_of_blocks, final_block_count);

    res
}

// This is to demonstrate that, if `amount_of_blocks` is zero, then the RPC returns success even if
// the address is wrong.
pub async fn generate_zero_blocks_assert_ok(
    regtest: &RegtestDaemonJsonRpcClient,
    wallet_address: Address,
) {
    if let Network::Mainnet = wallet_address.network {
        panic!("generate_blocks_zero_blocks only accepts an address that is not in the Mainnet/Regtest format.")
    }

    let start_block_count = regtest.get_block_count().await.unwrap().get();

    let amount_of_blocks = 0;
    let res = regtest
        .generate_blocks(amount_of_blocks, wallet_address)
        .await
        .unwrap();

    let expected_height =
        get_expected_height_returned_by_generate_blocks(start_block_count, amount_of_blocks);

    assert_eq!(res.height, expected_height + 1);
    assert!(res.blocks.is_none());
    assert_eq!(
        start_block_count + amount_of_blocks,
        regtest.get_block_count().await.unwrap().get(),
    );
}

// We are on regtest, but the address used in this function is **not** a regtest address.
pub async fn generate_blocks_error_invalid_address(
    regtest: &RegtestDaemonJsonRpcClient,
    wallet_address: Address,
) {
    if let Network::Mainnet = wallet_address.network {
        panic!("generate_blocks_error_invalid_address only accepts an address that is not in the Mainnet/Regtest format.")
    }

    let err = regtest
        .generate_blocks(100, wallet_address)
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Server error: Failed to parse wallet address"
    );
}

pub async fn generate_blocks_error_subaddress_not_supported(
    regtest: &RegtestDaemonJsonRpcClient,
    wallet_address: Address,
) {
    let err = regtest
        .generate_blocks(100, wallet_address)
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Server error: Mining to subaddress is not supported yet"
    );
}

pub async fn get_block_template_assert_block_template(
    regtest: &RegtestDaemonJsonRpcClient,
    address: Address,
    reserve_size: u64,
    expected_block_template: BlockTemplate,
) {
    let mut res_block_template = regtest
        .get_block_template(address, reserve_size)
        .await
        .unwrap();

    // this field is not deterministic
    res_block_template.blockhashing_blob = HashString(vec![]);
    // this field is not deterministic
    res_block_template.blocktemplate_blob = HashString(vec![]);

    // since this may very, we change the response to whatever `expected_block_template` variable
    // has
    res_block_template.reserved_offset = expected_block_template.reserved_offset;

    assert_eq!(res_block_template, expected_block_template);
}

pub async fn get_block_template_error_invalid_reserve_size(
    regtest: &RegtestDaemonJsonRpcClient,
    address: Address,
) {
    let res_err = regtest.get_block_template(address, 256).await.unwrap_err();
    assert_eq!(
        res_err.to_string(),
        "Server error: Too big reserved size, maximum 255"
    );
}

pub async fn get_block_template_error_invalid_address(regtest: &RegtestDaemonJsonRpcClient) {
    let key_pair_1 = super::get_keypair_1();
    let address_testnet = Address::from_keypair(Network::Testnet, &key_pair_1);
    let res_err = regtest
        .get_block_template(address_testnet, 10)
        .await
        .unwrap_err();
    assert_eq!(
        res_err.to_string(),
        "Server error: Failed to parse wallet address"
    );
}

pub async fn submit_block_assert_ok(
    regtest: &RegtestDaemonJsonRpcClient,
    block_template_blob: HashString<Vec<u8>>,
) {
    let start_block_count = regtest.get_block_count().await.unwrap().get();
    regtest
        .submit_block(block_template_blob.to_string())
        .await
        .unwrap();
    assert_eq!(
        start_block_count + 1,
        regtest.get_block_count().await.unwrap().get()
    );

    // submitting same blob again returns success but does not increase block count
    regtest
        .submit_block(block_template_blob.to_string())
        .await
        .unwrap();
    assert_eq!(
        start_block_count + 1,
        regtest.get_block_count().await.unwrap().get()
    );
}

pub async fn submit_block_error_wrong_block_blob(regtest: &RegtestDaemonJsonRpcClient) {
    let block_template_blob = "0123456789";

    let res_err = regtest
        .submit_block(block_template_blob.to_string())
        .await
        .unwrap_err();
    assert_eq!(res_err.to_string(), "Server error: Wrong block blob");
}

pub async fn submit_block_error_block_not_accepted(regtest: &RegtestDaemonJsonRpcClient) {
    let block_template_blob = "0707e6bdfedc053771512f1bc27c62731ae9e8f2443db64ce742f4e57f5cf8d393de28551e441a0000000002fb830a01ffbf830a018cfe88bee283060274c0aae2ef5730e680308d9c00b6da59187ad0352efe3c71d36eeeb28782f29f2501bd56b952c3ddc3e350c2631d3a5086cac172c56893831228b17de296ff4669de020200000000";
    let res_err = regtest
        .submit_block(block_template_blob.to_string())
        .await
        .unwrap_err();
    assert_eq!(res_err.to_string(), "Server error: Block not accepted");
}

fn test_get_block_header_assert_block_header(
    block_header: BlockHeaderResponse,
    expected_block_header: BlockHeaderResponse,
) {
    #[derive(Debug, PartialEq, Deserialize)]
    // `block_size` is not tested because it varies
    struct Helper {
        depth: u64,
        difficulty: u64,
        hash: BlockHash,
        height: u64,
        nonce: u32,
        num_txes: u64,
        orphan_status: bool,
        prev_hash: BlockHash,
        reward: u64,
    }

    if block_header.height == 0 {
        assert_eq!(block_header.timestamp, expected_block_header.timestamp);
    } else {
        let start_2022_date = NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0);
        let start_2022_date = DateTime::<Utc>::from_utc(start_2022_date, Utc);
        assert!(block_header.timestamp >= start_2022_date);
    }

    let v = serde_json::to_value(&block_header).unwrap();
    let helper_block_header: Helper = serde_json::from_value(v).unwrap();

    let v = serde_json::to_value(&expected_block_header).unwrap();
    let helper_expected_block_header: Helper = serde_json::from_value(v).unwrap();

    assert_eq!(helper_block_header, helper_expected_block_header);
}

pub async fn get_last_block_header_assert_block_header(
    regtest: &RegtestDaemonJsonRpcClient,
    expected_block_header: BlockHeaderResponse,
) {
    let block_header = regtest
        .get_block_header(monero_rpc::GetBlockHeaderSelector::Last)
        .await
        .unwrap();
    test_get_block_header_assert_block_header(block_header, expected_block_header);
}

pub async fn get_block_header_from_block_hash_assert_block_header(
    regtest: &RegtestDaemonJsonRpcClient,
    block_hash: BlockHash,
    expected_block_header: BlockHeaderResponse,
) {
    let block_header = regtest
        .get_block_header(monero_rpc::GetBlockHeaderSelector::Hash(block_hash))
        .await
        .unwrap();
    test_get_block_header_assert_block_header(block_header, expected_block_header);
}

pub async fn get_block_header_from_block_hash_error_not_found(
    regtest: &RegtestDaemonJsonRpcClient,
    block_hash: BlockHash,
) {
    let block_header_err = regtest
        .get_block_header(monero_rpc::GetBlockHeaderSelector::Hash(block_hash))
        .await
        .unwrap_err();
    assert_eq!(
        block_header_err.to_string(),
        format!(
            "Server error: Internal error: can't get block by hash. Hash = {:x}.",
            block_hash
        )
    );
}

pub async fn get_block_header_at_height_assert_block_header(
    regtest: &RegtestDaemonJsonRpcClient,
    height: u64,
    expected_block_header: BlockHeaderResponse,
) {
    let block_header = regtest
        .get_block_header(monero_rpc::GetBlockHeaderSelector::Height(height))
        .await
        .unwrap();
    test_get_block_header_assert_block_header(block_header, expected_block_header);
}

pub async fn get_block_header_at_height_error(
    regtest: &RegtestDaemonJsonRpcClient,
    height: u64,
    current_top_block_height: u64,
) {
    let block_header_err = regtest
        .get_block_header(monero_rpc::GetBlockHeaderSelector::Height(height))
        .await
        .unwrap_err();
    assert_eq!(
        block_header_err.to_string(),
        format!(
            "Server error: Requested block height: {height} greater than current top block height: {current_top_block_height}"
        )
    );
}

pub async fn get_block_headers_range_assert_block_headers(
    regtest: &RegtestDaemonJsonRpcClient,
    range: RangeInclusive<u64>,
    expected_block_headers: Vec<BlockHeaderResponse>,
) {
    let (block_headers, _) = regtest.get_block_headers_range(range).await.unwrap();
    assert_eq!(block_headers, expected_block_headers);
}

pub async fn get_block_headers_range_error(
    regtest: &RegtestDaemonJsonRpcClient,
    range: RangeInclusive<u64>,
) {
    let block_headers_err = regtest.get_block_headers_range(range).await.unwrap_err();
    assert_eq!(
        block_headers_err.to_string(),
        "Server error: Invalid start/end heights."
    );
}
