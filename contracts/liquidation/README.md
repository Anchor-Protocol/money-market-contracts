# Liquidation

**NOTE**: Reference documentation for this contract is available [here](https://docs.anchorprotocol.com/smart-contracts/liquidations).

The Liquidation Contract enable users to submit Terra stablecoin bids for
a Cw20-compliant token. Bidders can specify the rate of premium they will
receive on bid execution. The maximum premium rate is set at 20%.

Upon execution of a bid, the Cw20 tokens are sent to the bidder. The
bidder's Terra stablecoins are sent to the repay address. If a repay address is not specified,
the Terra stablecoins are sent sent to the message sender. The Oracle contract is responsible for providing
the relevant Cw20 token prices.

Additionally, the Liquidation contract serves as the point of calculation
for partial collateral liquidations where a loan position is liquidated
until it reaches a safe `borrow_amount / borrow_limit` ratio. The required
liquidation amount for each collateral is calculated based on the fed-in
loan position's attributes.

Price data from the Oracle contract are only valid for 60 seconds
`price_timeframe`. The Liquidation contract disables bid executions until
new price data is fed into the Oracle contract.
