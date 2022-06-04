use astroport::token::InstantiateMsg as TokenInstantiateMsg;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    coins, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use moneymarket::custody::{
    BAssetInfo, ExecuteMsg as CustodyExecuteMsg, InstantiateMsg as CustodyInstantiateMsg,
    QueryMsg as CustodyQueryMsg,
};
use moneymarket::distribution_model::InstantiateMsg as DistributionModelInstantiateMsg;
use moneymarket::interest_model::InstantiateMsg as InterestModelInstantiateMsg;
use moneymarket::market::{
    BorrowerInfoResponse, ExecuteMsg as MarketExecuteMsg, InstantiateMsg as MarketInstantiateMsg,
    MigrateMsg as MarketMigrateMsg, QueryMsg as MarketQueryMsg,
};
use moneymarket::oracle::{ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg};
use moneymarket::overseer::{
    ExecuteMsg as OverseerExecuteMsg, InstantiateMsg as OverseerInstantiateMsg,
    MigrateMsg as OverseerMigrateMsg,
};
use std::str::FromStr;
use terra_multi_test::{AppBuilder, BankKeeper, ContractWrapper, Executor, TerraApp, TerraMock};

const OWNER: &str = "owner";
const USER: &str = "user";
const ADMIN: &str = "admin";

fn mock_app() -> TerraApp {
    let env = mock_env();
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();
    let custom = TerraMock::luna_ust_case();

    AppBuilder::new()
        .with_api(api)
        .with_block(env.block)
        .with_bank(bank)
        .with_storage(storage)
        .with_custom(custom)
        .build()
}

fn mock_custody_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: CustodyInstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

fn mock_custody_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: CustodyExecuteMsg,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

fn mock_custody_query(_deps: Deps, _env: Env, _msg: CustodyQueryMsg) -> StdResult<Binary> {
    to_binary(&())
}

fn store_token_contract_code(app: &mut TerraApp) -> u64 {
    let token_contracct = Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code(token_contracct)
}

fn store_custody_contract_code(app: &mut TerraApp) -> u64 {
    let custody_contract = Box::new(ContractWrapper::new_with_empty(
        mock_custody_execute,
        mock_custody_instantiate,
        mock_custody_query,
    ));
    app.store_code(custody_contract)
}

fn store_market_contract_code_old(app: &mut TerraApp) -> u64 {
    let market_contract = Box::new(
        ContractWrapper::new_with_empty(
            moneymarket_market_old::contract::execute,
            moneymarket_market_old::contract::instantiate,
            moneymarket_market_old::contract::query,
        )
        .with_reply_empty(moneymarket_market_old::contract::reply),
    );

    app.store_code(market_contract)
}

fn store_market_contract_code(app: &mut TerraApp) -> u64 {
    let market_contract = Box::new(
        ContractWrapper::new_with_empty(
            moneymarket_market::contract::execute,
            moneymarket_market::contract::instantiate,
            moneymarket_market::contract::query,
        )
        .with_reply_empty(moneymarket_market::contract::reply)
        .with_migrate_empty(moneymarket_market::contract::migrate),
    );

    app.store_code(market_contract)
}

fn store_overseer_contract_code_old(app: &mut TerraApp) -> u64 {
    let overseer_contract = Box::new(ContractWrapper::new_with_empty(
        moneymarket_overseer_old::contract::execute,
        moneymarket_overseer_old::contract::instantiate,
        moneymarket_overseer_old::contract::query,
    ));

    app.store_code(overseer_contract)
}

fn store_overseer_contract_code(app: &mut TerraApp) -> u64 {
    let overseer_contract = Box::new(
        ContractWrapper::new_with_empty(
            moneymarket_overseer::contract::execute,
            moneymarket_overseer::contract::instantiate,
            moneymarket_overseer::contract::query,
        )
        .with_migrate_empty(moneymarket_overseer::contract::migrate),
    );

    app.store_code(overseer_contract)
}

fn store_oracle_contract_code(app: &mut TerraApp) -> u64 {
    let oracle_contract = Box::new(ContractWrapper::new_with_empty(
        moneymarket_oracle::contract::execute,
        moneymarket_oracle::contract::instantiate,
        moneymarket_oracle::contract::query,
    ));

    app.store_code(oracle_contract)
}

fn store_interest_model_code(app: &mut TerraApp) -> u64 {
    let interest_model_contract = Box::new(ContractWrapper::new_with_empty(
        moneymarket_interest_model::contract::execute,
        moneymarket_interest_model::contract::instantiate,
        moneymarket_interest_model::contract::query,
    ));

    app.store_code(interest_model_contract)
}

