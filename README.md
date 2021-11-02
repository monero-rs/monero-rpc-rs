# monero-rpc

[![GitHub Actions workflow status](https://github.com/vorot93/monero-rpc-rs/workflows/Continuous%20integration/badge.svg)](https://github.com/vorot93/monero-rpc-rs/actions)

Monero daemon and wallet RPC.

License: MIT OR Apache-2.0

## Example with tokio::test

```
#[tokio::test]
async fn monero_daemon_transactions_test() {
    let tx_id = "7c50844eced8ab78a8f26a126fbc1f731134e0ae3e6f9ba0f205f98c1426ff60".to_string();
    let daemon_client = RpcClient::new("http://node.monerooutreach.org:18081".to_string());
    let daemon = daemon_client.daemon_rpc();
    let mut fixed_hash: [u8; 32] = [0; 32];
    hex::decode_to_slice(tx_id, &mut fixed_hash).unwrap();
    let tx = daemon
        .get_transactions(vec![fixed_hash.into()], Some(true), Some(true))
        .await;
    println!("tx {:?}", tx);
    println!(
        "unlock time: {:?}",
        serde_json::from_str::<JsonTransaction>(&tx.unwrap().txs_as_json.unwrap()[0])
    );
}
```
