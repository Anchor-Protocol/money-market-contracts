[![codecov](https://codecov.io/gh/Anchor-Protocol/money-market-contracts/branch/main/graph/badge.svg?token=B4B2YUSXEU)](https://codecov.io/gh/Anchor-Protocol/money-market-contracts)

# Anchor Money Market Contracts
A Rust and [CosmWasm](https://cosmwasm.com/) implementation of the Anchor Protocol money market on the [Terra blockchain](https://terra.money).

You can find information about the architecture, usage, and function of the smart contracts in the [documentation](https://docs.anchorprotocol.com/).

### Dependencies

Money Market has dependencies on [Anchor Token Contracts](https://github.com/anchor-protocol/anchor-token-contracts) and [bAsset Contracts](https://github.com/Anchor-Protocol/anchor-bAsset-contracts).

## Contracts

| Contract                                               | Reference                                                                                  | Description                                                                   |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| [`overseer`](./contracts/overseer)                     | [doc](https://docs.anchorprotocol.com/smart-contracts/money-market/overseer)               | Manages money market overalls, stores borrower information                    |
| [`market`](../contracts/market)                        | [doc](https://docs.anchorprotocol.com/smart-contracts/money-market/market)                 | Handles Terra stablecoin deposits and borrows, ANC distribution to borrowers  |
| [`custody_bluna`](./contracts/custody_bluna)           | [doc](https://docs.anchorprotocol.com/smart-contracts/money-market/custody-bluna-specific) | Handles bLuna collateral deposits and withdrawals                             |
| [`custody_beth`](./contracts/custody_beth)             | [doc](https://docs.anchorprotocol.com/smart-contracts/money-market/custody-beth)           | Handles bEth collateral deposits and withdrawals                              |
| [`interest_model`](./contracts/interest_model)         | [doc](https://docs.anchorprotocol.com/smart-contracts/money-market/interest-model)         | Calculates the current borrow interest rate based on the market situation     |
| [`distribution_model`](./contracts/distribution_model) | [doc](https://docs.anchorprotocol.com/smart-contracts/money-market/distribution-model)     | Calculates the borrower ANC emission rate based on the previous emission rate |
| [`oracle`](./contracts/oracle)                         | [doc](https://docs.anchorprotocol.com/smart-contracts/money-market/oracle)                 | Provides a price feed for bAsset collaterals                                  |
| [`liquidation`](./contracts/liquidation)               | [doc](https://docs.anchorprotocol.com/smart-contracts/liquidations)                        | OTC exchange contract for bAsset collateral liquidations                      |

## Development

### Environment Setup

- Rust v1.44.1+
- `wasm32-unknown-unknown` target
- Docker

1. Install `rustup` via https://rustup.rs/

2. Run the following:

```sh
rustup default stable
rustup target add wasm32-unknown-unknown
```

3. Make sure [Docker](https://www.docker.com/) is installed.

### Unit / Integration Tests

Each contract contains Rust unit and integration tests embedded within the contract source directories. You can run:

```sh
cargo unit-test
cargo integration-test
```

### Compiling

After making sure tests pass, you can compile each contract with the following:

```sh
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw1_subkeys.wasm .
ls -l cw1_subkeys.wasm
sha256sum cw1_subkeys.wasm
```

#### Production

For production builds, run the following:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.11.5
```

This performs several optimizations which can significantly reduce the final size of the contract binaries, which will be available inside the `artifacts/` directory.

## License

This repository is licensed under the Apache 2.0 license. See [LICENSE](./LICENSE) for full disclosure.

© 2021 Anchor Protocol.
