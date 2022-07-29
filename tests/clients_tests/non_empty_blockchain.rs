use chrono::{DateTime, NaiveDateTime, Utc};
use monero::{Address, Amount, Network};
use monero_rpc::{BlockHash, BlockHeaderResponse, GetBlockHeaderSelector};

use super::helpers;

/*
* The purpose of this test is to test functions from the `DaemonJsonRpcClient`
* (i.e, functions from https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods)
* that modify the blockchain (namely, `generate_blocks` and `submit_block`), and then test
* the state of the blockchain before and after blocks are added.
*
* The steps of the test are explained below.
*/

pub async fn run() {
    let (regtest, _, _) = helpers::setup_monero();

    let key_pair_1 = helpers::get_keypair_1();

    // STEP 1: we get a keypair and create a `testnet` address from it; however,
    // we are running in `regtest` mode, whose addresses have the same format as `mainnet`
    // addresses. Thus, `generate_blocks` should return an error when trying to generate blocks
    // and giving the coins created to a `testnet` address.
    //
    // However, there is an interesting catch: if the amount of blocks to be created
    // is zero, then passing a `testnet` address does not throw an error.
    let address_testnet = Address::from_keypair(Network::Testnet, &key_pair_1);
    helpers::regtest::generate_blocks_error_invalid_address(&regtest, address_testnet).await;
    helpers::regtest::generate_blocks_zero_blocks(&regtest, address_testnet).await;

    // STEP 2: we generate blocks and give the coins to a `mainnet/regtest` address.
    // We then test the blockchain state after the blocks are created, and we get
    // the last two blocks created and call functions on the RPC using their hashes, test
    // their heights, etc.
    let address_1 = Address::from_keypair(Network::Mainnet, &key_pair_1);
    let generate_blocks_res = helpers::regtest::generate_blocks(&regtest, 60, address_1).await;

    let last_two_added_blocks: Vec<BlockHash> = generate_blocks_res
        .blocks
        .unwrap()
        .into_iter()
        .rev()
        .take(2)
        .collect();
    let last_added_block_hash = last_two_added_blocks[0];
    let last_but_one_added_block_hash = last_two_added_blocks[1];

    helpers::regtest::on_get_block_hash_error_invalid_height(
        &regtest,
        generate_blocks_res.height + 1,
    )
    .await;
    helpers::regtest::on_get_block_hash(
        &regtest,
        generate_blocks_res.height,
        last_added_block_hash,
    )
    .await;

    let last_added_block_header = BlockHeaderResponse {
        // `block_size` is not tested inside the test functions below because it varies
        block_size: 85,
        depth: 0,
        difficulty: 1,
        hash: last_added_block_hash,
        height: regtest.get_block_count().await.unwrap().get() - 1,
        major_version: 16,
        minor_version: 16,
        nonce: 0,
        num_txes: 0,
        orphan_status: false,
        prev_hash: last_but_one_added_block_hash,
        reward: Amount::from_pico(35180379334199),
        // this is not used inside the test functions below, since its value depend on when the
        // test was run, so use any date in this field since it is insignificant for testing.
        timestamp: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
    };
    helpers::regtest::get_last_block_header(&regtest, last_added_block_header.clone()).await;
    helpers::regtest::get_block_header_from_block_hash(
        &regtest,
        last_added_block_hash,
        last_added_block_header.clone(),
    )
    .await;

    let current_top_block_height = regtest.get_block_count().await.unwrap().get() - 1;

    helpers::regtest::get_block_header_at_height(&regtest, 60, last_added_block_header).await;
    helpers::regtest::get_block_header_at_height_error(
        &regtest,
        u64::MAX,
        current_top_block_height,
    )
    .await;

    let last_block_header = regtest
        .get_block_header(GetBlockHeaderSelector::Height(current_top_block_height))
        .await
        .unwrap();
    let last_but_one_block_header = regtest
        .get_block_header(GetBlockHeaderSelector::Height(current_top_block_height - 1))
        .await
        .unwrap();
    helpers::regtest::get_block_headers_range(
        &regtest,
        59..=60,
        vec![last_but_one_block_header, last_block_header],
    )
    .await;

    // STEP 3: we test the last function that can modify the blockchain state: `submit_block`.
    // In order for it to work, we just get a block template on which to mine. Since the difficulty
    // of the network is `1`, any correct block template should be accepted by `submit_block`.
    let block_template = regtest.get_block_template(address_1, 0).await.unwrap();
    helpers::regtest::submit_block(&regtest, block_template.blocktemplate_blob).await;
    helpers::regtest::submit_block_error_wrong_block_blob(&regtest).await;
    helpers::regtest::submit_block_error_block_not_accepted(&regtest).await;
}
