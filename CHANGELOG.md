# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Contract PR guidelines section in `CONTRIBUTING.md` covering `no_std` safety, error enum conventions, storage bounds, TTL management, unit tests, and a pre-review checklist (#489, #490, #503, #504)
- `SECURITY.md` with vulnerability disclosure policy, scope, safe-harbor guidelines, and response timeline (#490)
- Complete contract index in `contracts/README.md` covering all 10 contract crates (#503)
- `docs/bindings-error-mapping.md` documenting how Rust error enums map to TS unions and HTTP status codes (#504)
- All 10 contract error enums documented in `docs/error_codes.md` (previously only 5 were listed; added mux-delegation, mux-policy, mux-recovery, mux-spending-policy, mux-wallet-registry) (#504)

### Changed
- Upgraded migration notes for `mux-account` in `docs/account-upgrade-migration.md` and inline module docs
- `RegistryMeta` struct (`name`, `version`, `description`) and `DataKey::Metadata` storage key for `mux-account`
- `set_metadata()` and `get_metadata()` contract functions on `mux-account` (owner-only write, public read)
- Negative-path unit tests for `mux-account-factory`: exact error assertions for `InvalidAccount` and `TooManyAccounts`, `MetadataNotFound` after deploy without metadata, wrong-owner metadata lookup, and unauthorized deploy without auth
- `WalletMetadata` struct (`label`, `description`) for `mux-wallet-registry` contract (#318)
- `register_wallet_with_metadata()` and `get_metadata()` contract functions in `mux-wallet-registry` (#318)
- `registerWalletWithMetadata()` and `getMetadata()` methods on `MuxWalletRegistryClient` TS binding (#318)
- `WalletMetadata` and `MuxWalletRegistryError` TypeScript types exported from the binding (#318, #319)
- `WalletNotFound` mapped to HTTP 404 in `ERROR_HTTP_MAP`; `MuxWalletRegistryError` added to the `ContractError` union (#319)
- Wallet registry error codes documented in `docs/error_codes.md` (#319)
- Integration test stub for `mux-wallet-registry` in `bindings/__tests__/wallet-registry.test.ts` (#320)
- All five `MuxBatcherError` variants (`EmptyBatch`, `BatchTooLarge`, `RequiredOperationFailed`, `Unauthorized`, `ReentrancyDetected`) documented with numeric codes and HTTP mappings in `docs/error_codes.md` (#244)
- Integration test stubs for batcher error cases (`BatchTooLarge`, `RequiredOperationFailed`, `Unauthorized`) added to `bindings/__tests__/batch-integration.test.ts` (#245)

### Changed
- `mux-account-factory` deploy / simulate paths share `load_accounts_under_cap` so the per-owner Accounts vec stays bounded at 64
- Documented factory Accounts cap in `docs/storage-griefing.md` and `docs/abi_reference.md`
