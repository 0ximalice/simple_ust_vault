# Simple (not simplest) UST Vault

Building a simple UST Vault that generate yield from Anchor while also have UST reserved for lending, and etc.

This code has been presented in "Interact with Anchor Protocol from Your CosmWasm Smart Contract" session on **Terra Meetup Thailand: How to Dev on Terra, 2022**. _By Alpha Venture DAO x Rustaceans BKK._

## Features

#### 😍 Basic

- Deposit UST
- Redeem UST
- Rebalance: maintain minimum UST reserved for other operations, otherwise deposit the difference to Anchor Protocol.

#### 🤮 Additional

- Flashloan
- Flashloan Assertion
