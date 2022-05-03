#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, WasmMsg,
};
use cw20::Denom;

use crate::anchor::calc_ust_to_aust;
use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, FlashloadExecution, InstantiateMsg, QueryMsg, TotalValueLockedResponse,
};
use crate::querier::{get_deposit_uusd_amount, query_balance, query_uusd_balance, query_vault_tvl};
use crate::state::{FlashloanContext, State, STATE};
use moneymarket::market::{Cw20HookMsg as AnchorMarkerHookMsg, ExecuteMsg as AnchorExecuteMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        anchor_money_market_addr: msg.anchor_money_market_addr,
        aust_addr: msg.aust_addr,
        minimum_ust_reserved: msg.minimum_ust_reserved,
        owner: info.sender.clone(),
    };

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // basic
        ExecuteMsg::Deposit {} => try_deposit(deps, env, info),
        ExecuteMsg::Redeem { amount } => try_redeem(deps, env, info, amount),
        ExecuteMsg::Rebalance { reserved_target } => {
            try_rebalance(deps, env, info, reserved_target)
        }
        // bonus
        ExecuteMsg::Flashloan { execution } => try_flashloan(deps, env, info, execution),
        ExecuteMsg::RepayAssertion { context } => try_repay_assertion(deps, env, info, context),
    }
}

pub fn try_deposit(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if !info.sender.eq(&state.owner) {
        return Err(ContractError::Unauthorized {});
    }

    let mut response = Response::default().add_attribute("action", "deposit");

    // 1. validate deposited denom & amount
    let deposit_amount = get_deposit_uusd_amount(&info);
    if deposit_amount.is_zero() {
        return Err(ContractError::Generic(
            "invalid deposit denom or amount".to_string(),
        ));
    }

    // 2. rebalance
    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::Rebalance {
            reserved_target: state.minimum_ust_reserved,
        })?,
        funds: vec![],
    }));

    Ok(response)
}

pub fn try_redeem(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    redeem_amount: Uint128,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if !info.sender.eq(&state.owner) {
        return Err(ContractError::Unauthorized {});
    }

    let mut response = Response::default().add_attribute("action", "redeem");

    // 1. calculate total value locked
    let tvl = query_vault_tvl(deps.as_ref(), &env, &state)?;
    if tvl.lt(&redeem_amount) {
        return Err(ContractError::Generic("insufficient balance".to_string()));
    }

    // 2. calculate stable needed for reservation & redemption
    let reserved_target = redeem_amount
        .checked_add(state.minimum_ust_reserved)
        .unwrap();
    response = response.add_attribute("redeem:reserved_target", reserved_target);

    // 3. rebalance
    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::Rebalance { reserved_target })?,
        funds: vec![],
    }));

    // 4. transfer ust to redeemer
    response = response.add_message(CosmosMsg::Bank(
        BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: coins(redeem_amount.u128(), "uusd"),
        }
        .into(),
    ));

    Ok(response)
}

fn try_rebalance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    reserved_target: Uint128,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if !info.sender.eq(&env.contract.address) {
        return Err(ContractError::Generic(
            "rebalance can be called by vault only".to_string(),
        ));
    }

    let mut response = Response::default().add_attribute("action", "rebalance");

    // 1. query uusd in vault
    let ust_balance = query_uusd_balance(deps.as_ref(), &env.contract.address)?;

    if ust_balance.gt(&reserved_target) {
        // 2.1 If the current uusd balance greater than minimum uusd reserved amount,
        // we instantly deposit the difference
        let deposit_amount = ust_balance.checked_sub(reserved_target).unwrap();
        response = response.add_attribute("rebalance:deposit_ust", deposit_amount);

        // add anchor deposit msg
        response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: state.anchor_money_market_addr.to_string(),
            msg: to_binary(&AnchorExecuteMsg::DepositStable {})?,
            funds: vec![Coin::new(deposit_amount.u128(), "uusd")],
        }));
    } else if ust_balance.lt(&reserved_target) {
        // 2.2 Otherwise try to redeem more stable from Anchor
        let redeem_ust_amount = reserved_target.checked_sub(ust_balance).unwrap();
        response = response.add_attribute("rebalance:redeem_ust", redeem_ust_amount);

        // calculate aust required
        let mut aust_required = calc_ust_to_aust(
            deps.as_ref(),
            &env,
            &state.anchor_money_market_addr,
            redeem_ust_amount,
        )?;
        response = response.add_attribute("rebalance:redeem_aust", aust_required);

        // have enough aust?
        let aust_balance = query_balance(
            deps.as_ref(),
            &env.contract.address,
            Denom::Cw20(state.aust_addr.clone()),
        )?;
        if aust_balance.lt(&aust_required) {
            aust_required = aust_balance;
        }

        // add anchor redeem msg
        if !aust_balance.is_zero() {
            response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.aust_addr.to_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                    contract: state.anchor_money_market_addr.to_string(),
                    amount: aust_required,
                    msg: to_binary(&AnchorMarkerHookMsg::RedeemStable {})?,
                })?,
                funds: vec![],
            }));
        }
    }

    Ok(response)
}

pub fn try_flashloan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    execution: FlashloadExecution,
) -> Result<Response, ContractError> {
    // 1. assert loan amount
    if execution.amount.is_zero() {
        return Err(ContractError::Generic("invalid loan amount".to_string()));
    }

    let mut response = Response::default().add_attribute("action", "flashloan");

    // 2. do we have enough stable?
    let ust_balance = query_uusd_balance(deps.as_ref(), &env.contract.address)?;
    let mut reserved_target = ust_balance;
    if ust_balance.lt(&execution.amount) {
        // 2.1 fund is not enough, do redemption
        reserved_target = execution.amount.checked_add(ust_balance).unwrap();

        // 2.2 rebalance
        response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::Rebalance { reserved_target })?,
            funds: vec![],
        }));
    }

    // 3. create loan context
    response = response.add_attribute("flashloan:reserved_target", reserved_target);
    let context = FlashloanContext {
        loan_amount: execution.amount,
        stable_before_exec: reserved_target,
    };

    // 4. add user execution
    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        // 🚨 Beware: using customized contract addr lead to exploit CW20
        // in smart contract by adding increase_allowance in execution msg
        contract_addr: info.sender.to_string(),
        msg: execution.msg,
        funds: coins(execution.amount.u128(), "uusd"),
    }));

    // 5. add final repay assertion
    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::RepayAssertion { context: context })?,
        funds: vec![],
    }));

    Ok(response)
}

pub fn try_repay_assertion(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    context: FlashloanContext,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if !info.sender.eq(&env.contract.address) {
        return Err(ContractError::Unauthorized {});
    }

    let mut response = Response::default().add_attribute("action", "flashloan_assertion");

    // 1. validate stable amount after execution
    let ust_balance = query_balance(
        deps.as_ref(),
        &env.contract.address,
        Denom::Native("uusd".to_string()),
    )?;
    if ust_balance.lt(&context.stable_before_exec) {
        return Err(ContractError::Generic("imbalance repay".to_string()));
    }

    // TODO: 💵 flashloan fees assertion

    // 2. rebalance
    response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::Rebalance {
            reserved_target: state.minimum_ust_reserved,
        })?,
        funds: vec![],
    }));

    Ok(response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::TotalValueLocked {} => to_binary(&query_tvl(deps, env)?),
    }
}

fn query_tvl(deps: Deps, env: Env) -> StdResult<TotalValueLockedResponse> {
    let state = STATE.load(deps.storage)?;
    let tvl = query_vault_tvl(deps, &env, &state)?;
    Ok(TotalValueLockedResponse { tvl: tvl })
}
