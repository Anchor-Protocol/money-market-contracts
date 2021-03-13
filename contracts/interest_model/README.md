# Interest Model

**NOTE**: Reference documentation for this contract is available [here](https://app.gitbook.com/@anchor-protocol/s/anchor-2/smart-contracts/money-market/interest-model).

The Interest Model contract is responsible for calculating the current borrow 
interest rate for stablecoin loans, based on the fed in market details. The 
interest rate is initially set to increase proportionally with market utilization, 
or the stablecoin borrow demand of the Anchor Money Market.
