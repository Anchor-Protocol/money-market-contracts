# Overseer

**NOTE**: Reference documentation for this contract is available [here](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/overseer).

The Overseer contract is responsible for storing key protocol parameters 
and the whitelisting of new bAsset collaterals. The borrow limit of users 
are calculated here, as the Overseer keeps track of locked collateral 
amounts for all users.

This contract is the recipient for collected bAsset rewards claimed by 
Custody contracts. The Overseer calculates the amount of depositor 
subsidies that has to be distributed, and the resulting amount is sent to 
the Market contract.

The Overseer halts borrow-related operations if the Oracle's price data is 
older than 60 seconds `price_timeframe`. Operations are resumed when new 
price data is fed-in.
