use cosmwasm_std::{Addr, Binary, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::FlashloanContext;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub anchor_money_market_addr: Addr,
    pub aust_addr: Addr,
    pub minimum_ust_reserved: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Deposit {},
    Redeem { amount: Uint128 },
    Rebalance { reserved_target: Uint128 },
    // loan
    Flashloan { execution: FlashloadExecution },
    FlashloadAssertion { context: FlashloanContext },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FlashloadExecution {
    pub contract_addr: Option<Addr>,
    pub msg: Binary,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    TotalValueLocked {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TotalValueLockedResponse {
    pub tvl: Uint128,
}
