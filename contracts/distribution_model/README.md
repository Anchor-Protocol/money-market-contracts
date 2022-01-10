# Distribution Model

**NOTE**: Reference documentation for this contract is available [here](https://docs.anchorprotocol.com/smart-contracts/money-market/distribution-model).


The Distribution Model contract manages the calculation of the ANC emission rate,
using fed-in deposit rate information. At the time of protocol genesis, the 
emission rate adjusts to double when the deposit rate is below the targeted rate
and decreases by 10% if the deposit rate is above the targeted rate. Further
descriptions on the ANC emission rate control mechanism can be found [here](https://docs.anchorprotocol.com/protocol/anchor-token-anc#anchor-token-supply).
