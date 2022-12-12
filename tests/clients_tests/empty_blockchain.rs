use chrono::{DateTime, NaiveDateTime, Utc};
use monero::{Address, Amount, Network};
use monero_rpc::{BlockHash, BlockHeaderResponse, BlockTemplate, HashString};

use super::helpers;

/*
* The purpose of this test is to test functions from the `DaemonJsonRpcClient`
* (i.e, functions from https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods)
* ON A FRESH BLOCKCHAIN (i.e. with only the genesis block), and such functions
* tested must NOT modify the state of the blockchain.
*
* The steps in the test are pretty straightforward.
*/

pub async fn run() {
    let (regtest, _, _) = helpers::setup_monero();

    let genesis_block_hash = helpers::get_genesis_block_hash();

    helpers::regtest::get_block_count_assert_height(&regtest, 1).await;
    helpers::regtest::on_get_block_hash_error_invalid_height(&regtest, 10).await;
    helpers::regtest::on_get_block_hash_assert_hash(&regtest, 0, genesis_block_hash).await;

    let key_pair_1 = helpers::get_keypair_1();
    let address_1 = Address::from_keypair(Network::Mainnet, &key_pair_1);

    helpers::regtest::get_block_template_assert_block_template(
        &regtest,
        address_1,
        10,
        BlockTemplate {
            // this field is not deterministic, so set it to empty vec
            blockhashing_blob: HashString(vec![]),
            // this field is not deterministic, so set it to empty vec
            blocktemplate_blob: HashString(vec![]),
            difficulty: 1,
            expected_reward: Amount::from_pico(35184338534400),
            height: 1,
            prev_hash: HashString(genesis_block_hash),
            reserved_offset: 185, // may very, so not used inside the test
            untrusted: false,
        },
    )
    .await;
    helpers::regtest::get_block_template_error_invalid_reserve_size(&regtest, address_1).await;
    helpers::regtest::get_block_template_error_invalid_address(&regtest).await;

    let genesis_block_header = BlockHeaderResponse {
        block_size: 80,
        depth: 0,
        difficulty: 1,
        hash: genesis_block_hash,
        height: 0,
        major_version: 1,
        minor_version: 0,
        nonce: 10000,
        num_txes: 0,
        orphan_status: false,
        prev_hash: BlockHash::zero(),
        reward: Amount::from_pico(17592186044415),
        // this **is** used inside the test functions, since this block header corresponds
        // to the genesis block;
        // note that in the `non_empty_blockchain`, this field is **not** tested.
        timestamp: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp_opt(0, 0).unwrap(), Utc),
    };

    helpers::regtest::get_last_block_header_assert_block_header(
        &regtest,
        genesis_block_header.clone(),
    )
    .await;
    helpers::regtest::get_block_header_from_block_hash_assert_block_header(
        &regtest,
        genesis_block_hash,
        genesis_block_header.clone(),
    )
    .await;

    helpers::regtest::get_block_header_from_block_hash_error_not_found(
        &regtest,
        BlockHash::from_slice(&[0; 32]),
    )
    .await;
    helpers::regtest::get_block_header_from_block_hash_error_not_found(
        &regtest,
        BlockHash::from_slice(&[42; 32]),
    )
    .await;

    let current_top_block_height = regtest.get_block_count().await.unwrap().get() - 1;
    helpers::regtest::get_block_header_at_height_assert_block_header(
        &regtest,
        0,
        genesis_block_header.clone(),
    )
    .await;
    helpers::regtest::get_block_header_at_height_error(&regtest, 10, current_top_block_height)
        .await;

    helpers::regtest::get_block_headers_range_assert_block_headers(
        &regtest,
        0..=0,
        vec![genesis_block_header],
    )
    .await;
    helpers::regtest::get_block_headers_range_error(&regtest, 0..=4).await;
    helpers::regtest::get_block_headers_range_error(&regtest, 2..=4).await;

    // we deny the next lint because, in this case, it does not make sense; this is because,
    // when the `RangeInclusive<u64>` gets passed to the `get_block_headers_range` function in `src/lib.rs`,
    // the function will call the `start` and `end` methods on the range, which will return `4` and
    // `0`, respectively. Such values will then be passed to the RPC call as `start_height` and
    // `end_height` params, and the RPC should then return an error.
    #[allow(clippy::reversed_empty_ranges)]
    helpers::regtest::get_block_headers_range_error(&regtest, 4..=0).await;
}
