# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added `create_account` method by @mchtilianov ([#120](https://github.com/monero-rs/monero-rpc-rs/pull/120)).
- Added `get_tx_proof` and `check_tx_proof` methods by @mchtilianov ([#122](https://github.com/monero-rs/monero-rpc-rs/pull/122)).
- Made request timeout configurable by @silverpill ([#131](https://github.com/monero-rs/monero-rpc-rs/pull/131)).
- Added `get_block` method by @essecara ([#123](https://github.com/monero-rs/monero-rpc-rs/pull/123)).
- Added `rustls` support by @silverpill ([#133](https://github.com/monero-rs/monero-rpc-rs/pull/133)).

### Changed

- Bumped MSRV to `1.66.0`
- Updated `reqwest` to version `0.12` by @silverpill ([#140](https://github.com/monero-rs/monero-rpc-rs/pull/140)).

### Fixed

- Fixed `get_transfers` with `Out=true` in `category_selector` by @bytenotbark ([#127](https://github.com/monero-rs/monero-rpc-rs/pull/127)).

## [0.4.0] - 2023-09-15

### Added

- Add `sign` and `verify` methods by @silverpill ([#105](https://github.com/monero-rs/monero-rpc-rs/pull/105))
- Add `get_attribute` and `set_attribute` methods by @refactor-ring ([#108](https://github.com/monero-rs/monero-rpc-rs/pull/108))
- Add `restore_deterministic_wallet` method by @cilki ([#139](https://github.com/monero-rs/monero-rpc-rs/pull/139))

### Changed

- Bump MSRV to `1.63.0`
- Run test suite against Monero node and wallet `0.18.1.2` and `0.18.2.2`

## [0.3.2] - 2022-12-13

### Removed

- Disable chrono default features by @silverpill ([#87](https://github.com/monero-rs/monero-rpc-rs/pull/87))

## [0.3.1] - 2022-12-12

### Changed

- Rollback Monero library bump from `0.18` to `0.17`

## [0.3.0] - 2022-12-12

### Added

- Add a Builder pattern for the RpcClient to include a proxy and authentication argument. The proxy allows the usage of e.g. a Tor socks proxy. This argument has to be a string pointing to the the address of the proxy and its protocol prefix, .e.g. "socks5://127.0.0.1:9050" for using a socks5 proxy ([#92](https://github.com/monero-rs/monero-rpc-rs/pull/92))
- Implement `Eq` on more structs ([#78](https://github.com/monero-rs/monero-rpc-rs/pull/78))
- Running tests against the latest Monero RPC versions ([#75](https://github.com/monero-rs/monero-rpc-rs/pull/75), [#90](https://github.com/monero-rs/monero-rpc-rs/pull/90))

### Changed

- Monero library bumped to version `0.18` ([#83](https://github.com/monero-rs/monero-rpc-rs/pull/83))
- Update fixed-hash requirement from 0.7 to 0.8 ([#85](https://github.com/monero-rs/monero-rpc-rs/pull/85))

## [0.2.0] - 2022-07-29

### Added

- Add tests for types implementing `HashType` in `utils.rs` ([#59](https://github.com/monero-rs/monero-rpc-rs/pull/59))
- Add tests for `HashString`'s implementation of the traits `Debug`, `Serialize`, and `Deserialize`, in `utils.rs` ([#59](https://github.com/monero-rs/monero-rpc-rs/pull/59))
- Add tests for `models.rs` ([#63](https://github.com/monero-rs/monero-rpc-rs/pull/63))
- Add `PartialEq` trait for the following types in `src/models.rs` ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/)):
  - BlockTemplate
  - Transaction
  - SubaddressBalanceData
  - BalanceData
  - TransferPriority
  - SubaddressData
  - SubaddressIndex
  - Payment
  - AddressData
  - IncomingTransfers
  - GotAccount
  - GetAccountsData
  - GotTransfer
  - SignedKeyImage
  - KeyImageImportResponse
- Add `GenerateBlocksResponse` struct ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))
- Add `all` paremeter, of type `Option<bool>` to the `export_key_images` method, and pass it to the RPC ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))
- Add an error for `on_get_block_hash` on invalid height, instead of returning success with an incorrect hash ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))

### Removed

- Remove `SubAddressIndex` from `src/models.rs` ([#55](https://github.com/monero-rs/monero-rpc-rs/pull/55))
- Remove `SubaddressIndex` from `src/models.rs` ([#62](https://github.com/monero-rs/monero-rpc-rs/pull/62))

### Changed

- Change any use of `SubAddressIndex` to `SubaddressIndex` ([#55](https://github.com/monero-rs/monero-rpc-rs/pull/55))
- Change `HashType`'s `from_str` implementation for `Vec<u8>` in order to accept inputs starting with `0x` ([#61](https://github.com/monero-rs/monero-rpc-rs/pull/61))
- Change `HashType`'s `bytes` implementation by adding `AsRef<[u8]>` as a trait bound and returning the `as_ref` implementation ([#61](https://github.com/monero-rs/monero-rpc-rs/pull/61))
- Change any use of `SubaddressIndex` to subaddress::Index\` ([#62](https://github.com/monero-rs/monero-rpc-rs/pull/62))
- Change types of `address` and `account` indices to use `u32` instead of u64 ([#62](https://github.com/monero-rs/monero-rpc-rs/pull/62))
- Change `label_address` to receive an argument named `index` of type `subaddress::Index` instead of receiving the arguments `account_index` and `address_index`, both of type `u64` ([#62](https://github.com/monero-rs/monero-rpc-rs/pull/62))
- Change `get_address_index` to return `anyhow::Result<subaddress::Index>` instead of `anyhow::Result<(u64, u64)>` ([#62](https://github.com/monero-rs/monero-rpc-rs/pull/62))
- Use `Amount` type from `monero-rs` where possible ([#68](https://github.com/monero-rs/monero-rpc-rs/pull/68))
- Rename `DaemonClient` to `DaemonJsonRpcClient`, and `RegtestDaemonClient` to `RegtestDaemonJsonRpcClient` ([70](https://github.com/monero-rs/monero-rpc-rs/pull/70))
- Change `TransferData`'s `tx_key` field from `HashString<CryptoNoteHash>` to `HashString<Vec<u8>>` ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))
- `get_balance` method now passes the correct parameter name to the RPC ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))
- Change `generate_blocks` to return `anyhow::Result<GenerateBlocksResponse>` instead of `anyhow::Result<u64>` ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))
- `submit_block` method now works correctly and had its return type changed to `anyhow::Result<()>` ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))
- Change `get_payments` to actually return a vector of `Payments` ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))
- Change `check_tx_key`'s `tx_key` parameter to type `Vec<u8>` ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))
- Change `check_tx_key`'s return type from `anyhow::Result<(NonZeroU64, bool, NonZeroU64)>` to `anyhow::Result<(u64, bool, Amount)>`, since the first element can be `0`, and the last element depicts an amount ([#65](https://github.com/monero-rs/monero-rpc-rs/pull/65/))

## [0.1.0] - 2022-06-29

### Added

- Create RPC clients (`RpcClient`) for: `WalletClient`, `DaemonClient`, `RegtestDaemonClient`, and `DaemonRpcClient`
- Wallet methods:
  - `check_tx_key`
  - `close_wallet`
  - `create_address`
  - `create_wallet`
  - `export_key_images`
  - `generate_from_keys`
  - `get_accounts`
  - `get_address`
  - `get_address_index`
  - `get_balance`
  - `get_bulk_payments`
  - `get_height`
  - `get_payments`
  - `get_transfer`
  - `get_transfers`
  - `get_version`
  - `import_key_images`
  - `incoming_transfers`
  - `label_address`
  - `open_wallet`
  - `query_key`
  - `refresh`
  - `relay_tx`
  - `sign_transfer`
  - `submit_transfer`
  - `sweep_all`
  - `transfer`
- Daemon methods:
  - `get_block_count`
  - `get_block_header`
  - `get_block_headers_range`
  - `get_block_template`
  - `on_get_block_hash`
  - `regtest`
  - `submit_block`
- Regtest daemon methods:
  - `generate_blocks`
- Daemon RPC methods:
  - `get_transactions`

[Unreleased]: https://github.com/monero-rs/monero-rpc-rs/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/monero-rs/monero-rpc-rs/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/monero-rs/monero-rpc-rs/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/monero-rs/monero-rpc-rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/monero-rs/monero-rpc-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/monero-rs/monero-rpc-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/monero-rs/monero-rpc-rs/compare/363c433023318877e9d397dbe2b50bdf88cdee9d...v0.1.0
