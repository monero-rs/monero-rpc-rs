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

#[cfg(feature = "client")]
mod clients_tests;

#[cfg(feature = "client")]
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
