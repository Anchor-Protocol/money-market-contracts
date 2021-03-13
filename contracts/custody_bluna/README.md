# Custody bLuna

**NOTE**: Reference documentation for this contract is available [here](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/custody-bluna-specific).

The Custody contract is where supplied bAsset collaterals are managed. Users can make collateral 
deposits and withdrawals to and from this contract. The Custody contract is also responsible for 
claiming bAsset rewards and converting them to Terra stable coins, which is then sent to the [Overseer contract](../overseer) for eventual distribution.
