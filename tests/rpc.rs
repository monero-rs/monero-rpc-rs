// Copyright 2019-2022 Artem Vorotnikov and Monero Rust Contributors
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

use monero::Hash;
use monero_rpc::{RpcAuthentication, RpcClient};
use std::env;

mod clients_tests;

#[tokio::test]
async fn main_functional_test() {
    /* FAQ:
     *
     * - Q: What is the purpose of each test function?
     *   A: See the comments in each of ther files in `tests/clients_tests.rs`.
     *
     *  - Q: Why are the functions called in the order below?
     *    A: `basic_wallet` only calls functions from the `WalletClient` that do not modify
     *    the blockchain. For example: it creates wallets, but it does not create transfers.
     *
     *    At the same time, `empty_blockchain` needs the blockchain to be empty, and so it can
     *    run at the same time `basic_wallet` runs. Moreover, `basic_daemon_rpc` has only
     *    one function: `get_transactions`; and since neither `basic_wallet` nor `empty_blockchain`
     *    nor `non_empty_blockchain` create transactions, it can also run at the same time then
     *    other functions are running because its result will always be the same.
     *
     *    Now, inside `handle2`, `empty_blockchain` runs before `non_empty_blockchain`
     *    because, as said, `empty_blockchain` needs a fresh blockchain, but `non_empty_blockchain`
     *    modifies the blockchain by adding blocks to it.
     *
     *    Finally, `all_clients_interaction` runs at the end because it modifies the blockchain
     *    in ways the other tests do not (for example, it creates transacions), and it also creates
     *    blocks, so the other tests would not work if running at the same time
     *    `all_clients_interaction` runs. Also, it makes sense `all_clients_interaction` to
     *    run last because the other tests test each client individually, but `all_clients_interaction`
     *    calls functions from all clients.
     *
     */

    let handle1 = tokio::spawn(clients_tests::basic_wallet::run());
    let handle2 = tokio::spawn(async {
        clients_tests::empty_blockchain::run().await;
        clients_tests::non_empty_blockchain::run().await;
    });
    let handle3 = tokio::spawn(async {
        clients_tests::basic_daemon_rpc::run().await;
    });

    let res = tokio::try_join!(handle1, handle2, handle3);
    res.unwrap();

    clients_tests::all_clients_interaction::run().await;
}

#[cfg(feature = "rpc_authentication")]
fn setup_rpc_auth_client(username: &str, password: &str, port: u32) -> RpcClient {
    let whost = env::var("MONERO_WALLET_HOST_1").unwrap_or_else(|_| "localhost".into());
    let rpc_credentials = RpcAuthentication::Credentials {
        username: username.into(),
        password: password.into(),
    };
    let rpc_client =
        RpcClient::with_authentication(format!("http://{}:{}", whost, port), rpc_credentials);

    rpc_client
}

#[tokio::test]
#[cfg(feature = "rpc_authentication")]
async fn test_daemon_rpc_auth() {
    let rpc_client = setup_rpc_auth_client("foo", "bar", 18085).daemon();
    let daemon_transactions = rpc_client.get_block_count().await;

    assert!(daemon_transactions.is_ok());
}

#[tokio::test]
#[cfg(feature = "rpc_authentication")]
async fn test_daemon_rpc_auth_fail() {
    let rpc_client = setup_rpc_auth_client("invalid", "bar", 18085).daemon();
    let daemon_transactions = rpc_client.get_block_count().await;

    assert!(daemon_transactions.is_err());
}

#[tokio::test]
#[cfg(feature = "rpc_authentication")]
async fn test_daemon_rpc_rpc_auth() {
    let rpc_client = setup_rpc_auth_client("foo", "bar", 18085).daemon_rpc();
    let transactions = vec![Hash::from_low_u64_be(1)];
    let daemon_transactions = rpc_client
        .get_transactions(transactions, Some(true), Some(true))
        .await;

    assert!(daemon_transactions.is_ok());
}

#[tokio::test]
#[cfg(feature = "rpc_authentication")]
async fn test_rpc_auth() {
    let rpc_client = setup_rpc_auth_client("foo", "bar", 18084).wallet();
    assert!(rpc_client.get_version().await.is_ok());

    let version = rpc_client.get_version().await.unwrap();
    assert!(version.0 > 0);
}

#[tokio::test]
#[cfg(feature = "rpc_authentication")]
async fn test_rpc_auth_fail() {
    let rpc_client = setup_rpc_auth_client("invalid", "auth", 18084).wallet();

    assert!(rpc_client.get_version().await.is_err());
}
