use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, CanonicalAddr, Coin, Decimal, Extern, HumanAddr,
    Querier, QuerierResult, QueryRequest, SystemError, Uint128, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use std::collections::HashMap;

use cw20::TokenInfoResponse;
use moneymarket::{
    DistributionParamsResponse, EpochStateResponse, LoanAmountResponse, OraclePriceResponse,
};
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query epoch state to market contract
    EpochState {},
    /// Query borrow amount to market contract
    LoanAmount { borrower: HumanAddr },
    /// Query oracle price to oracle contract
    OraclePrice { base: String, quote: String },
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
    epoch_state_querier: EpochStateQuerier,
    oracle_price_querier: OraclePriceQuerier,
    borrow_amount_querier: LoanAmountQuerier,
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
pub struct OraclePriceQuerier {
    // this lets us iterate over all pairs that match the first string
    oracle_price: HashMap<(String, String), (Decimal, u64, u64)>,
}

impl OraclePriceQuerier {
    pub fn new(oracle_price: &[(&(String, String), &(Decimal, u64, u64))]) -> Self {
        OraclePriceQuerier {
            oracle_price: oracle_price_to_map(oracle_price),
        }
    }
}

pub(crate) fn oracle_price_to_map(
    oracle_price: &[(&(String, String), &(Decimal, u64, u64))],
) -> HashMap<(String, String), (Decimal, u64, u64)> {
    let mut oracle_price_map: HashMap<(String, String), (Decimal, u64, u64)> = HashMap::new();
    for (base_quote, oracle_price) in oracle_price.iter() {
        oracle_price_map.insert((*base_quote).clone(), **oracle_price);
    }

    oracle_price_map
}

#[derive(Clone, Default)]
pub struct EpochStateQuerier {
    // this lets us iterate over all pairs that match the first string
    epoch_state: HashMap<HumanAddr, (Uint128, Decimal)>,
}

impl EpochStateQuerier {
    pub fn new(epoch_state: &[(&HumanAddr, &(Uint128, Decimal))]) -> Self {
        EpochStateQuerier {
            epoch_state: epoch_state_to_map(epoch_state),
        }
    }
}

pub(crate) fn epoch_state_to_map(
    epoch_state: &[(&HumanAddr, &(Uint128, Decimal))],
) -> HashMap<HumanAddr, (Uint128, Decimal)> {
    let mut epoch_state_map: HashMap<HumanAddr, (Uint128, Decimal)> = HashMap::new();
    for (market_contract, epoch_state) in epoch_state.iter() {
        epoch_state_map.insert((*market_contract).clone(), **epoch_state);
    }
    epoch_state_map
}

#[derive(Clone, Default)]
pub struct LoanAmountQuerier {
    // this lets us iterate over all pairs that match the first string
    borrower_amount: HashMap<HumanAddr, Uint128>,
}

impl LoanAmountQuerier {
    pub fn new(borrower_amount: &[(&HumanAddr, &Uint128)]) -> Self {
        LoanAmountQuerier {
            borrower_amount: borrower_amount_to_map(borrower_amount),
        }
    }
}

pub(crate) fn borrower_amount_to_map(
    borrower_amount: &[(&HumanAddr, &Uint128)],
) -> HashMap<HumanAddr, Uint128> {
    let mut borrower_amount_map: HashMap<HumanAddr, Uint128> = HashMap::new();
    for (market_contract, borrower_amount) in borrower_amount.iter() {
        borrower_amount_map.insert((*market_contract).clone(), **borrower_amount);
    }
    borrower_amount_map
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
                    QueryMsg::EpochState {} => {
                        match self.epoch_state_querier.epoch_state.get(&contract_addr) {
                            Some(v) => Ok(to_binary(&EpochStateResponse {
                                a_token_supply: v.0,
                                exchange_rate: v.1,
                            })),
                            None => Err(SystemError::InvalidRequest {
                                error: "No epoch state exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    QueryMsg::LoanAmount { borrower } => {
                        match self.borrow_amount_querier.borrower_amount.get(&borrower) {
                            Some(v) => Ok(to_binary(&LoanAmountResponse {
                                borrower,
                                loan_amount: *v,
                            })),
                            None => Err(SystemError::InvalidRequest {
                                error: "No borrow amount exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    QueryMsg::OraclePrice { base, quote } => {
                        match self.oracle_price_querier.oracle_price.get(&(base, quote)) {
                            Some(v) => Ok(to_binary(&OraclePriceResponse {
                                rate: v.0,
                                last_updated_base: v.1,
                                last_updated_quote: v.2,
                            })),
                            None => Err(SystemError::InvalidRequest {
                                error: "No oracle price exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
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
            epoch_state_querier: EpochStateQuerier::default(),
            oracle_price_querier: OraclePriceQuerier::default(),
            borrow_amount_querier: LoanAmountQuerier::default(),
            canonical_length,
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    // configure the tax mock querier
    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    pub fn with_epoch_state(&mut self, epoch_state: &[(&HumanAddr, &(Uint128, Decimal))]) {
        self.epoch_state_querier = EpochStateQuerier::new(epoch_state);
    }

    pub fn with_oracle_price(
        &mut self,
        oracle_price: &[(&(String, String), &(Decimal, u64, u64))],
    ) {
        self.oracle_price_querier = OraclePriceQuerier::new(oracle_price);
    }

    pub fn with_borrow_amount(&mut self, borrow_amount: &[(&HumanAddr, &Uint128)]) {
        self.borrow_amount_querier = LoanAmountQuerier::new(borrow_amount);
    }
}
