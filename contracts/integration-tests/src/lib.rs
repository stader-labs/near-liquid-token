mod context;
mod helpers;

use crate::helpers::ntoy;
use context::IntegrationTestContext;
use near_sdk::json_types::{U128, U64};
use near_units::*;
use near_x::state::{AccountResponse, Fraction, NearxPoolStateResponse, ValidatorInfoResponse};
use serde_json::json;

// Tests: Autocompound with operator rewards and autocompound in the same epoch
#[tokio::test]
async fn test_autocompound_with_operator_rewards() -> anyhow::Result<()> {
    let context = IntegrationTestContext::new().await?;

    // Add user deposits
    println!("User 1 depositing");
    context.deposit(&context.user1).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context.deposit(&context.user2).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context.deposit(&context.user3).await?;
    println!("User 3 successfully deposited");

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(10)));
    assert_eq!(user2_staked_amount, U128(ntoy(10)));
    assert_eq!(user3_staked_amount, U128(ntoy(10)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(10)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(ntoy(30)));

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(30)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(0)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));

    // Set reward fee to 10%
    context
        .set_reward_fee(Fraction {
            numerator: 10,
            denominator: 100,
        })
        .await?;
    let reward_fee = context.get_reward_fee().await?;
    assert_eq!(reward_fee.numerator, 10);
    assert_eq!(reward_fee.denominator, 100);

    // Add 30Near of rewards
    context.add_stake_pool_rewards(U128(ntoy(30))).await?;

    // Get the operator details
    let operator_account = context
        .worker
        .view_account(&context.nearx_operator.id())
        .await?;
    let previous_operator_balance = operator_account.balance;

    // auto compound the rewards
    context
        .auto_compound_rewards(context.stake_pool_contract.id())
        .await?;

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(2)));

    let validator = context
        .get_validator_info(context.stake_pool_contract.id().clone())
        .await?;
    println!("validator is {:?}", validator);
    assert_eq!(
        validator,
        ValidatorInfoResponse {
            account_id: validator.account_id.clone(),
            staked: U128(ntoy(60)),
            last_asked_rewards_epoch_height: context.get_current_epoch().await?,
            lock: false
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(60)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));

    let operator_account = context
        .worker
        .view_account(&context.nearx_operator.id())
        .await?;
    let current_operator_balance = operator_account.balance;

    assert_eq!(
        (current_operator_balance - previous_operator_balance),
        ntoy(3)
    );

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(10)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(20)));
    assert_eq!(user2_staked_amount, U128(ntoy(20)));
    assert_eq!(user3_staked_amount, U128(ntoy(20)));

    // Deposit with NearX price > 1
    println!("User 1 depositing");
    context.deposit(&context.user1).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context.deposit(&context.user2).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context.deposit(&context.user3).await?;
    println!("User 3 successfully deposited");

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(30)));
    assert_eq!(user2_staked_amount, U128(ntoy(30)));
    assert_eq!(user3_staked_amount, U128(ntoy(30)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(15)));
    assert_eq!(user2_token_balance, U128(ntoy(15)));
    assert_eq!(user3_token_balance, U128(ntoy(15)));

    let total_token_supply = context.get_total_tokens_supply().await?;
    assert_eq!(total_token_supply, U128(ntoy(45)));

    let validator = context
        .get_validator_info(context.stake_pool_contract.id().clone())
        .await?;
    println!("validator is {:?}", validator);
    assert_eq!(
        validator,
        ValidatorInfoResponse {
            account_id: validator.account_id.clone(),
            staked: U128(ntoy(90)),
            last_asked_rewards_epoch_height: context.get_current_epoch().await?,
            lock: false
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(90)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));

    // Autocompound in the same epoch
    let operator_account = context
        .worker
        .view_account(&context.nearx_operator.id())
        .await?;
    let previous_operator_balance = operator_account.balance;

    context
        .auto_compound_rewards(context.stake_pool_contract.id())
        .await?;

    let operator_account = context
        .worker
        .view_account(&context.nearx_operator.id())
        .await?;
    let current_operator_balance = operator_account.balance;

    let validator = context
        .get_validator_info(context.stake_pool_contract.id().clone())
        .await?;
    println!("validator is {:?}", validator);
    assert_eq!(
        validator,
        ValidatorInfoResponse {
            account_id: validator.account_id.clone(),
            staked: U128(ntoy(90)),
            last_asked_rewards_epoch_height: context.get_current_epoch().await?,
            lock: false
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(90)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(45)));

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(2)));

    assert_eq!((current_operator_balance - previous_operator_balance), 0);

    Ok(())
}

