use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, CanonicalAddr, Coin, Decimal, Extern, HumanAddr,
    Querier, QuerierResult, QueryRequest, SystemError, Uint128, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use std::collections::HashMap;

use cw20::TokenInfoResponse;
use moneymarket::distribution_model::AncEmissionRateResponse;
use moneymarket::interest_model::BorrowRateResponse;
use moneymarket::overseer::BorrowLimitResponse;
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query borrow rate to interest model contract
    BorrowRate {
        market_balance: Uint128,
        total_liabilities: Decimal256,
        total_reserves: Decimal256,
    },
    /// Query borrow limit to overseer contract
    BorrowLimit {
        borrower: HumanAddr,
        block_time: Option<u64>,
    },
    /// Query ANC emission rate to distribution model contract
    AncEmissionRate {
        target_deposit_rate: Decimal256,
        deposit_rate: Decimal256,
        current_emission_rate: Decimal256,
    },
}

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    canonical_length: usize,
    contract_balance: &[Coin],
) -> Extern<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = HumanAddr::from(MOCK_CONTRACT_ADDR);
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
        MockQuerier::new(&[(&contract_addr, contract_balance)]),
        canonical_length,
        MockApi::new(canonical_length),
    );

    Extern {
        storage: MockStorage::default(),
        api: MockApi::new(canonical_length),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    token_querier: TokenQuerier,
    tax_querier: TaxQuerier,
    borrow_rate_querier: BorrowRateQuerier,
    borrow_limit_querier: BorrowLimitQuerier,
    canonical_length: usize,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<HumanAddr, HashMap<HumanAddr, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])],
) -> HashMap<HumanAddr, HashMap<HumanAddr, Uint128>> {
    let mut balances_map: HashMap<HumanAddr, HashMap<HumanAddr, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<HumanAddr, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(HumanAddr::from(addr), **balance);
        }

        balances_map.insert(HumanAddr::from(contract_addr), contract_balances_map);
    }
    balances_map
}

#[derive(Clone, Default)]
pub struct TaxQuerier {
    rate: Decimal,
    // this lets us iterate over all pairs that match the first string
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
}

#[derive(Clone, Default)]
pub struct BorrowRateQuerier {
    // this lets us iterate over all pairs that match the first string
    borrower_rate: HashMap<HumanAddr, Decimal256>,
}

impl BorrowRateQuerier {
    pub fn new(borrower_rate: &[(&HumanAddr, &Decimal256)]) -> Self {
        BorrowRateQuerier {
            borrower_rate: borrower_rate_to_map(borrower_rate),
        }
    }
}

pub(crate) fn borrower_rate_to_map(
    borrower_rate: &[(&HumanAddr, &Decimal256)],
) -> HashMap<HumanAddr, Decimal256> {
    let mut borrower_rate_map: HashMap<HumanAddr, Decimal256> = HashMap::new();
    for (market_contract, borrower_rate) in borrower_rate.iter() {
        borrower_rate_map.insert((*market_contract).clone(), **borrower_rate);
    }
    borrower_rate_map
}

#[derive(Clone, Default)]
pub struct BorrowLimitQuerier {
    // this lets us iterate over all pairs that match the first string
    borrow_limit: HashMap<HumanAddr, Uint256>,
}

impl BorrowLimitQuerier {
    pub fn new(borrow_limit: &[(&HumanAddr, &Uint256)]) -> Self {
        BorrowLimitQuerier {
            borrow_limit: borrow_limit_to_map(borrow_limit),
        }
    }
}

