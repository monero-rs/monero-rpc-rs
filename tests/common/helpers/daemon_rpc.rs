use monero::cryptonote::hash::Hash;
use monero_rpc::{DaemonRpcClient, TransactionsResponse};

pub async fn get_transactions(
    daemon_rpc: &DaemonRpcClient,
    txs_hashes: Vec<Hash>,
    expected_transactions_response: TransactionsResponse,
) {
    let transactions_response = daemon_rpc
        .get_transactions(txs_hashes, None, None)
        .await
        .unwrap();
    assert_eq!(transactions_response, expected_transactions_response);
}

pub async fn get_transactions_as_hex_not_pruned(
    daemon_rpc: &DaemonRpcClient,
    txs_hashes: Vec<Hash>,
    expected_transactions_response: TransactionsResponse,
) {
    let transactions_response_with_none = daemon_rpc
        .get_transactions(txs_hashes.clone(), None, None)
        .await
        .unwrap();

    let transactions_response_with_some_false = daemon_rpc
        .get_transactions(txs_hashes, Some(false), Some(false))
        .await
        .unwrap();

    assert_eq!(
        transactions_response_with_none,
        expected_transactions_response
    );
    assert_eq!(
        transactions_response_with_some_false,
        expected_transactions_response
    );
}

pub async fn get_transactions_as_hex_pruned(
    daemon_rpc: &DaemonRpcClient,
    txs_hashes: Vec<Hash>,
    expected_transactions_response: TransactionsResponse,
) {
    let transactions_response = daemon_rpc
        .get_transactions(txs_hashes, None, Some(true))
        .await
        .unwrap();
    assert_eq!(transactions_response, expected_transactions_response);
}

fn test_tx_json_not_empty(transactions_response: TransactionsResponse) {
    let txs_json = transactions_response.txs_as_json.unwrap();
    if txs_json.is_empty() {
        panic!("txs_as_json should not be empty");
    }
    let txs = transactions_response.txs.unwrap();
    let first_tx = &txs[0];
    let first_tx_as_json = first_tx.as_json.as_ref().unwrap();
    assert_ne!(first_tx_as_json, &"".to_string());
}

pub async fn get_transactions_as_json_not_pruned(
    daemon_rpc: &DaemonRpcClient,
    txs_hashes: Vec<Hash>,
) {
    let transactions_response_with_none = daemon_rpc
        .get_transactions(txs_hashes.clone(), Some(true), None)
        .await
        .unwrap();

    let transactions_response_with_some = daemon_rpc
        .get_transactions(txs_hashes, Some(true), Some(false))
        .await
        .unwrap();

    assert_eq!(
        transactions_response_with_none,
        transactions_response_with_some,
    );

    test_tx_json_not_empty(transactions_response_with_some);
}

pub async fn get_transactions_as_json_pruned(daemon_rpc: &DaemonRpcClient, txs_hashes: Vec<Hash>) {
    let transactions_response = daemon_rpc
        .get_transactions(txs_hashes, Some(true), Some(true))
        .await
        .unwrap();
    test_tx_json_not_empty(transactions_response);
}
