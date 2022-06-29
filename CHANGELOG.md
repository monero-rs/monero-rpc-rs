# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/monero-ecosystem/monero-rpc-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/monero-ecosystem/monero-rpc-rs/compare/6f4db8235439de6d4b758cba92fb1e81ae7f0bf5...v0.1.0