pub(crate) fn borrow_limit_to_map(
    borrow_limit: &[(&HumanAddr, &Uint256)],
) -> HashMap<HumanAddr, Uint256> {
    let mut borrow_limit_map: HashMap<HumanAddr, Uint256> = HashMap::new();
    for (market_contract, borrow_limit) in borrow_limit.iter() {
        borrow_limit_map.insert((*market_contract).clone(), **borrow_limit);
    }
    borrow_limit_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                if &TerraRoute::Treasury == route {
                    match query_data {
                        TerraQuery::TaxRate {} => {
                            let res = TaxRateResponse {
                                rate: self.tax_querier.rate,
                            };
                            Ok(to_binary(&res))
                        }
                        TerraQuery::TaxCap { denom } => {
                            let cap = self
                                .tax_querier
                                .caps
                                .get(denom)
                                .copied()
                                .unwrap_or_default();
                            let res = TaxCapResponse { cap };
                            Ok(to_binary(&res))
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(&msg).unwrap() {
                    QueryMsg::BorrowRate {
                        market_balance: _,
                        total_liabilities: _,
                        total_reserves: _,
                    } => match self.borrow_rate_querier.borrower_rate.get(&contract_addr) {
                        Some(v) => Ok(to_binary(&BorrowRateResponse { rate: *v })),
                        None => Err(SystemError::InvalidRequest {
                            error: "No borrow rate exists".to_string(),
                            request: msg.as_slice().into(),
                        }),
                    },
                    QueryMsg::BorrowLimit {
                        borrower,
                        block_time: _,
                    } => match self.borrow_limit_querier.borrow_limit.get(&borrower) {
                        Some(v) => Ok(to_binary(&BorrowLimitResponse {
                            borrower,
                            borrow_limit: *v,
                        })),
                        None => Err(SystemError::InvalidRequest {
                            error: "No borrow limit exists".to_string(),
                            request: msg.as_slice().into(),
                        }),
                    },
                    QueryMsg::AncEmissionRate {
                        target_deposit_rate: _,
                        deposit_rate: _,
                        current_emission_rate: _,
                    } => Ok(to_binary(&AncEmissionRateResponse {
                        emission_rate: Decimal256::from_uint256(5u64),
                    })),
                }
            }
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                let key: &[u8] = key.as_slice();

                let prefix_token_info = to_length_prefixed(b"token_info").to_vec();
                let prefix_balance = to_length_prefixed(b"balance").to_vec();

                let balances: &HashMap<HumanAddr, Uint128> =
                    match self.token_querier.balances.get(contract_addr) {
                        Some(balances) => balances,
                        None => {
                            return Err(SystemError::InvalidRequest {
                                error: format!(
                                    "No balance info exists for the contract {}",
                                    contract_addr
                                ),
                                request: key.into(),
                            })
                        }
                    };

                if key.to_vec() == prefix_token_info {
                    let mut total_supply = Uint128::zero();

                    for balance in balances {
                        total_supply += *balance.1;
                    }

                    Ok(to_binary(
                        &to_binary(&TokenInfoResponse {
                            name: "mAPPL".to_string(),
                            symbol: "mAPPL".to_string(),
                            decimals: 6,
                            total_supply: total_supply,
                        })
                        .unwrap(),
                    ))
                } else if key[..prefix_balance.len()].to_vec() == prefix_balance {
                    let key_address: &[u8] = &key[prefix_balance.len()..];
                    let address_raw: CanonicalAddr = CanonicalAddr::from(key_address);
                    let api: MockApi = MockApi::new(self.canonical_length);
                    let address: HumanAddr = match api.human_address(&address_raw) {
                        Ok(v) => v,
                        Err(e) => {
                            return Err(SystemError::InvalidRequest {
                                error: format!("Parsing query request: {}", e),
                                request: key.into(),
                            })
                        }
                    };
                    let balance = match balances.get(&address) {
                        Some(v) => v,
                        None => {
                            return Err(SystemError::InvalidRequest {
                                error: "Balance not found".to_string(),
                                request: key.into(),
                            })
                        }
                    };
                    Ok(to_binary(&to_binary(&balance).unwrap()))
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new<A: Api>(
        base: MockQuerier<TerraQueryWrapper>,
        canonical_length: usize,
        _api: A,
    ) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            tax_querier: TaxQuerier::default(),
            borrow_rate_querier: BorrowRateQuerier::default(),
            borrow_limit_querier: BorrowLimitQuerier::default(),
            canonical_length,
        }
    }

    // set a new balance for the given address and return the old balance
    pub fn update_balance<U: Into<HumanAddr>>(
        &mut self,
        addr: U,
        balance: Vec<Coin>,
    ) -> Option<Vec<Coin>> {
        self.base.update_balance(addr, balance)
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    // configure the tax mock querier
    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    pub fn with_borrow_rate(&mut self, borrow_rate: &[(&HumanAddr, &Decimal256)]) {
        self.borrow_rate_querier = BorrowRateQuerier::new(borrow_rate);
    }

    pub fn with_borrow_limit(&mut self, borrow_limit: &[(&HumanAddr, &Uint256)]) {
        self.borrow_limit_querier = BorrowLimitQuerier::new(borrow_limit);
    }
}
