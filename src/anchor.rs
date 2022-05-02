use std::str::FromStr;

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Addr, Deps, Env, StdResult, Uint128};

use crate::querier::query_aust_exchange_rate;

pub fn calc_ust_to_aust(
    deps: Deps,
    env: &Env,
    anchor_money_market_addr: &Addr,
    ust_amount: Uint128,
) -> StdResult<Uint128> {
    let aust_exchange_rate = query_aust_exchange_rate(deps, &env, &anchor_money_market_addr)?;
    let aust_amount = {
        let aust_amount = Decimal256::from_str(&ust_amount.to_string())? / aust_exchange_rate;
        Uint128::from(aust_amount * Uint256::one())
    };
    Ok(aust_amount)
}

pub fn calc_aust_to_ust(
    deps: Deps,
    env: &Env,
    anchor_money_market_addr: &Addr,
    aust_amount: Uint128,
) -> StdResult<Uint128> {
    let aust_exchange_rate = query_aust_exchange_rate(deps, &env, &anchor_money_market_addr)?;
    let ust_amount = {
        let ust_amount = Decimal256::from_str(&aust_amount.to_string())? * aust_exchange_rate;
        Uint128::from(ust_amount * Uint256::one())
    };
    Ok(ust_amount)
}
