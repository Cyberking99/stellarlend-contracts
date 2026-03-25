use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

fn setup_test(
    env: &Env,
) -> (
    LendingContractClient<'_>,
    Address,
    Address,
    Address,
    Address,
) {
    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let user = Address::generate(env);
    let asset = Address::generate(env);
    let collateral_asset = Address::generate(env);

    client.initialize(&admin, &1_000_000_000, &1000);
    (client, admin, user, asset, collateral_asset)
}

#[test]
fn test_borrow_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, user, asset, collateral_asset) = setup_test(&env);

    client.borrow(&user, &asset, &10_000, &collateral_asset, &20_000);

    let debt = client.get_user_debt(&user);
    assert_eq!(debt.borrowed_amount, 10_000);
    assert_eq!(debt.interest_accrued, 0);

    let collateral = client.get_user_collateral(&user);
    assert_eq!(collateral.amount, 20_000);
}

#[test]
fn test_borrow_insufficient_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, user, asset, collateral_asset) = setup_test(&env);

    let result = client.try_borrow(&user, &asset, &10_000, &collateral_asset, &10_000);
    assert_eq!(result, Err(Ok(BorrowError::InsufficientCollateral)));
}

#[test]
fn test_borrow_protocol_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, user, asset, collateral_asset) = setup_test(&env);

    client.set_pause(&admin, &PauseType::Borrow, &true);

    let result = client.try_borrow(&user, &asset, &10_000, &collateral_asset, &20_000);
    assert_eq!(result, Err(Ok(BorrowError::ProtocolPaused)));
}

#[test]
fn test_borrow_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, user, asset, collateral_asset) = setup_test(&env);

    let result = client.try_borrow(&user, &asset, &0, &collateral_asset, &20_000);
    assert_eq!(result, Err(Ok(BorrowError::InvalidAmount)));

    let result = client.try_borrow(&user, &asset, &10_000, &collateral_asset, &0);
    assert_eq!(result, Err(Ok(BorrowError::InvalidAmount)));
}

#[test]
fn test_borrow_below_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    client.initialize(&admin, &1_000_000_000, &5000);

    let result = client.try_borrow(&user, &asset, &1000, &collateral_asset, &2000);
    assert_eq!(result, Err(Ok(BorrowError::BelowMinimumBorrow)));
}

#[test]
fn test_borrow_debt_ceiling() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    client.initialize(&admin, &50_000, &1000);

    let result = client.try_borrow(&user, &asset, &100_000, &collateral_asset, &200_000);
    assert_eq!(result, Err(Ok(BorrowError::DebtCeilingReached)));
}

#[test]
fn test_borrow_multiple_times() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, user, asset, collateral_asset) = setup_test(&env);

    client.borrow(&user, &asset, &10_000, &collateral_asset, &20_000);
    client.borrow(&user, &asset, &5_000, &collateral_asset, &10_000);

    let debt = client.get_user_debt(&user);
    assert_eq!(debt.borrowed_amount, 15_000);

    let collateral = client.get_user_collateral(&user);
    assert_eq!(collateral.amount, 30_000);
}

#[test]
fn test_borrow_interest_accrual() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let (client, _admin, user, asset, collateral_asset) = setup_test(&env);
    client.borrow(&user, &asset, &100_000, &collateral_asset, &200_000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + 31536000; // 1 year later
    });

    let debt = client.get_user_debt(&user);
    assert!(debt.interest_accrued > 0);
    assert!(debt.interest_accrued <= 5000); // ~5% of 100,000
}

#[test]
fn test_collateral_ratio_validation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, user, asset, collateral_asset) = setup_test(&env);

    // Exactly 150% collateral - should succeed
    client.borrow(&user, &asset, &10_000, &collateral_asset, &15_000);

    // Below 150% collateral - should fail
    let user2 = Address::generate(&env);
    let result = client.try_borrow(&user2, &asset, &10_000, &collateral_asset, &14_999);
    assert_eq!(result, Err(Ok(BorrowError::InsufficientCollateral)));
}

#[test]
fn test_pause_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, user, asset, collateral_asset) = setup_test(&env);

    client.set_pause(&admin, &PauseType::Borrow, &true);
    let result = client.try_borrow(&user, &asset, &10_000, &collateral_asset, &20_000);
    assert_eq!(result, Err(Ok(BorrowError::ProtocolPaused)));

    client.set_pause(&admin, &PauseType::Borrow, &false);
    client.borrow(&user, &asset, &10_000, &collateral_asset, &20_000);
}

#[test]
fn test_overflow_protection() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LendingContract, ());
    let client = LendingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let collateral_asset = Address::generate(&env);

    client.initialize(&admin, &i128::MAX, &1000);

    // First borrow with reasonable amount
    client.borrow(&user, &asset, &1_000_000, &collateral_asset, &2_000_000);

    // Try to borrow amount that would overflow when added to existing debt
    let huge_amount = i128::MAX - 500_000;
    let huge_collateral = i128::MAX / 2; // Large but won't overflow in calculation
    let result = client.try_borrow(
        &user,
        &asset,
        &huge_amount,
        &collateral_asset,
        &huge_collateral,
    );
    assert_eq!(result, Err(Ok(BorrowError::Overflow)));
}

#[test]
fn test_coverage_boost_lib() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, user, asset, _) = setup_test(&env);

    // Hit get_performance_stats
    let _stats = client.get_performance_stats();

    // Hit get_admin
    assert_eq!(client.get_admin(), Some(admin.clone()));
    
    // Hit data_store methods via lib
    client.data_store_init(&admin);
    client.data_store_init(&admin); // double to hit early return
    client.data_grant_writer(&admin, &user);
    client.data_revoke_writer(&admin, &user);
    let _ = client.data_key_exists(&soroban_sdk::String::from_str(&env, "test"));
    
    // Note: deposit paused
    client.set_pause(&admin, &PauseType::Deposit, &true);
    let dep_res = client.try_deposit(&user, &asset, &100);
    assert_eq!(dep_res, Err(Ok(DepositError::DepositPaused)));
    
    let dep_res2 = client.try_deposit_collateral(&user, &asset, &100);
    assert_eq!(dep_res2, Err(Ok(BorrowError::ProtocolPaused)));
    
    // Repay paused
    client.set_pause(&admin, &PauseType::Repay, &true);
    let rep_res = client.try_repay(&user, &asset, &100);
    assert_eq!(rep_res, Err(Ok(BorrowError::ProtocolPaused)));
    
    // Liquidate paused
    client.set_pause(&admin, &PauseType::Liquidation, &true);
    let liq_res = client.try_liquidate(&admin, &user, &asset, &asset, &100);
    assert_eq!(liq_res, Err(Ok(BorrowError::ProtocolPaused)));
}

#[test]
fn test_coverage_boost_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, user, _asset, _) = setup_test(&env);
    
    // Emergency states
    assert_eq!(client.get_emergency_state(), EmergencyState::Normal);
    
    // Setup guardian
    client.set_guardian(&admin, &user);
    assert_eq!(client.get_guardian(), Some(user.clone()));
    
    // trigger shutdown
    client.emergency_shutdown(&user); // caller is guardian
    assert_eq!(client.get_emergency_state(), EmergencyState::Shutdown);
    
    // try recovery
    client.start_recovery(&admin);
    assert_eq!(client.get_emergency_state(), EmergencyState::Recovery);
    
    // Complete recovery
    client.complete_recovery(&admin);
    assert_eq!(client.get_emergency_state(), EmergencyState::Normal);
}
