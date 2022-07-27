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

mod common;
use common::main_tests;

#[tokio::test]
async fn main_functional_test() {
    // run those tests functions concurrently since the state one changes does not affect the state
    // the other one interacts with.
    let handle1 = tokio::spawn(main_tests::basic_wallet_test());
    let handle2 = tokio::spawn(async {
        main_tests::empty_blockchain_test().await;
        main_tests::non_empty_blockchain_test().await;
    });
    let handle3 = tokio::spawn(async {
        main_tests::basic_daemon_rpc_test().await;
    });

    let res = tokio::try_join!(handle1, handle2, handle3);
    res.unwrap();

    main_tests::all_clients_interaction_test().await;
}
