# Liquidation Queue

**NOTE**: Reference documentation for this contract is available [here](https://docs.anchorprotocol.com/smart-contracts/liquidations/liquidation-queue-contract).

The Liquidation contract enables users to submit Terra stablecoin bids for a Cw20-compliant token. Bidders can submit a bid to one of the bid pools; each of the pools deposited funds are used to buy the liquidated collateral at different discount rates. There are 31 slots per collateral, from 0% to 30%; users can bid on one or more slots.
Upon execution of a bid, Cw20 tokens are sent to the bidder, while the bidder's Terra stablecoins are sent to the repay address (if not specified, sent to message sender). A portion of the collateral value liquidated will be given to the address triggering the liquidation (liquidator_fee).

Additionally, the Liquidation contract serves as the point of calculation for partial collateral liquidations, where a loan position is liquidated until it reaches a safe borrow_amount / borrow_limit ratio. The required liquidation amount for each collateral is calculated based on the fed-in loan position's attributes and the state of the bid pools.
The oracle contract is responsible for providing the relevant Cw20 token prices. Price data from the Oracle contract are only valid for 60 seconds (price_timeframe). The Liquidation contract disables bid executions until new price data is fed in to the Oracle contract.
