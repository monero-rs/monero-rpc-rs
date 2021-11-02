[![Build Status](https://img.shields.io/github/workflow/status/monero-ecosystem/monero-rpc-rs/Continuous%20integration)](https://github.com/monero-ecosystem/monero-rpc-rs/blob/master/.github/workflows/main.yml)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
![MSRV](https://img.shields.io/badge/MSRV-1.51.0-blue)

# Monero Daemon & Wallet RPC

Monero daemon and wallet RPC written in asynchronous Rust :crab:.

## Example with tokio::test

```rust
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

## Release Notes

See [CHANGELOG.md](CHANGELOG.md).

## Licensing

The code in this project is licensed under the [Apache-2.0](LICENSE)
