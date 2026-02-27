# Zero-Amount Operation Semantics

This document specifies the expected behavior of all amount-bearing operations
in the StellarLend contracts when called with zero or negative amounts.

## Core Lending Operations

All core lending operations **reject** amounts ≤ 0 with their respective
`InvalidAmount` error variants. No state mutations occur on rejection.

| Operation              | Zero / Negative Amount Result              |
|------------------------|--------------------------------------------|
| `deposit_collateral`   | `Err(DepositError::InvalidAmount)`         |
| `withdraw_collateral`  | `Err(WithdrawError::InvalidAmount)`        |
| `borrow_asset`         | `Err(BorrowError::InvalidAmount)`          |
| `repay_debt`           | `Err(RepayError::InvalidAmount)`           |

### Invariants

1. **No state mutation**: When an operation returns an error, storage (balances,
   positions, analytics) must remain exactly as before the call.
2. **Clean revert**: The operation returns a typed `Result::Err`, not an
   unhandled panic or abort.
3. **Composability**: A rejected zero-amount operation must not corrupt state
   for subsequent valid operations.

## Risk Management / Liquidation Functions

These functions accept zero values and handle them gracefully:

| Function                            | Zero-Value Behavior                       |
|-------------------------------------|-------------------------------------------|
| `can_be_liquidated(_, 0)`           | `Ok(false)` — no debt means not liquidatable |
| `can_be_liquidated(0, debt)`        | `Ok(true)` — zero collateral is liquidatable |
| `can_be_liquidated(0, 0)`           | `Ok(false)` — no debt means not liquidatable |
| `get_max_liquidatable_amount(0)`    | `Ok(0)` — nothing to liquidate             |
| `get_liquidation_incentive_amount(0)` | `Ok(0)` — no incentive for zero amount   |
| `require_min_collateral_ratio(_, 0)`| `Ok(())` — no debt always satisfies ratio  |

## References

- **Issue**: [#385 - Zero-Amount Operation Handling Tests](https://github.com/StellarLend/stellarlend-contracts/issues/385)
- **Test module**: `stellar-lend/contracts/hello-world/src/test_zero_amount.rs`
