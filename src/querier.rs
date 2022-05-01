use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{
    to_binary, Addr, Deps, Env, MessageInfo, QueryRequest, StdResult, Uint128, WasmQuery,
};
use cw20::Denom;
use moneymarket::market::{EpochStateResponse, QueryMsg as AnchorQueryMsg};

use crate::{anchor::calc_aust_to_ust, state::State};

pub fn get_deposit_uusd_amount(info: &MessageInfo) -> Uint128 {
    return match info.funds.len() {
        1 => {
            let token = &info.funds[0];
            if token.denom != "uusd" {
                return Uint128::zero();
            }
            token.amount
        }
        _ => Uint128::zero(),
    };
}

pub fn query_aust_exchange_rate(
    deps: Deps,
    env: &Env,
    anchor_money_market_addr: &Addr,
) -> StdResult<Decimal256> {
    let response: EpochStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: anchor_money_market_addr.to_string(),
            msg: to_binary(&AnchorQueryMsg::EpochState {
                block_height: Some(env.block.height),
                distributed_interest: None,
            })?,
        }))?;
    Ok(response.exchange_rate)
}

pub fn query_uusd_balance(deps: Deps, address: &Addr) -> StdResult<Uint128> {
    query_balance(deps, address, Denom::Native("uusd".to_string()))
}

pub fn query_balance(deps: Deps, address: &Addr, denom: Denom) -> StdResult<Uint128> {
    match denom {
        Denom::Native(denom) => {
            let bal = deps.querier.query_balance(address, denom.as_str())?;
            Ok(bal.amount)
        }
        Denom::Cw20(contract_addr) => {
            let bal: StdResult<cw20::BalanceResponse> = deps.querier.query_wasm_smart(
                contract_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: address.to_string(),
                },
            );
            Ok(bal?.balance)
        }
    }
}

pub fn query_vault_tvl(deps: Deps, env: &Env, state: &State) -> StdResult<Uint128> {
    let ust_balance = query_uusd_balance(deps, &env.contract.address)?;
    let aust_balance = query_balance(
        deps,
        &env.contract.address,
        Denom::Cw20(state.aust_addr.clone()),
    )?;
    let aust_equity = calc_aust_to_ust(deps, &env, &state.anchor_money_market_addr, aust_balance)?;
    Ok(ust_balance.checked_add(aust_equity)?)
}
