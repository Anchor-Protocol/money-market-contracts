# Anchor Money Market Contracts
This monorepository contains the source code for the Money Market smart contracts implementing Anchor Protocol on the [Terra](https://terra.money) blockchain.

You can find information about the architecture, usage, and function of the smart contracts on the official Anchor documentation [site](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market).

### Dependencies

Money Market depends on [Anchor Token Contracts](https://github.com/anchor-protocol/anchor-token-contracts) and [bAsset Contracts](https://github.com/Anchor-Protocol/anchor-bAsset-contracts).

## Contracts

| Contract                                            | Reference                                              | Description                                                                                                                        |
| --------------------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| [`overseer`](./contracts/overseer)  | [doc](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/overseer) | Manages money market overalls, stores borrower information                                              |
| [`market`](../contracts/market) | [doc](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/market) | Handles Terra stablecoin deposits and borrows, ANC distribution to borrowers                                                                                                    |
| [`custody_bLuna`](./contracts/custody_bluna)      | [doc](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/custody-bluna-specific)   | Handles bLuna collateral deposits and withdrawals                                                         |
| [`interest_model`](./contracts/interest_model)              | [doc](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/interest_model)       | Calculates the current borrow interest rate based on the market situation |
| [`distribution_model`](./contracts/distribution_model)        | [doc](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/distribution_model)    | Calculates the borrower ANC emission rate based on the previous emission rate                                                                   |
| [`oracle`](./contracts/oracle)      | [doc](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/oracle)   | Provides a price feed for bAsset collaterals                                                                            |
| [`liquidation`](./contracts/liquidation)      | [doc](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/liquidations/liquidation-contract)   | OTC exchange contract for bAsset collateral liquidations                                                                            |

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

3. Make sure [Docker](https://www.docker.com/) is installed

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
  cosmwasm/workspace-optimizer:0.10.4
```

This performs several optimizations which can significantly reduce the final size of the contract binaries, which will be available inside the `artifacts/` directory.

## License

Copyright 2020 Anchor Protocol

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at http://www.apache.org/licenses/LICENSE-2.0. Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

See the License for the specific language governing permissions and limitations under the License.
