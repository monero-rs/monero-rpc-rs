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

use std::{env, path::PathBuf};

use monero_rpc::{RpcClient, TlsClientConfig, TlsConfig, TlsServerConfig};

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

#[tokio::test]
async fn tls_config_test() {
    let dhost = env::var("MONERO_DAEMON_HOST").unwrap_or_else(|_| "localhost".into());

    // this monerod daemon uses a SSL certificate created with `monero-gen-ssl-cert`
    let monerod_1_tls_url = format!("https://{}:18090", dhost);

    // with all `TlsConfig` fields None, but servers returns a non-trusted certificate
    let monero_1_tls_config_all_none_invalid_cert = TlsConfig {
        server: None,
        client: None,
    };
    let rpc_client = RpcClient::new(
        monerod_1_tls_url.clone(),
        Some(monero_1_tls_config_all_none_invalid_cert),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_err());

    // with `TlsConfig` field 'server`, but has incorrect `root_certificates` and
    // we try to skip hostname verification
    let monero_1_tls_config_server_incorrect_cert = TlsConfig {
        server: Some(TlsServerConfig {
            // the certificate below has nothing to do with `monerod-1-tls`
            root_certificates_path: PathBuf::from(
                "./tests/certificates/monero_openssl_no_cn_server.crt",
            ),
            danger_skip_hostname_verification: true,
        }),
        client: None,
    };
    let rpc_client = RpcClient::new(
        monerod_1_tls_url.clone(),
        Some(monero_1_tls_config_server_incorrect_cert),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_err());

    // with `TlsConfig` field 'server`, but certificate has no
    // hostname and we do not skip hostname verification
    let monero_1_tls_config_server_valid_cert_with_hostname_verification = TlsConfig {
        server: Some(TlsServerConfig {
            root_certificates_path: PathBuf::from("./tests/certificates/monero_gen_server.crt"),
            danger_skip_hostname_verification: false,
        }),
        client: None,
    };
    let rpc_client = RpcClient::new(
        monerod_1_tls_url.clone(),
        Some(monero_1_tls_config_server_valid_cert_with_hostname_verification),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_err());

    // with `TlsConfig` field 'server`, certificate is valid and we skip hostname verification
    let monero_1_tls_config_server_valid_cert_without_hostname_verification = TlsConfig {
        server: Some(TlsServerConfig {
            root_certificates_path: PathBuf::from("./tests/certificates/monero_gen_server.crt"),
            danger_skip_hostname_verification: true,
        }),
        client: None,
    };
    let rpc_client = RpcClient::new(
        monerod_1_tls_url.clone(),
        Some(monero_1_tls_config_server_valid_cert_without_hostname_verification),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_ok());

    // this monerod daemon uses a SSL certificate created by calling `openssl` from
    // the command line; note that the certificate **has no** CN
    let monerod_2_tls_url = format!("https://{}:18091", dhost);

    // with `TlsConfig` field 'server`, but certificate has no
    // hostname and we do not skip hostname verification
    let monero_2_tls_config_server_valid_cert_with_hostname_verification = TlsConfig {
        server: Some(TlsServerConfig {
            root_certificates_path: PathBuf::from(
                "./tests/certificates/monero_openssl_no_cn_server.crt",
            ),
            danger_skip_hostname_verification: false,
        }),
        client: None,
    };
    let rpc_client = RpcClient::new(
        monerod_2_tls_url.clone(),
        Some(monero_2_tls_config_server_valid_cert_with_hostname_verification),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_err());

    // with `TlsConfig` field 'server`, certificate is valid and we skip hostname verification
    let monero_2_tls_config_server_valid_cert_without_hostname_verification = TlsConfig {
        server: Some(TlsServerConfig {
            root_certificates_path: PathBuf::from(
                "./tests/certificates/monero_openssl_no_cn_server.crt",
            ),
            danger_skip_hostname_verification: true,
        }),
        client: None,
    };
    let rpc_client = RpcClient::new(
        monerod_2_tls_url.clone(),
        Some(monero_2_tls_config_server_valid_cert_without_hostname_verification),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_ok());

    // this monerod daemon uses a SSL certificate created by calling `openssl` from
    // the command line; note that the certificate **does have** a CN
    let monerod_3_tls_url = format!("https://{}:18092", dhost);

    // with `TlsConfig` field 'server`, but has incorrect `root_certificates` and
    // we try to skip hostname verification
    let monero_3_tls_config_server_incorrect_cert = TlsConfig {
        server: Some(TlsServerConfig {
            // the certificate below has nothing to do with `monerod-3-tls`
            root_certificates_path: PathBuf::from("./tests/certificates/monero_gen_server.crt"),
            danger_skip_hostname_verification: true,
        }),
        client: None,
    };
    let rpc_client = RpcClient::new(
        monerod_3_tls_url.clone(),
        Some(monero_3_tls_config_server_incorrect_cert),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_err());

    // with `TlsConfig` field 'server`, certificate is valid and we **do not** skip hostname verification
    let monero_3_tls_config_server_valid_cert_without_hostname_verification = TlsConfig {
        server: Some(TlsServerConfig {
            root_certificates_path: PathBuf::from(
                "./tests/certificates/monero_openssl_with_cn_server.crt",
            ),
            danger_skip_hostname_verification: false,
        }),
        client: None,
    };
    let rpc_client = RpcClient::new(
        monerod_3_tls_url.clone(),
        Some(monero_3_tls_config_server_valid_cert_without_hostname_verification),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_ok());

    // this monerod daemon uses the SSL certificate that is generated by `monero-gen-ssl-cert`;
    // but what we are interested in this case is that this monerod daemon only accepts
    // clients that possess allowed certificates/identities
    let monerod_4_tls_url = format!("https://{}:18093", dhost);
    let monerod_4_server_config = Some(TlsServerConfig {
        root_certificates_path: PathBuf::from("./tests/certificates/monero_gen_server.crt"),
        danger_skip_hostname_verification: true,
    });

    // with `TlsConfig` field `client`, with correct `client.identity_path`, and
    // correct `client.password`
    let monerod_4_client_config_correct_identity_and_password = TlsClientConfig {
        identity_path: PathBuf::from("./tests/certificates/monero_client.pfx"),
        password: None,
    };
    let rpc_client = RpcClient::new(
        monerod_4_tls_url.clone(),
        Some(TlsConfig {
            server: monerod_4_server_config.clone(),
            client: Some(monerod_4_client_config_correct_identity_and_password),
        }),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_ok());

    // with `TlsConfig` field `client`, with correct `client.identity_path`, and
    // incorrect `client.password`
    let monerod_4_client_config_correct_identity_and_incorrect_password = TlsClientConfig {
        identity_path: PathBuf::from("./tests/certificates/monero_client.pfx"),
        password: Some("wrong :)".to_string()),
    };
    let rpc_client_should_err = std::panic::catch_unwind(|| {
        RpcClient::new(
            monerod_4_tls_url.clone(),
            Some(TlsConfig {
                server: monerod_4_server_config.clone(),
                client: Some(monerod_4_client_config_correct_identity_and_incorrect_password),
            }),
        )
        .daemon()
        .regtest();
    });
    assert!(rpc_client_should_err.is_err());

    // with `TlsConfig` field `client`, with uncorrelated `client.identity_path`
    // with correct `client.password` of the `client.identity_path` passed
    let monerod_4_client_config_incorrect_identity_and_correct_password = TlsClientConfig {
        identity_path: PathBuf::from("./tests/certificates/monero_gen_server.pfx"),
        password: None,
    };
    let rpc_client = RpcClient::new(
        monerod_4_tls_url.clone(),
        Some(TlsConfig {
            server: monerod_4_server_config.clone(),
            client: Some(monerod_4_client_config_incorrect_identity_and_correct_password),
        }),
    )
    .daemon()
    .regtest();
    assert!(rpc_client.get_block_count().await.is_err());
}
