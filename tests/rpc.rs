use monero::{Address, Amount};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

#[tokio::test]
async fn functional_daemon_test() {
    let addr_str = "4AdUndXHHZ6cfufTMvppY6JwXNouMBzSkbLYfpAV5Usx3skxNgYeYTRj5UzqtReoS44qo9mtmXCqY45DJ852K5Jv2684Rge";
    let (regtest, _) = setup_monero();
    let address = Address::from_str(addr_str).unwrap();
    regtest.get_block_template(address, 60).await.unwrap();
    regtest.get_block_count().await.unwrap();
    regtest.on_get_block_hash(1).await.unwrap();
    regtest
        .get_block_header(monero_rpc::GetBlockHeaderSelector::Last)
        .await
        .unwrap();
    regtest.generate_blocks(4, address).await.unwrap();
    regtest
        .get_block_headers_range(std::ops::RangeInclusive::new(1, 2))
        .await
        .unwrap();
}

#[tokio::test]
async fn functional_wallet_test() {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    let spend_wallet_name: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();
    let view_wallet_name: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();

    let (regtest, wallet) = setup_monero();
    match wallet
        .create_wallet(spend_wallet_name.clone(), None, "English".to_string())
        .await
    {
        Ok(_) => {}
        Err(err) => {
            assert_eq!(
                format!("{}", err),
                "Server error: Cannot create wallet. Already exists."
            );
        }
    }
    wallet
        .open_wallet(spend_wallet_name.clone(), None)
        .await
        .unwrap();
    wallet.get_balance(1, Some(vec![0])).await.unwrap();
    let address = wallet.get_address(0, Some(vec![0])).await.unwrap().address;
    wallet.get_address_index(address).await.unwrap();
    wallet
        .create_address(0, Some("new_label".to_string()))
        .await
        .unwrap();
    wallet
        .label_address(0, 0, "other_label".to_string())
        .await
        .unwrap();
    wallet.get_accounts(None).await.unwrap();
    wallet.get_height().await.unwrap();
    wallet.get_version().await.unwrap();

    regtest.generate_blocks(500, address).await.unwrap();
    wallet.refresh(Some(0)).await.unwrap();

    let mut destination: HashMap<Address, Amount> = HashMap::new();
    destination.insert(address, Amount::from_xmr(0.00001).unwrap());

    let transfer_options = monero_rpc::TransferOptions {
        account_index: None,
        subaddr_indices: None,
        mixin: Some(10),
        ring_size: Some(11),
        unlock_time: Some(0),
        payment_id: None,
        do_not_relay: Some(true),
    };

    let transfer_data = wallet
        .transfer(
            destination,
            monero_rpc::TransferPriority::Default,
            transfer_options,
        )
        .await
        .unwrap();

    wallet
        .relay_tx(transfer_data.tx_metadata.to_string())
        .await
        .unwrap();

    let res = wallet
        .check_tx_key(transfer_data.tx_hash.0, transfer_data.tx_key.0, address)
        .await;

    wallet.export_key_images().await.unwrap();

    wallet
        .query_key(monero_rpc::PrivateKeyType::Spend)
        .await
        .unwrap();
    let viewkey = wallet
        .query_key(monero_rpc::PrivateKeyType::View)
        .await
        .unwrap();

    match wallet
        .generate_from_keys(monero_rpc::GenerateFromKeysArgs {
            restore_height: Some(0),
            filename: view_wallet_name.clone(),
            address,
            spendkey: None,
            viewkey,
            password: "".to_string(),
            autosave_current: None,
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            assert_eq!(format!("{}", err), "Server error: Wallet already exists.");
        }
    }

    wallet
        .open_wallet(view_wallet_name.clone(), None)
        .await
        .unwrap();
    wallet.export_key_images().await.unwrap();

    wallet.refresh(Some(0)).await.unwrap();

    wallet
        .incoming_transfers(monero_rpc::TransferType::All, Some(0), Some(vec![0, 1, 2]))
        .await
        .unwrap();

    use monero_rpc::{GetTransfersCategory, GetTransfersSelector};

    let mut category_selector: HashMap<GetTransfersCategory, bool> = HashMap::new();
    category_selector.insert(GetTransfersCategory::In, true);
    category_selector.insert(GetTransfersCategory::Out, true);
    category_selector.insert(GetTransfersCategory::Pending, true);
    category_selector.insert(GetTransfersCategory::Pool, true);

    let selector = GetTransfersSelector {
        category_selector,
        subaddr_indices: None,
        account_index: None,
        block_height_filter: Some(monero_rpc::BlockHeightFilter {
            min_height: Some(0),
            max_height: None,
        }),
    };

    wallet.get_transfers(selector).await.unwrap();

    let mut destination: HashMap<Address, Amount> = HashMap::new();
    destination.insert(address, Amount::from_xmr(0.00001).unwrap());

    let transfer_options = monero_rpc::TransferOptions {
        account_index: Some(0),
        subaddr_indices: Some(vec![0]),
        mixin: Some(10),
        ring_size: Some(11),
        unlock_time: Some(0),
        payment_id: None,
        do_not_relay: Some(true),
    };

    let transfer_data = wallet
        .transfer(
            destination,
            monero_rpc::TransferPriority::Default,
            transfer_options,
        )
        .await
        .unwrap();

    wallet
        .open_wallet(spend_wallet_name.clone(), None)
        .await
        .unwrap();
    wallet.refresh(Some(0)).await.unwrap();

    let sweep_args = monero_rpc::SweepAllArgs {
        address,
        account_index: 0,
        subaddr_indices: None,
        priority: monero_rpc::TransferPriority::Default,
        mixin: 10,
        ring_size: 11,
        unlock_time: 0,
        get_tx_keys: None,
        below_amount: None,
        do_not_relay: None,
        get_tx_hex: None,
        get_tx_metadata: None,
    };
    wallet.sweep_all(sweep_args).await.unwrap();

    let res = wallet
        .sign_transfer(transfer_data.unsigned_txset.0)
        .await
        .unwrap();
    println!("res: {:?}", res);
}

fn setup_monero() -> (monero_rpc::RegtestDaemonClient, monero_rpc::WalletClient) {
    let dhost = env::var("MONERO_DAEMON_HOST").unwrap_or("localhost".into());
    let daemon_client = monero_rpc::RpcClient::new(format!("http://{}:18081", dhost));
    let daemon = daemon_client.daemon();
    let regtest = daemon.regtest();
    let whost = env::var("MONERO_WALLET_HOST_1").unwrap_or("localhost".into());
    let wallet_client = monero_rpc::RpcClient::new(format!("http://{}:18083", whost));
    let wallet = wallet_client.wallet();
    (regtest, wallet)
}
