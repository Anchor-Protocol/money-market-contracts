use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ReceiveMsg, MinterResponse};
use protobuf::Message;
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

use moneymarket::ve_aterra::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse,
};

use crate::deposit::{bond_aterra, claim_unlocked_aterra, unbond_ve_aterra};
use crate::error::ContractError;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{read_config, read_state, store_config, store_state, Config, State};

pub const INITIAL_DEPOSIT_AMOUNT: u128 = 1000000;

// #[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    store_config(
        deps.storage,
        &Config {
            contract_addr: deps.api.addr_canonicalize(env.contract.address.as_str())?,
            owner_addr: deps.api.addr_canonicalize(msg.owner_addr.as_str())?,
            market_addr: deps.api.addr_canonicalize(msg.market_addr.as_str())?,
            aterra_contract: deps.api.addr_canonicalize(msg.aterra_contract.as_str())?,
            ve_aterra_contract: CanonicalAddr::from(vec![]),
        },
    )?;

    store_state(
        deps.storage,
        &State {
            ve_aterra_premium_rate: Decimal256::one(),
            prev_ve_aterra_exchange_rate: Decimal256::one(),
            prev_ve_aterra_supply: Uint256::zero(),
            last_executed_height: 0,
            last_ve_aterra_updated: 0,
            target_share: msg.target_share,
            end_goal_share: msg.end_goal_ve_share,
            premium_rate: msg.premium_rate,
        },
    )?;

    Ok(Response::new().add_submessages([
        // create ve aterra cw20 instance
        SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: msg.ve_aterra_code_id,
                funds: vec![],
                label: "".to_string(),
                msg: to_binary(&TokenInstantiateMsg {
                    name: format!(
                        "Vote Escrow Anchor Terra {}",
                        msg.stable_denom[1..].to_uppercase()
                    ),
                    symbol: format!(
                        "vea{}T",
                        msg.stable_denom[1..(msg.stable_denom.len() - 1)].to_uppercase()
                    ),
                    decimals: 6u8,
                    initial_balances: vec![Cw20Coin {
                        address: env.contract.address.to_string(),
                        amount: Uint128::zero(),
                    }],
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                })?,
            }),
            REGISTER_VE_ATERRA_REPLY_ID,
        ),
    ]))
}

const REGISTER_VE_ATERRA_REPLY_ID: u64 = 1;

// #[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {} => {
            todo!()
        }
        ExecuteMsg::ExecuteEpochOperations {} => {
            todo!()
        }
        ExecuteMsg::ClaimATerra {
            amount,
            unlock_time,
        } => claim_unlocked_aterra(deps, env, info, unlock_time, amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REGISTER_VE_ATERRA_REPLY_ID => {
            // get new ve_aterra token's contract address
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(
                msg.result.unwrap().data.unwrap().as_slice(),
            )
            .map_err(|_| {
                ContractError::Std(StdError::parse_err(
                    "MsgInstantiateContractResponse",
                    "failed to parse data",
                ))
            })?;
            let token_addr = Addr::unchecked(res.get_contract_address());
            register_ve_aterra(deps, token_addr)
        }
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

pub fn register_ve_aterra(deps: DepsMut, token_addr: Addr) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if config.ve_aterra_contract != CanonicalAddr::from(vec![]) {
        return Err(ContractError::Unauthorized {});
    }

    config.ve_aterra_contract = deps.api.addr_canonicalize(token_addr.as_str())?;
    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("ve_aterra", token_addr)]))
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let contract_addr = info.sender;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::UnbondVeATerra {}) => {
            // only asset contract can execute this message
            let config: Config = read_config(deps.storage)?;
            if deps.api.addr_canonicalize(contract_addr.as_str())? != config.ve_aterra_contract {
                return Err(ContractError::Unauthorized {});
            }

            let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            unbond_ve_aterra(deps, env, cw20_sender_addr, Uint256::from(cw20_msg.amount))
        }
        Ok(Cw20HookMsg::BondATerra {}) => {
            // only asset contract can execute this message
            let config: Config = read_config(deps.storage)?;
            if deps.api.addr_canonicalize(contract_addr.as_str())? != config.aterra_contract {
                return Err(ContractError::Unauthorized {});
            }

            let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            bond_aterra(deps, env, cw20_sender_addr, Uint256::from(cw20_msg.amount))
        }
        _ => Err(ContractError::UnsupportedCw20Hook(cw20_msg.msg.to_string())),
    }
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    todo!();

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn execute_epoch_operations(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

// #[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State { block_height } => to_binary(&query_state(deps, env, block_height)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    Ok(ConfigResponse {
        contract_addr: deps.api.addr_humanize(&config.contract_addr)?.to_string(),
        owner_addr: deps.api.addr_humanize(&config.owner_addr)?.to_string(),
        market_addr: deps.api.addr_humanize(&config.market_addr)?.to_string(),
        aterra_contract: deps.api.addr_humanize(&config.aterra_contract)?.to_string(),
        ve_aterra_contract: deps
            .api
            .addr_humanize(&config.ve_aterra_contract)?
            .to_string(),
    })
}

pub fn query_state(deps: Deps, env: Env, block_height: Option<u64>) -> StdResult<StateResponse> {
    let block_height = block_height.unwrap_or(env.block.height);
    let state: State = read_state(deps.storage)?;

    if block_height < state.last_ve_aterra_updated {
        return Err(StdError::generic_err(
            "block_height must bigger than last_ve_aterra_updated",
        ));
    }
    if block_height < state.last_executed_height {
        return Err(StdError::generic_err(
            "block_height must bigger than last_executed_height",
        ));
    }

    Ok(StateResponse {})
}
