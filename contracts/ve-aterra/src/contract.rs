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

use moneymarket::ve_aterra::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::bonding::{bond, claim_unlocked_aterra, rebond, unbond};
use crate::error::ContractError;
use crate::execute_epoch_operations::execute_epoch_operations;
use crate::querier::{query_config, query_state};
use crate::response::MsgInstantiateContractResponse;
use crate::state::{read_config, store_config, store_state, Config, State};

const REGISTER_VE_ATERRA_REPLY_ID: u64 = 1;

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
            overseer_addr: deps.api.addr_canonicalize(msg.overseer_addr.as_str())?,
            aterra_contract: deps.api.addr_canonicalize(msg.aterra_contract.as_str())?,
            ve_aterra_contract: CanonicalAddr::from(vec![]),
            max_pos_change: msg.max_pos_change,
            max_neg_change: msg.max_neg_change,
            max_rate: msg.max_rate,
            min_rate: msg.min_rate,
            diff_multiplier: msg.diff_multiplier,
            premium_rate_epoch: msg.premium_rate_epoch,
        },
    )?;

    store_state(
        deps.storage,
        &State {
            prev_epoch_ve_aterra_exchange_rate: Decimal256::one(),
            ve_aterra_supply: Uint256::zero(),
            last_updated: env.block.height,
            target_share: msg.target_share,
            premium_rate: msg.initial_premium_rate,
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

// #[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {
            owner_addr,
            market_addr,
            aterra_contract,
            ve_aterra_contract,
            max_pos_change,
            max_neg_change,
            max_rate,
            min_rate,
            diff_multiplier,
        } => update_config(
            deps,
            info,
            owner_addr,
            market_addr,
            aterra_contract,
            ve_aterra_contract,
            max_pos_change,
            max_neg_change,
            max_rate,
            min_rate,
            diff_multiplier,
        ),
        ExecuteMsg::ExecuteEpochOperations {} => execute_epoch_operations(deps, env, info),
        ExecuteMsg::ClaimATerra { amount } => claim_unlocked_aterra(deps, env, info, amount),
        ExecuteMsg::RebondLockedATerra { amount } => rebond(deps, env, info, amount),
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
            unbond(deps, env, cw20_sender_addr, Uint256::from(cw20_msg.amount))
        }
        Ok(Cw20HookMsg::BondATerra {}) => {
            // only asset contract can execute this message
            let config: Config = read_config(deps.storage)?;
            if deps.api.addr_canonicalize(contract_addr.as_str())? != config.aterra_contract {
                return Err(ContractError::Unauthorized {});
            }

            let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            bond(deps, env, cw20_sender_addr, Uint256::from(cw20_msg.amount))
        }
        _ => Err(ContractError::UnsupportedCw20Hook(cw20_msg.msg.to_string())),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner_addr: Option<String>,
    market_addr: Option<String>,
    aterra_contract: Option<String>,
    ve_aterra_contract: Option<String>,
    max_pos_change: Option<Decimal256>,
    max_neg_change: Option<Decimal256>,
    max_rate: Option<Decimal256>,
    min_rate: Option<Decimal256>,
    diff_multiplier: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner_addr) = owner_addr {
        config.owner_addr = deps.api.addr_canonicalize(&owner_addr)?;
    }
    if let Some(addr) = market_addr {
        config.market_addr = deps.api.addr_canonicalize(&addr)?;
    }
    if let Some(addr) = aterra_contract {
        config.aterra_contract = deps.api.addr_canonicalize(&addr)?;
    }
    if let Some(addr) = ve_aterra_contract {
        config.ve_aterra_contract = deps.api.addr_canonicalize(&addr)?;
    }
    if let Some(max_pos_change) = max_pos_change {
        config.max_pos_change = max_pos_change;
    }
    if let Some(max_neg_change) = max_neg_change {
        config.max_neg_change = max_neg_change;
    }
    if let Some(max_rate) = max_rate {
        config.max_rate = max_rate;
    }
    if let Some(min_rate) = min_rate {
        config.min_rate = min_rate;
    }
    if let Some(diff_multiplier) = diff_multiplier {
        config.diff_multiplier = diff_multiplier;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

// #[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State { block_height } => to_binary(&query_state(deps, env, block_height)?),
    }
}
