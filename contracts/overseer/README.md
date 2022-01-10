# Overseer

**NOTE**: Reference documentation for this contract is available [here](https://docs.anchorprotocol.com/smart-contracts/money-market/overseer).

The Overseer contract is responsible for storing key protocol parameters
and the whitelisting of new bAsset collaterals. The Overseer keeps track of locked collateral amounts and calculates the borrow limits for each user.

This contract is the recipient for collected bAsset rewards claimed by
Custody contracts. The Overseer calculates the amount of depositor
subsidies to be distributed, and the resulting amount is sent to
the Market contract.

The Overseer halts borrow-related operations if the Oracle's price data is
older than 60 seconds `price_timeframe`. Operations are resumed when new
price data is fed-in.