fn store_distribution_model_code(app: &mut TerraApp) -> u64 {
    let distribution_model_contract = Box::new(ContractWrapper::new_with_empty(
        moneymarket_distribution_model::contract::execute,
        moneymarket_distribution_model::contract::instantiate,
        moneymarket_distribution_model::contract::query,
    ));

    app.store_code(distribution_model_contract)
}

fn create_contracts() -> (TerraApp, Addr, Addr, Addr, Addr, Addr) {
    let mut app = mock_app();
    let owner = Addr::unchecked(OWNER);
    let admin = Addr::unchecked(ADMIN);

    app.init_bank_balance(&owner, coins(10000000, "uusd"))
        .unwrap();

    // these 3 contracts are not needed for now
    let liquidator_addr = "liquidation_addr";
    let collector_addr = "collector_addr";
    let distributor_addr = "distributor_addr";
    let reward_contract_addr = "reward_contract_addr";

    // store contract codes
    let token_code_id = store_token_contract_code(&mut app);
    let custody_code_id = store_custody_contract_code(&mut app);
    let oracle_code_id = store_oracle_contract_code(&mut app);
    let interest_model_code_id = store_interest_model_code(&mut app);
    let distribution_model_code_id = store_distribution_model_code(&mut app);
    let market_code_id_old = store_market_contract_code_old(&mut app);
    let overseer_code_id_old = store_overseer_contract_code_old(&mut app);

    // instantiate oracle contract
    let msg = OracleInstantiateMsg {
        owner: owner.to_string(),
        base_asset: "uusd".to_string(),
    };
    let oracle_addr = app
        .instantiate_contract(
            oracle_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("ORACLE"),
            None,
        )
        .unwrap();

    // instantiate interest model contract
    let msg = InterestModelInstantiateMsg {
        owner: owner.to_string(),
        base_rate: Decimal256::percent(10),
        interest_multiplier: Decimal256::percent(10),
    };
    let interest_model_addr = app
        .instantiate_contract(
            interest_model_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("INTEREST MODEL"),
            None,
        )
        .unwrap();

    // instantiate distribution model contract
    let msg = DistributionModelInstantiateMsg {
        owner: owner.to_string(),
        emission_cap: Decimal256::from_uint256(100u64),
        emission_floor: Decimal256::from_uint256(10u64),
        increment_multiplier: Decimal256::percent(110),
        decrement_multiplier: Decimal256::percent(90),
    };
    let distribution_model_addr = app
        .instantiate_contract(
            distribution_model_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("INTEREST MODEL"),
            None,
        )
        .unwrap();

    // instantitate market contract
    let msg = MarketInstantiateMsg {
        owner_addr: owner.to_string(),
        stable_denom: "uusd".to_string(),
        aterra_code_id: token_code_id,
        anc_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };
    let market_addr = app
        .instantiate_contract(
            market_code_id_old,
            owner.clone(),
            &msg,
            &coins(1000000, "uusd"),
            String::from("MARKET"),
            Some(admin.to_string()),
        )
        .unwrap();

    // instantiate overseer contract
    let msg = OverseerInstantiateMsg {
        owner_addr: owner.to_string(),
        oracle_contract: oracle_addr.to_string(),
        market_contract: market_addr.to_string(),
        liquidation_contract: liquidator_addr.to_string(),
        collector_contract: collector_addr.to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        anc_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 8600u64,
        dyn_rate_maxchange: Decimal256::permille(5),
        dyn_rate_yr_increase_expectation: Decimal256::permille(1),
        dyn_rate_min: Decimal256::from_ratio(1000000000000u64, 1000000000000000000u64),
        dyn_rate_max: Decimal256::from_ratio(1200000000000u64, 1000000000000000000u64),
    };
    let overseer_addr = app
        .instantiate_contract(
            overseer_code_id_old,
            owner.clone(),
            &msg,
            &[],
            String::from("OVERSEER"),
            Some(admin.to_string()),
        )
        .unwrap();

    // register contracts to market
    let msg = MarketExecuteMsg::RegisterContracts {
        overseer_contract: overseer_addr.to_string(),
        interest_model: interest_model_addr.to_string(),
        distribution_model: distribution_model_addr.to_string(),
        collector_contract: collector_addr.to_string(),
        distributor_contract: distributor_addr.to_string(),
    };

    app.execute_contract(owner.clone(), market_addr.clone(), &msg, &[])
        .unwrap();

    // instantiate bluna
    let msg = TokenInstantiateMsg {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: None,
    };

    let bluna_token_addr = app
        .instantiate_contract(token_code_id, owner.clone(), &msg, &[], "bluna", None)
        .unwrap();

    // instantiate custody contract
    let msg = CustodyInstantiateMsg {
        owner: owner.to_string(),
        collateral_token: bluna_token_addr.to_string(),
        overseer_contract: overseer_addr.to_string(),
        market_contract: market_addr.to_string(),
        reward_contract: reward_contract_addr.to_string(),
        liquidation_contract: liquidator_addr.to_string(),
        stable_denom: "uusd".to_string(),
        basset_info: BAssetInfo {
            name: "bluna".to_string(),
            symbol: "bluna".to_string(),
            decimals: 6,
        },
    };

    let custody_contract_addr = app
        .instantiate_contract(
            custody_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("CUSTODY"),
            None,
        )
        .unwrap();

    (
        app,
        market_addr,
        overseer_addr,
        bluna_token_addr,
        custody_contract_addr,
        oracle_addr,
    )
}

