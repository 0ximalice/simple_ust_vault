use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub anchor_money_market_addr: Addr,
    pub aust_addr: Addr,
    pub owner: Addr,
    pub minimum_ust_reserved: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FlashloanContext {
    pub loan_amount: Uint128,
    pub stable_before_exec: Uint128,
}

pub const STATE: Item<State> = Item::new("state");