// Tests: Autocompound with no stake
#[tokio::test]
async fn test_autocompound_with_no_stake() -> anyhow::Result<()> {
    println!("***** Step 1: Initialization *****");
    let context = IntegrationTestContext::new().await?;

    // Auto compound
    context.auto_compound_rewards(context.stake_pool_contract.id());

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(1)));
    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(0));
    assert_eq!(nearx_state.total_staked, U128(0));
    assert_eq!(nearx_state.total_stake_shares, U128(0));

    let validator_info = context
        .get_validator_info(context.stake_pool_contract.id().clone())
        .await?;
    assert_eq!(validator_info.staked, U128(0));
    assert_eq!(validator_info.last_asked_rewards_epoch_height, U64(0));

    Ok(())
}

#[tokio::test]
async fn test_deposit_flows() -> anyhow::Result<()> {
    println!("***** Step 1: Initialization *****");
    let context = IntegrationTestContext::new().await?;

    // First test
    // user1, user2 and user3 deposit 10 NEAR each. We check whether the staking contract
    // Check initial deposits
    println!("**** Step 2: User deposit test ****");
    println!("Checking initial user deposits");

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;
    assert_eq!(user1_staked_amount, U128(0));
    assert_eq!(user2_staked_amount, U128(0));
    assert_eq!(user3_staked_amount, U128(0));

    println!("Successfully checked initial user deposits");

    let validator_info = context
        .get_validator_info(context.stake_pool_contract.id().clone())
        .await?;
    println!(
        "validator account details before deposits are {:?}",
        validator_info
    );

    println!("**** Simulating user deposits ****");
    println!("User 1 depositing");
    context.deposit(&context.user1).await?;
    println!("User 1 successfully deposited");

    println!("User 2 depositing");
    context.deposit(&context.user2).await?;
    println!("User 2 successfully deposited");

    println!("User 3 depositing");
    context.deposit(&context.user3).await?;
    println!("User 3 successfully deposited");

    println!("Checking user deposits after users have deposited");
    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(10)));
    assert_eq!(user2_staked_amount, U128(ntoy(10)));
    assert_eq!(user3_staked_amount, U128(ntoy(10)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(10)));
    assert_eq!(user2_token_balance, U128(ntoy(10)));
    assert_eq!(user3_token_balance, U128(ntoy(10)));

    let nearx_price = context.get_nearx_price().await?;
    println!("nearx_price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(ntoy(1)));

    let total_staked_amount = context.get_total_staked_amount().await?;
    println!("total_staked_amount is {:?}", total_staked_amount);
    assert_eq!(total_staked_amount, U128(ntoy(30)));

    let stake_pool_staked_amount = context.get_stake_pool_total_staked_amount().await?;
    println!("stake_pool_staked_amount is {:?}", stake_pool_staked_amount);
    assert_eq!(stake_pool_staked_amount, U128(ntoy(30)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(30)));

    // Second test
    // Test token transfers
    println!("**** Step 3: Token transferring ****");

    println!("Successfully checked initial user deposits");

    println!("User 1 transfers 5 tokens to User 2");
    context
        .ft_transfer(&context.user1, &context.user2, ntoy(5).to_string())
        .await?;
    println!("User 2 transfers 3 tokens to User 3");
    context
        .ft_transfer(&context.user2, &context.user3, ntoy(3).to_string())
        .await?;
    println!("User 3 transfers 1 token to User 1");
    context
        .ft_transfer(&context.user3, &context.user1, ntoy(1).to_string())
        .await?;

    println!("Checking user deposits after users have deposited");
    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(6)));
    assert_eq!(user2_staked_amount, U128(ntoy(12)));
    assert_eq!(user3_staked_amount, U128(ntoy(12)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(6)));
    assert_eq!(user2_token_balance, U128(ntoy(12)));
    assert_eq!(user3_token_balance, U128(ntoy(12)));

    let nearx_price = context.get_nearx_price().await?;
    assert_eq!(nearx_price, U128(ntoy(1)));

    let total_staked_amount = context.get_total_staked_amount().await?;
    assert_eq!(total_staked_amount, U128(ntoy(30)));

    let stake_pool_staked_amount = context.get_stake_pool_total_staked_amount().await?;
    assert_eq!(stake_pool_staked_amount, U128(ntoy(30)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(30)));

    println!("**** Step 4: Auto compounding ****");

    println!("Fast forward 61400 blocks");
    // context.worker.fast_forward(61400).await?;

    println!("Auto compounding stake pool");

    // Adding rewards
    context.add_stake_pool_rewards(U128(ntoy(30))).await?;
    let stake_pool_staked_amount = context.get_stake_pool_total_staked_amount().await?;
    println!("stake pool staked amount is {:?}", stake_pool_staked_amount);
    assert_eq!(stake_pool_staked_amount, U128(ntoy(60)));

    // restake_staking_pool(&worker, &stake_pool_contract).await?;
    let nearx_state = context.get_nearx_state().await?;
    println!("nearx_state before auto compounding is {:?}", nearx_state);

    println!("Auto compounding nearx pool");
    context
        .auto_compound_rewards(context.stake_pool_contract.id())
        .await?;
    //
    let nearx_price = context.get_nearx_price().await?;
    // println!("nearx_price is {:?}", nearx_price);
    assert_eq!(nearx_price, U128(ntoy(2)));

    let total_tokens_minted = context.get_total_tokens_supply().await?;
    assert_eq!(total_tokens_minted, U128(ntoy(30)));

    let user1_staked_amount = context.get_user_deposit(context.user1.id().clone()).await?;
    let user2_staked_amount = context.get_user_deposit(context.user2.id().clone()).await?;
    let user3_staked_amount = context.get_user_deposit(context.user3.id().clone()).await?;

    assert_eq!(user1_staked_amount, U128(ntoy(12)));
    assert_eq!(user2_staked_amount, U128(ntoy(24)));
    assert_eq!(user3_staked_amount, U128(ntoy(24)));

    let user1_token_balance = context
        .get_user_token_balance(context.user1.id().clone())
        .await?;
    let user2_token_balance = context
        .get_user_token_balance(context.user2.id().clone())
        .await?;
    let user3_token_balance = context
        .get_user_token_balance(context.user3.id().clone())
        .await?;

    assert_eq!(user1_token_balance, U128(ntoy(6)));
    assert_eq!(user2_token_balance, U128(ntoy(12)));
    assert_eq!(user3_token_balance, U128(ntoy(12)));

    let validator = context
        .get_validator_info(context.stake_pool_contract.id().clone())
        .await?;
    println!("validator is {:?}", validator);
    assert_eq!(
        validator,
        ValidatorInfoResponse {
            account_id: validator.account_id.clone(),
            staked: U128(ntoy(60)),
            last_asked_rewards_epoch_height: context.get_current_epoch().await?,
            lock: false
        }
    );

    let nearx_state = context.get_nearx_state().await?;
    assert_eq!(nearx_state.total_staked, U128(ntoy(60)));
    assert_eq!(nearx_state.accumulated_staked_rewards, U128(ntoy(30)));
    assert_eq!(nearx_state.total_stake_shares, U128(ntoy(30)));

    println!("nearx_state after auto compounding is {:?}", nearx_state);

    Ok(())
}