fn migrate_contracts(app: &mut TerraApp, market_addr: &Addr, overseer_addr: &Addr) {
    let admin = Addr::unchecked(ADMIN);

    // store new contract code
    let market_code_id = store_market_contract_code(app);
    let overseer_code_id = store_overseer_contract_code(app);

    // migrate market contract
    let msg = MarketMigrateMsg {};
    app.migrate_contract(admin.clone(), market_addr.clone(), &msg, market_code_id)
        .unwrap();

    // migrate overseer contract
    let msg = OverseerMigrateMsg {};
    app.migrate_contract(admin, overseer_addr.clone(), &msg, overseer_code_id)
        .unwrap();
}

#[test]
fn test_migration() {
    let (mut app, market_addr, overseer_addr, _, _, _) = create_contracts();
    migrate_contracts(&mut app, &market_addr, &overseer_addr);
}

#[test]
fn test_successfully_repay_stable_from_yield_reserve() {
    let owner = Addr::unchecked(OWNER);
    let user = Addr::unchecked(USER);

    let (mut app, market_addr, overseer_addr, bluna_token_addr, custody_contract_addr, oracle_addr) =
        create_contracts();
    app.init_bank_balance(&market_addr, coins(847_426_363u128, "uusd"))
        .unwrap();
    app.init_bank_balance(&overseer_addr, coins(1_000_000_000u128, "uusd"))
        .unwrap();

    // register whitelist
    let msg = OverseerExecuteMsg::Whitelist {
        name: "bluna".to_string(),
        symbol: "bluna".to_string(),
        collateral_token: bluna_token_addr.to_string(),
        custody_contract: custody_contract_addr.to_string(),
        max_ltv: Decimal256::percent(60),
    };

    app.execute_contract(owner.clone(), overseer_addr.clone(), &msg, &[])
        .unwrap();

    // lock some bluna
    let msg = OverseerExecuteMsg::LockCollateral {
        collaterals: vec![(
            bluna_token_addr.to_string(),
            Uint256::from(1_000_000_000u64),
        )],
    };

    app.execute_contract(user.clone(), overseer_addr.clone(), &msg, &[])
        .unwrap();

    // feed bluna price
    let msg = OracleExecuteMsg::RegisterFeeder {
        asset: bluna_token_addr.to_string(),
        feeder: owner.to_string(),
    };

    app.execute_contract(owner.clone(), oracle_addr.clone(), &msg, &[])
        .unwrap();

    let msg = OracleExecuteMsg::FeedPrice {
        prices: vec![(
            bluna_token_addr.to_string(),
            Decimal256::from_str("10").unwrap(),
        )],
    };

    app.execute_contract(owner, oracle_addr, &msg, &[]).unwrap();

    // borrow UST agaist bluna
    let msg = MarketExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(847_426_363u64),
        to: None,
    };

    app.execute_contract(user.clone(), market_addr.clone(), &msg, &[])
        .unwrap();

    migrate_contracts(&mut app, &market_addr, &overseer_addr);

    // repay stable from yield reserve
    let msg = OverseerExecuteMsg::RepayStableFromYieldReserve {
        borrower: user.to_string(),
    };

    app.execute_contract(user.clone(), overseer_addr.clone(), &msg, &[])
        .unwrap();

    // check remain loan amount of the user
    let res: BorrowerInfoResponse = app
        .wrap()
        .query_wasm_smart(
            market_addr.clone(),
            &MarketQueryMsg::BorrowerInfo {
                borrower: user.to_string(),
                block_height: None,
            },
        )
        .unwrap();

    assert_eq!(res.loan_amount, Uint256::zero());
}
