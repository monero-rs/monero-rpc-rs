use monero::{
    cryptonote::subaddress::{self, Index},
    Address, Amount, Network, ViewPair,
};
use monero_rpc::{AddressData, GenerateFromKeysArgs, GetAccountsData, GotAccount, SubaddressData};

use super::helpers;

/*
* The purpose of this test is to test function from the `WalletClient`
* (i.e. from https://www.getmonero.org/resources/developer-guides/wallet-rpc.html)
* that modify the wallet, but that do **not** modify the blockchain and that do not depend
* on the state of the blockchain. For example, functions that create wallets are tested,
* but functions that create transfers or that call `get_height` are not tested.
*
* The steps of the test are described below.
*/

pub async fn run() {
    let (_, _, wallet) = helpers::setup_monero();

    // STEP 1: we test the version of the software running, and then we test
    // creating wallets, opening them, closing them, etc, and also test scenarios
    // where errors could happen.
    // Note that the wallets created in this step are created "from scratch", i.e.
    // they are not created from known spend/view keys.
    let expected_wallet_version = (1, 25);
    helpers::wallet::get_version_assert_version(&wallet, expected_wallet_version).await;

    let (wallet_with_pwd, wallet_with_no_pwd, wallet_with_empty_pwd) = tokio::join!(
        helpers::wallet::create_wallet_with_password_assert_ok(&wallet, helpers::PWD_1),
        helpers::wallet::create_wallet_with_no_password_parameter_assert_ok(&wallet),
        helpers::wallet::create_wallet_with_empty_password_assert_ok(&wallet),
    );
    helpers::wallet::create_wallet_error_already_exists(&wallet, &wallet_with_pwd).await;
    helpers::wallet::create_wallet_error_invalid_language(&wallet).await;

    // close opened wallet, and then call `close_wallet` again.
    helpers::wallet::close_wallet_assert_ok(&wallet).await;
    helpers::wallet::close_wallet_error_no_wallet_file(&wallet).await;

    // open same wallet twice
    helpers::wallet::open_wallet_with_password_assert_ok(&wallet, &wallet_with_pwd, helpers::PWD_1)
        .await;
    helpers::wallet::open_wallet_with_password_assert_ok(&wallet, &wallet_with_pwd, helpers::PWD_1)
        .await;

    helpers::wallet::open_wallet_with_no_or_empty_password_assert_ok(&wallet, &wallet_with_no_pwd)
        .await;
    helpers::wallet::open_wallet_with_no_or_empty_password_assert_ok(
        &wallet,
        &wallet_with_empty_pwd,
    )
    .await;

    helpers::wallet::open_wallet_error_filename_invalid(&wallet, "troll_wallet").await;
    helpers::wallet::open_wallet_error_wrong_password(&wallet, &wallet_with_pwd, None).await;
    helpers::wallet::open_wallet_error_wrong_password(
        &wallet,
        &wallet_with_no_pwd,
        Some("wrong_password :)".to_string()),
    )
    .await;

    // STEP 2: we create wallets from known spend/view keys. From that, we create
    // view-only and spend ("full") wallets. At the same time, we test possible
    // scenarios that could cause errors.
    let key_pair_1 = helpers::get_keypair_1();
    let generate_wallet_args_1 = GenerateFromKeysArgs {
        restore_height: None,
        filename: "".to_string(), // empty because will be generated by the below function call
        address: Address::from_keypair(Network::Mainnet, &key_pair_1),
        spendkey: None,
        viewkey: key_pair_1.view,
        password: "".to_string(),
        autosave_current: None,
    };
    let _ = helpers::wallet::generate_from_keys_assert_ok(&wallet, generate_wallet_args_1.clone())
        .await;

    // creating wallet again, but with a different name, causes no error; note
    // that the "different name" is because `generate_from_keys` creates random names
    // for wallets each time it is called.
    let wallet_creation_from_key_pair_1 =
        helpers::wallet::generate_from_keys_assert_ok(&wallet, generate_wallet_args_1).await;

    let key_pair_2 = helpers::get_keypair_2();
    let generate_wallet_args_2 = GenerateFromKeysArgs {
        restore_height: Some(0),
        filename: "".to_string(), // empty because will be generated by the below function call
        address: Address::from_keypair(Network::Mainnet, &key_pair_2),
        spendkey: Some(key_pair_2.spend),
        viewkey: key_pair_2.view,
        password: helpers::PWD_1.to_string(),
        autosave_current: Some(false),
    };
    let wallet_creation_from_key_pair_2 =
        helpers::wallet::generate_from_keys_assert_ok(&wallet, generate_wallet_args_2).await;

    let key_pair_3 = helpers::get_keypair_3();
    let generate_wallet_args_3 = GenerateFromKeysArgs {
        restore_height: None,
        filename: wallet_with_empty_pwd,
        address: Address::from_keypair(Network::Mainnet, &key_pair_3),
        spendkey: Some(key_pair_3.spend),
        viewkey: key_pair_3.view,
        password: "".to_string(),
        autosave_current: None,
    };
    helpers::wallet::generate_from_keys_error_filename_already_exists(
        &wallet,
        generate_wallet_args_3,
    )
    .await;

    let generate_wallet_args_3 = GenerateFromKeysArgs {
        // invalid `restore_height` returns no error
        restore_height: Some(u64::MAX),
        filename: "".to_string(), // empty because will be generated by the below function call
        address: Address::from_keypair(Network::Mainnet, &key_pair_3),
        spendkey: Some(key_pair_3.spend),
        viewkey: key_pair_3.view,
        password: "".to_string(),
        autosave_current: None,
    };
    let _ = helpers::wallet::generate_from_keys_assert_ok(&wallet, generate_wallet_args_3).await;

    // generate wallet from invalid address (we are in `mainnet/regtest`, but address is a `testnet` one).
    let generate_wallet_args_3 = GenerateFromKeysArgs {
        restore_height: None,
        filename: "".to_string(), // empty because will be generated by the below function call
        address: Address::from_keypair(Network::Testnet, &key_pair_3),
        spendkey: Some(key_pair_3.spend),
        viewkey: key_pair_3.view,
        password: "".to_string(),
        autosave_current: None,
    };
    helpers::wallet::generate_from_keys_error_invalid_address(&wallet, generate_wallet_args_3)
        .await;

    // STEP 3: from here on, we test functions related to a wallet's functionality; for
    // example: creating accounts, (sub)addresses, getting them, etc.
    helpers::wallet::close_wallet_assert_ok(&wallet).await;
    helpers::wallet::get_address_error_no_wallet_file(&wallet).await;

    helpers::wallet::open_wallet_with_no_or_empty_password_assert_ok(
        &wallet,
        &wallet_creation_from_key_pair_1.0,
    )
    .await;
    helpers::wallet::get_address_error_invalid_account_index(&wallet, 10).await;
    helpers::wallet::get_address_error_invalid_address_index(&wallet, 0, Some(vec![10])).await;

    let expected_get_address_from_key_pair_1_subaddress_data = SubaddressData {
        address: wallet_creation_from_key_pair_1.1.address,
        address_index: 0,
        label: "Primary account".to_string(),
        used: false, // this field is not tested inside the test functions because it varies
    };
    let expected_get_address_from_key_pair_1 = AddressData {
        address: wallet_creation_from_key_pair_1.1.address,
        addresses: vec![
            expected_get_address_from_key_pair_1_subaddress_data.clone(),
            expected_get_address_from_key_pair_1_subaddress_data.clone(),
            expected_get_address_from_key_pair_1_subaddress_data,
        ],
    };
    helpers::wallet::get_address_assert_address_data(
        &wallet,
        0,
        Some(vec![0, 0, 0]),
        expected_get_address_from_key_pair_1,
    )
    .await;

    helpers::wallet::get_address_index_assert_index(
        &wallet,
        wallet_creation_from_key_pair_1.1.address,
        Index { major: 0, minor: 0 },
    )
    .await;

    // get address from a wallet that is not the one currently opened
    helpers::wallet::get_address_index_error_address_from_another_wallet(
        &wallet,
        wallet_creation_from_key_pair_2.1.address,
    )
    .await;
    helpers::wallet::get_address_index_error_invalid_address(
        &wallet,
        Address::from_keypair(Network::Testnet, &key_pair_1),
    )
    .await;

    // open a different wallet for the next few tests
    helpers::wallet::open_wallet_with_password_assert_ok(
        &wallet,
        &wallet_creation_from_key_pair_2.0,
        helpers::PWD_1,
    )
    .await;

    let expected_first_new_address = (
        subaddress::get_subaddress(
            &ViewPair::from(&key_pair_2),
            subaddress::Index { major: 0, minor: 1 },
            Some(Network::Mainnet),
        ),
        1,
    );
    let first_new_address_for_wallet_from_key_pair_2 =
        helpers::wallet::create_address_assert_address_and_address_index(
            &wallet,
            0,
            None,
            expected_first_new_address,
        )
        .await;

    let expected_second_new_address = (
        subaddress::get_subaddress(
            &ViewPair::from(&key_pair_2),
            subaddress::Index { major: 0, minor: 2 },
            Some(Network::Mainnet),
        ),
        2,
    );
    let second_new_address_for_wallet_from_key_pair_2 =
        helpers::wallet::create_address_assert_address_and_address_index(
            &wallet,
            0,
            Some("new_label".to_string()),
            expected_second_new_address,
        )
        .await;
    helpers::wallet::create_address_error_invalid_account_index(&wallet, 10).await;

    helpers::wallet::label_address_assert_ok(
        &wallet,
        Index {
            major: 0,
            minor: first_new_address_for_wallet_from_key_pair_2.1,
        },
        "haha label :)".to_string(),
    )
    .await;
    helpers::wallet::label_address_assert_ok(
        &wallet,
        Index {
            major: 0,
            minor: second_new_address_for_wallet_from_key_pair_2.1,
        },
        "".to_string(),
    )
    .await;

    helpers::wallet::label_address_error_invalid_account_index(
        &wallet,
        Index {
            major: 10,
            minor: 0,
        },
    )
    .await;
    helpers::wallet::label_address_error_invalid_address_index(
        &wallet,
        Index {
            major: 0,
            minor: 10,
        },
    )
    .await;

    let expected_got_account_main_address = GotAccount {
        account_index: 0,
        balance: Amount::from_pico(0),
        base_address: Address::from_keypair(Network::Mainnet, &key_pair_2),
        label: Some("Primary account".to_string()),
        tag: Some("".to_string()),
        unlocked_balance: Amount::from_pico(0),
    };
    helpers::wallet::get_accounts_assert_accounts_data(
        &wallet,
        None,
        GetAccountsData {
            subaddress_accounts: vec![expected_got_account_main_address],
            total_balance: Amount::from_pico(0),
            total_unlocked_balance: Amount::from_pico(0),
        },
    )
    .await;
    helpers::wallet::get_accounts_error_unregistered_tag(
        &wallet,
        "no_account_with this_tag".to_string(),
    )
    .await;

    helpers::wallet::close_wallet_assert_ok(&wallet).await;
}
