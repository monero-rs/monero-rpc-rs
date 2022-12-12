[![Build Status](https://img.shields.io/github/workflow/status/monero-rs/monero-rpc-rs/Build)](https://github.com/monero-rs/monero-rpc-rs/blob/master/.github/workflows/build.yml)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Crates.io](https://img.shields.io/crates/v/monero-rpc.svg)](https://crates.io/crates/monero-rpc)
[![Documentation](https://docs.rs/monero-rpc/badge.svg)](https://docs.rs/monero-rpc)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![MSRV](https://img.shields.io/badge/MSRV-1.56.1-blue)](https://blog.rust-lang.org/2021/11/01/Rust-1.56.1.html)

# Monero Daemon & Wallet RPC

Monero daemon and wallet RPC written in asynchronous Rust :crab:.

## Example with `tokio::test`

Create the RPC client and transform it into a deamon RPC to call `/get_transactions` method and print the result.

```rust
use monero_rpc::{RpcClientBuilder, JsonTransaction};

#[tokio::test]
async fn monero_daemon_transactions_test() {
    let tx_id = "7c50844eced8ab78a8f26a126fbc1f731134e0ae3e6f9ba0f205f98c1426ff60".to_string();
    let rpc_client = monero_rpc::RpcClientBuilder::new()
        .build("http://node.monerooutreach.org:18081")
        .unwrap();
    let daemon_rpc_client = rpc_client.daemon_rpc();
    let mut fixed_hash: [u8; 32] = [0; 32];
    hex::decode_to_slice(tx_id, &mut fixed_hash).unwrap();
    let tx = daemon_rpc_client
        .get_transactions(vec![fixed_hash.into()], Some(true), Some(true))
        .await;
    println!("tx {:?}", tx);
    println!(
        "unlock time: {:?}",
        serde_json::from_str::<JsonTransaction>(&tx.unwrap().txs_as_json.unwrap()[0])
    );
}
```

## Testing

First, you'll need `docker` and `docker compose` to run the RPC integration tests, which are in `tests/`, in case you don't want to run `monerod` and `monero-wallet-rpc` on your own.

If you have the docker stack installed, go to the `tests` folder and run `docker compose up`. Note that the daemon will run on port `18081` and `monero-wallet-rpc` will run on port `18083`.

After that, just run `cargo test` as you normally would.

Also, you can run `docker compose down` to stop and remove the two containers started by `docker compose up`.

**Important**: the blockchain must be empty when running the `main_functional_test` test on `tests/rpc.rs`, i.e. it must have only the genesis block. In `regtest`, the blockchain restarts when `monerod` restarts (as a side note, if you want to keep the blockchain in `regtest` between restarts, you should pass the `--keep-fakechain` flag when starting `monerod`).

## Releases and Changelog

See [CHANGELOG.md](CHANGELOG.md) and [RELEASING.md](RELEASING.md).

## Licensing

The code in this project is licensed under the [Apache-2.0](LICENSE)
