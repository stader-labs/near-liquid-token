mod helpers;

use crate::helpers::abs_diff_eq;
use helpers::ntoy;
use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::storage_management::StorageManagement;
use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::testing_env_with_promise_results;
use near_sdk::{
    testing_env, AccountId, FunctionError, Gas, MockedBlockchain, PromiseOrValue, PromiseResult,
    PublicKey, RuntimeFeesConfig, VMConfig, VMContext,
};
use near_x::constants::NUM_EPOCHS_TO_UNLOCK;
use near_x::contract::{NearxPool, OperationControls};
use near_x::state::{Account, AccountResponse, AccountUpdateRequest, ContractStateUpdateRequest, Fraction, HumanReadableAccount, OperationsControlUpdateRequest, ValidatorInfo, ValidatorInfoResponse, ValidatorUpdateRequest};
use std::collections::HashMap;
use std::{convert::TryFrom, str::FromStr};

pub fn owner_account() -> AccountId {
    AccountId::from_str("owner_account").unwrap()
}

pub fn public_key(byte_val: u8) -> PublicKey {
    let mut pk = vec![byte_val; 33];
    pk[0] = 0;
    PublicKey::try_from(pk).unwrap()
}

pub fn system_account() -> AccountId {
    AccountId::from_str("system").unwrap()
}

pub fn to_nanos(num_days: u64) -> u64 {
    num_days * 86_400_000_000_000
}

pub fn to_ts(num_days: u64) -> u64 {
    // 2018-08-01 UTC in nanoseconds
    1_533_081_600_000_000_000 + to_nanos(num_days)
}

pub fn operator_account() -> AccountId {
    AccountId::from_str("operator_account").unwrap()
}

pub fn contract_account() -> AccountId {
    AccountId::from_str("nearx-pool").unwrap()
}

pub fn treasury_account() -> AccountId {
    AccountId::from_str("treasury_account").unwrap()
}

pub fn check_equal_vec<S: PartialEq>(v1: Vec<S>, v2: Vec<S>) -> bool {
    v1.len() == v2.len() && v1.iter().all(|x| v2.contains(x)) && v2.iter().all(|x| v1.contains(x))
}

pub fn default_pubkey() -> PublicKey {
    PublicKey::try_from(vec![0; 33]).unwrap()
}

pub fn get_context(
    predecessor_account_id: AccountId,
    account_balance: u128,
    account_locked_balance: u128,
    block_timestamp: u64,
) -> VMContext {
    VMContext {
        current_account_id: contract_account(),
        signer_account_id: predecessor_account_id.clone(),
        signer_account_pk: default_pubkey(),
        predecessor_account_id,
        input: vec![],
        block_index: 1,
        block_timestamp,
        epoch_height: 1,
        account_balance,
        account_locked_balance,
        storage_usage: 10u64.pow(6),
        attached_deposit: 0,
        prepaid_gas: Gas(10u64.pow(15)), //10u64.pow(15),
        random_seed: [0; 32],
        view_config: None,
        output_data_receivers: vec![],
    }
}

fn get_validator(contract: &NearxPool, validator: AccountId) -> ValidatorInfo {
    contract.validator_info_map.get(&validator).unwrap()
}

fn update_validator(
    contract: &mut NearxPool,
    validator: AccountId,
    validator_info: &ValidatorInfo,
) {
    contract
        .validator_info_map
        .insert(&validator, validator_info)
        .unwrap();
}

fn get_account(contract: &NearxPool, account_id: AccountId) -> Account {
    contract.accounts.get(&account_id).unwrap()
}

fn get_account_option(contract: &NearxPool, account_id: AccountId) -> Option<Account> {
    contract.accounts.get(&account_id)
}

fn update_account(contract: &mut NearxPool, account_id: AccountId, account: &Account) {
    contract.accounts.insert(&account_id, account);
}

fn basic_context() -> VMContext {
    get_context(system_account(), ntoy(100), 0, to_ts(500))
}

fn new_contract(
    owner_account: AccountId,
    operator_account: AccountId,
    treasury_account: AccountId,
) -> NearxPool {
    NearxPool::new(owner_account, operator_account, treasury_account)
}

fn contract_setup(
    owner_account: AccountId,
    operator_account: AccountId,
    treasury_account: AccountId,
) -> (VMContext, NearxPool) {
    let context = basic_context();
    testing_env!(context.clone());
    let contract = new_contract(owner_account, operator_account, treasury_account);
    (context, contract)
}

#[test]
#[should_panic]
fn test_non_owner_calling_update_operations_control() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = operator_account();
    context.signer_account_id = operator_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.update_operations_control(OperationsControlUpdateRequest {
        stake_paused: None,
        unstake_paused: None,
        withdraw_paused: None,
        staking_epoch_paused: None,
        unstaking_epoch_paused: None,
        withdraw_epoch_paused: None,
        autocompounding_epoch_paused: None,
        sync_validator_balance_paused: None,
    });
}

#[test]
fn test_update_operations_control_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.update_operations_control(OperationsControlUpdateRequest {
        stake_paused: Some(true),
        unstake_paused: Some(true),
        withdraw_paused: None,
        staking_epoch_paused: Some(true),
        unstaking_epoch_paused: Some(true),
        withdraw_epoch_paused: Some(true),
        autocompounding_epoch_paused: None,
        sync_validator_balance_paused: Some(true),
    });

    let operations_control = contract.get_operations_control();
    assert_eq!(
        operations_control,
        OperationControls {
            stake_paused: true,
            unstaked_paused: true,
            withdraw_paused: false,
            staking_epoch_paused: true,
            unstaking_epoch_paused: true,
            withdraw_epoch_paused: true,
            autocompounding_epoch_paused: false,
            sync_validator_balance_paused: true
        }
    );
}

#[test]
#[should_panic]
fn test_update_rewards_buffer_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = AccountId::from_str("abc").unwrap();
    context.attached_deposit = 1;
    testing_env!(context); // this updates the context

    contract.update_rewards_buffer();
}

#[test]
fn test_update_rewards_buffer_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = operator_account();
    context.attached_deposit = ntoy(10);
    testing_env!(context); // this updates the context

    contract.total_staked = ntoy(100);
    contract.rewards_buffer = 0;

    contract.update_rewards_buffer();

    assert_eq!(contract.total_staked, ntoy(110));
    assert_eq!(contract.rewards_buffer, ntoy(10));
    assert_eq!(contract.accumulated_rewards_buffer, ntoy(10));
}

#[test]
#[should_panic]
fn test_update_validator_fail() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
       Non operator adding stake pool
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.update_validator(stake_public_key_1.clone(), 10);
}

#[test]
#[should_panic]
fn test_update_validator_invalid_weight_fail() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
       Non operator adding stake pool
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.update_validator(stake_public_key_1.clone(), 0);
}

#[test]
fn test_update_validator_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
       owner adding stake pool
    */
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();
    let stake_public_key_3 = AccountId::from_str("stake_public_key_3").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);
    contract.add_validator(stake_public_key_2.clone(), 30);
    contract.add_validator(stake_public_key_3.clone(), 20);

    let mut val1 = get_validator(&contract, stake_public_key_1.clone());
    val1.weight = 10;
    val1.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_1.clone(), &val1);

    let mut val2 = get_validator(&contract, stake_public_key_2.clone());
    val2.weight = 10;
    val2.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_2.clone(), &val2);

    let mut val3 = get_validator(&contract, stake_public_key_3.clone());
    val3.weight = 10;
    val3.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_3.clone(), &val3);

    contract.total_validator_weight = 30;

    contract.update_validator(stake_public_key_1.clone(), 20);

    let mut val1 = get_validator(&contract, stake_public_key_1.clone());
    assert_eq!(val1.weight, 20);
    assert_eq!(val1.staked, ntoy(100));
    assert_eq!(contract.total_validator_weight, 40);
}

#[test]
#[should_panic]
fn test_add_validator_fail() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
       Non operator adding stake pool
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);
}

#[test]
#[should_panic]
fn test_remove_validator_fail() {
    let (_context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
       Non operator removing stake pool
    */
    contract.remove_validator(AccountId::from_str("test_validator").unwrap());
}

#[test]
#[should_panic]
fn test_remove_validator_validator_in_unbonding_period() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    context.epoch_height = 11;
    testing_env!(context); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);

    let mut val1 = get_validator(&contract, stake_public_key_1.clone());
    val1.weight = 0;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(0);
    val1.unstake_start_epoch = 10;
    update_validator(&mut contract, stake_public_key_1, &val1);

    contract.remove_validator(AccountId::from_str("stake_public_key_1").unwrap());
}

#[test]
#[should_panic]
fn test_remove_validator_validator_non_zero_weight() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    context.epoch_height = 11;
    testing_env!(context); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);

    let mut val1 = get_validator(&contract, stake_public_key_1.clone());
    val1.weight = 10;
    val1.staked = ntoy(10);
    val1.unstaked_amount = ntoy(0);
    val1.unstake_start_epoch = 10;
    update_validator(&mut contract, stake_public_key_1, &val1);

    contract.remove_validator(AccountId::from_str("stake_public_key_1").unwrap());
}

#[test]
#[should_panic]
fn test_remove_validator_validator_non_zero_staked_unstaked_amount() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    context.epoch_height = 11;
    testing_env!(context); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);

    let mut val1 = get_validator(&contract, stake_public_key_1.clone());
    val1.weight = 0;
    val1.staked = ntoy(10);
    val1.unstaked_amount = ntoy(0);
    val1.unstake_start_epoch = 10;
    update_validator(&mut contract, stake_public_key_1, &val1);

    contract.remove_validator(AccountId::from_str("stake_public_key_1").unwrap());
}

#[test]
fn test_remove_validator_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
       seed staking pools
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();
    let stake_public_key_3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.epoch_height = 40;
    context.attached_deposit = 1;
    testing_env!(context); // this updates the context

    contract.add_validator(stake_public_key_1.clone(), 10);
    contract.add_validator(stake_public_key_2.clone(), 10);
    contract.add_validator(stake_public_key_3.clone(), 10);

    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 3);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(0),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(0),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(0),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            }
        ]
    ));
    assert_eq!(contract.total_validator_weight, 30);

    /*
       Remove a stake pool
    */
    let mut val1 = get_validator(&contract, stake_public_key_1.clone());
    val1.weight = 0;
    contract.total_validator_weight = 20;
    val1.unstake_start_epoch = 10;
    update_validator(&mut contract, stake_public_key_1.clone(), &val1);

    contract.remove_validator(stake_public_key_1.clone());
    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 2);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(0),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(0),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            }
        ]
    ));
    assert_eq!(contract.total_validator_weight, 20);

    /*
        Remove another stake pool
    */
    let mut val2 = get_validator(&contract, stake_public_key_2.clone());
    val2.weight = 0;
    val1.unstake_start_epoch = 10;
    contract.total_validator_weight = 10;
    update_validator(&mut contract, stake_public_key_2.clone(), &val2);

    contract.remove_validator(stake_public_key_2.clone());
    let stake_pools = contract.get_validators();

    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_3.clone(),
            staked: U128(0),
            unstaked: U128(0),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
        }]
    ));
    assert_eq!(contract.total_validator_weight, 10);

    /*
        Remove last stake pool
    */
    let mut val3 = get_validator(&contract, stake_public_key_3.clone());
    val3.weight = 0;
    val1.unstake_start_epoch = 10;
    contract.total_validator_weight = 0;
    update_validator(&mut contract, stake_public_key_3.clone(), &val3);

    contract.remove_validator(stake_public_key_3);
    let stake_pools = contract.get_validators();

    assert!(stake_pools.is_empty());
    assert_eq!(contract.total_validator_weight, 0);
}

#[test]
fn test_add_validator_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
       initial staking pools should be empty
    */
    let stake_pools = contract.get_validators();
    assert!(
        stake_pools.is_empty(),
        "Stake pools should initially be empty!"
    );

    /*
       add a stake pool
    */
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert_eq!(contract.total_validator_weight, 10);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
        }]
    ));

    /*
       add another stake pool
    */
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();

    contract.add_validator(stake_public_key_2.clone(), 10);
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 2);
    assert_eq!(contract.total_validator_weight, 20);
    assert!(check_equal_vec(
        stake_pools,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1,
                staked: U128(0),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2,
                staked: U128(0),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            }
        ]
    ));
}

#[test]
fn test_get_validator_to_unstake() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
        Get validator in empty validator set
    */
    let validator = contract.get_validator_to_unstake();
    assert!(validator.is_none());

    /*
        seed staking pools
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();
    let stake_public_key_3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    context.epoch_height = 100;
    testing_env!(context); // this updates the context

    contract.add_validator(stake_public_key_1.clone(), 10);
    contract.add_validator(stake_public_key_2.clone(), 10);
    contract.add_validator(stake_public_key_3.clone(), 10);

    let mut validator_1 = get_validator(&contract, stake_public_key_1.clone());
    let mut validator_2 = get_validator(&contract, stake_public_key_2.clone());
    let mut validator_3 = get_validator(&contract, stake_public_key_3.clone());

    validator_1.staked = 100;
    validator_2.staked = 200;
    validator_3.staked = 300;

    update_validator(&mut contract, stake_public_key_1.clone(), &validator_1);
    update_validator(&mut contract, stake_public_key_2.clone(), &validator_2);
    update_validator(&mut contract, stake_public_key_3.clone(), &validator_3);

    let validators = contract.get_validators();

    assert_eq!(validators.len(), 3);
    assert!(check_equal_vec(
        validators,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(100),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(200),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(300),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            }
        ]
    ));

    contract.total_staked = 600;
    contract.total_stake_shares = 600;

    /*
       Get stake pool to stake into
    */
    let validator = contract.get_validator_to_unstake();
    assert!(validator.is_some());
    assert_eq!(validator.unwrap().account_id, stake_public_key_3);

    // Validators with non_equal weights

    let mut validator_1 = get_validator(&contract, stake_public_key_1.clone());
    let mut validator_2 = get_validator(&contract, stake_public_key_2.clone());
    let mut validator_3 = get_validator(&contract, stake_public_key_3.clone());

    validator_1.staked = ntoy(100);
    validator_1.weight = 10;
    validator_2.staked = ntoy(400);
    validator_2.weight = 20;
    validator_3.staked = ntoy(300);
    validator_3.weight = 30;
    contract.total_validator_weight = 60;

    update_validator(&mut contract, stake_public_key_1.clone(), &validator_1);
    update_validator(&mut contract, stake_public_key_2.clone(), &validator_2);
    update_validator(&mut contract, stake_public_key_3.clone(), &validator_3);

    let validators = contract.get_validators();

    assert_eq!(validators.len(), 3);
    assert!(check_equal_vec(
        validators,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(ntoy(100)),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(ntoy(400)),
                unstaked: U128(0),
                weight: 20,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(ntoy(300)),
                unstaked: U128(0),
                weight: 30,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            }
        ]
    ));

    contract.total_staked = ntoy(800);
    contract.total_stake_shares = ntoy(800);

    /*
       Get stake pool to stake into
    */
    let validator = contract.get_validator_to_unstake();
    assert!(validator.is_some());
    assert_eq!(validator.unwrap().account_id, stake_public_key_2);
}

#[test]
fn test_get_validator_to_stake() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
        Get stake pool in empty stake pool set
    */
    let stake_pool = contract.get_validator_to_stake(0);
    assert!(stake_pool.0.is_none());

    /*
       seed staking pools
    */
    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();
    let stake_public_key_3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context); // this updates the context

    contract.add_validator(stake_public_key_1.clone(), 10);
    contract.add_validator(stake_public_key_2.clone(), 10);
    contract.add_validator(stake_public_key_3.clone(), 10);

    let mut validator_1 = get_validator(&contract, stake_public_key_1.clone());
    let mut validator_2 = get_validator(&contract, stake_public_key_2.clone());
    let mut validator_3 = get_validator(&contract, stake_public_key_3.clone());

    validator_1.staked = 100;
    validator_2.staked = 200;
    validator_3.staked = 300;

    update_validator(&mut contract, stake_public_key_1.clone(), &validator_1);
    update_validator(&mut contract, stake_public_key_2.clone(), &validator_2);
    update_validator(&mut contract, stake_public_key_3.clone(), &validator_3);

    let validators = contract.get_validators();

    assert_eq!(validators.len(), 3);
    assert!(check_equal_vec(
        validators,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(100),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(200),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(300),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            }
        ]
    ));

    contract.total_staked = 700;
    contract.total_stake_shares = 700;

    /*
       Get stake pool to stake into
    */
    let validator = contract.get_validator_to_stake(100);
    assert!(validator.0.is_some());
    assert_eq!(validator.0.unwrap().account_id, stake_public_key_1);
    assert_eq!(validator.1, 100);

    // Validators with non_equal weights

    let mut validator_1 = get_validator(&contract, stake_public_key_1.clone());
    let mut validator_2 = get_validator(&contract, stake_public_key_2.clone());
    let mut validator_3 = get_validator(&contract, stake_public_key_3.clone());

    validator_1.staked = ntoy(100);
    validator_1.weight = 10;
    validator_2.staked = ntoy(100);
    validator_2.weight = 20;
    validator_3.staked = ntoy(400);
    validator_3.weight = 30;
    contract.total_validator_weight = 60;

    update_validator(&mut contract, stake_public_key_1.clone(), &validator_1);
    update_validator(&mut contract, stake_public_key_2.clone(), &validator_2);
    update_validator(&mut contract, stake_public_key_3.clone(), &validator_3);

    let validators = contract.get_validators();

    assert_eq!(validators.len(), 3);
    assert!(check_equal_vec(
        validators,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(ntoy(100)),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(ntoy(100)),
                unstaked: U128(0),
                weight: 20,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(ntoy(400)),
                unstaked: U128(0),
                weight: 30,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            }
        ]
    ));

    contract.total_staked = ntoy(700);
    contract.total_stake_shares = ntoy(700);

    /*
       Get stake pool to stake into
    */
    let validator = contract.get_validator_to_stake(ntoy(100));
    assert!(validator.0.is_some());
    assert_eq!(validator.0.unwrap().account_id, stake_public_key_2);
    assert_eq!(validator.1, ntoy(100));

    let validator = contract.get_validator_to_stake(ntoy(101));
    assert!(validator.0.is_some());
    assert_eq!(validator.0.unwrap().account_id, stake_public_key_2);
    assert_eq!(validator.1, ntoy(101));

    // Validators with equal weights and equal amounts

    let mut validator_1 = get_validator(&contract, stake_public_key_1.clone());
    let mut validator_2 = get_validator(&contract, stake_public_key_2.clone());
    let mut validator_3 = get_validator(&contract, stake_public_key_3.clone());

    validator_1.staked = ntoy(100);
    validator_1.weight = 10;
    validator_2.staked = ntoy(100);
    validator_2.weight = 10;
    validator_3.staked = ntoy(100);
    validator_3.weight = 10;
    contract.total_validator_weight = 30;

    update_validator(&mut contract, stake_public_key_1.clone(), &validator_1);
    update_validator(&mut contract, stake_public_key_2.clone(), &validator_2);
    update_validator(&mut contract, stake_public_key_3.clone(), &validator_3);

    let validators = contract.get_validators();

    assert_eq!(validators.len(), 3);
    assert!(check_equal_vec(
        validators,
        vec![
            ValidatorInfoResponse {
                account_id: stake_public_key_1.clone(),
                staked: U128(ntoy(100)),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_2.clone(),
                staked: U128(ntoy(100)),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            },
            ValidatorInfoResponse {
                account_id: stake_public_key_3.clone(),
                staked: U128(ntoy(100)),
                unstaked: U128(0),
                weight: 10,
                last_asked_rewards_epoch_height: U64(0),
                last_unstake_start_epoch: U64(0),
            }
        ]
    ));

    contract.total_staked = ntoy(700);
    contract.total_stake_shares = ntoy(700);

    /*
       Get stake pool to stake into
    */
    let validator = contract.get_validator_to_stake(ntoy(100));
    assert!(validator.0.is_some());
    assert_eq!(validator.0.unwrap().account_id, stake_public_key_1);
    assert_eq!(validator.1, ntoy(100));

    let validator = contract.get_validator_to_stake(ntoy(101));
    assert!(validator.0.is_some());
    assert_eq!(validator.0.unwrap().account_id, stake_public_key_1);
    assert_eq!(validator.1, ntoy(101));
}

#[test]
#[should_panic]
fn test_add_min_storage_reserve_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = operator_account();
    context.signer_account_id = operator_account();
    context.attached_deposit = 1;
    testing_env!(context.clone()); // this updates the context

    contract.add_min_storage_reserve();
}

#[test]
fn test_add_min_storage_reserve_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = ntoy(50);
    testing_env!(context.clone()); // this updates the context

    contract.min_storage_reserve = ntoy(10);

    contract.add_min_storage_reserve();

    assert_eq!(contract.min_storage_reserve, ntoy(60));
}

#[test]
#[should_panic]
fn test_set_reward_fee_fail() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.epoch_height = 5;
    context.attached_deposit = 1;
    testing_env!(context.clone()); // this updates the context

    /*
       Set reward fee more than 10%
    */
    contract.set_reward_fee(15, 100);
}

#[test]
fn test_set_reward_fee_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.epoch_height = 5;
    context.attached_deposit = 1;
    testing_env!(context.clone()); // this updates the context

    /*
       Set reward fee to 9%
    */
    contract.set_reward_fee(9, 100);

    assert!(contract.temp_reward_fee.is_some());
    assert_eq!(contract.temp_reward_fee.unwrap().numerator, 9);
    assert_eq!(contract.temp_reward_fee.unwrap().denominator, 100);

    /*
        Set reward fee to 10%
    */
    contract.set_reward_fee(10, 100);

    assert!(contract.temp_reward_fee.is_some());
    assert_eq!(contract.temp_reward_fee.unwrap().numerator, 10);
    assert_eq!(contract.temp_reward_fee.unwrap().denominator, 100);
}

#[test]
#[should_panic]
fn test_commit_future_reward_fee_not_set() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.epoch_height = 10;
    context.attached_deposit = 1;
    testing_env!(context.clone()); // this updates the context

    contract.commit_reward_fee();
}

#[test]
#[should_panic]
fn test_commit_future_reward_fee_in_wait_time() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.epoch_height = 10;
    context.attached_deposit = 1;
    testing_env!(context.clone()); // this updates the context

    contract.temp_reward_fee = Some(Fraction::new(5, 100));
    contract.last_reward_fee_set_epoch = 8;

    contract.commit_reward_fee();
}

#[test]
fn test_commit_future_reward_fee_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.epoch_height = 13;
    context.attached_deposit = 1;
    testing_env!(context.clone()); // this updates the context

    contract.rewards_fee = Fraction::new(9, 100);
    contract.temp_reward_fee = Some(Fraction::new(8, 100));
    contract.last_reward_fee_set_epoch = 8;

    contract.commit_reward_fee();

    assert_eq!(contract.rewards_fee.numerator, 8);
    assert_eq!(contract.rewards_fee.denominator, 100);
    assert!(contract.temp_reward_fee.is_none());
}

#[test]
#[should_panic]
fn test_autocompound_rewards_contract_busy() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.autocompounding_epoch(AccountId::from_str("random_validator").unwrap());
}

#[test]
#[should_panic]
fn test_autocompound_rewards_invalid_validator() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.autocompounding_epoch(AccountId::from_str("invalid_validator").unwrap());
}

#[test]
fn test_autocompound_rewards_stake_pool_with_no_stake() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    /*
       Add stake pool
    */
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.epoch_height = 5;
    context.attached_deposit = 1;
    testing_env!(context.clone()); // this updates the context

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
        }]
    ));

    // Redeeming rewards with no stake amount with validators
    contract.autocompounding_epoch(stake_public_key_1.clone());

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());

    /*
       Redeeming rewards in the same epoch
    */

    validator1.last_redeemed_rewards_epoch = context.epoch_height;
    validator1.staked = ntoy(100);

    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);
    contract.autocompounding_epoch(stake_public_key_1.clone());

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());

    /*
       Successful case
    */
    context.epoch_height = 100;
    testing_env!(context.clone());
    validator1.last_redeemed_rewards_epoch = 4;
    validator1.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);

    contract.autocompounding_epoch(stake_public_key_1.clone());
}

#[test]
fn test_on_get_sp_staked_balance_for_rewards() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);
    let stake_pools = contract.get_validators();
    assert_eq!(stake_pools.len(), 1);
    assert!(check_equal_vec(
        stake_pools,
        vec![ValidatorInfoResponse {
            account_id: stake_public_key_1.clone(),
            staked: U128(0),
            unstaked: U128(0),
            weight: 10,
            last_asked_rewards_epoch_height: U64(0),
            last_unstake_start_epoch: U64(0),
        }]
    ));

    context.predecessor_account_id = contract_account();
    context.epoch_height = 100;
    testing_env!(context.clone());

    let mut validator1 = get_validator(&contract, stake_public_key_1.clone());
    validator1.staked = ntoy(100);
    update_validator(&mut contract, stake_public_key_1.clone(), &validator1);

    contract.rewards_fee = Fraction::new(10, 100);
    contract.total_staked = ntoy(100);
    contract.total_stake_shares = ntoy(100);

    let _res = contract.on_get_sp_staked_balance_for_rewards(validator1, U128::from(ntoy(150)));

    let validator1 = get_validator(&contract, stake_public_key_1.clone());
    assert_eq!(validator1.staked, ntoy(150));
    assert_eq!(validator1.last_redeemed_rewards_epoch, context.epoch_height);
    assert_eq!(contract.total_staked, ntoy(150));
    assert_eq!(contract.total_stake_shares, 103333333333333333333333333);
    assert_eq!(contract.accumulated_staked_rewards, ntoy(50));

    let treasury_account = contract.get_account(treasury_account());
    assert!(abs_diff_eq(
        treasury_account.staked_balance.0,
        ntoy(5),
        ntoy(1)
    ));
}

#[test]
#[should_panic]
fn test_deposit_and_stake_fail_min_deposit() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.attached_deposit = 100;
    testing_env!(context);

    contract.min_deposit_amount = 200;

    contract.deposit_and_stake();
}

#[test]
#[should_panic]
fn test_deposit_and_stake_fail_zero_amount() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.attached_deposit = 0;
    testing_env!(context);

    contract.deposit_and_stake();
}

#[test]
fn test_deposit_and_stake_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();

    context.predecessor_account_id = user1.clone();
    testing_env!(context.clone());

    contract.min_deposit_amount = ntoy(1);
    contract.total_staked = ntoy(10);
    contract.total_stake_shares = ntoy(10);
    contract.user_amount_to_stake_in_epoch = ntoy(10);

    context.attached_deposit = 3000000000000000000000;
    testing_env!(context.clone());
    contract.storage_deposit(None, None);

    context.attached_deposit = ntoy(100);
    testing_env!(context.clone());
    contract.deposit_and_stake();

    let user1_account = contract.get_account(user1.clone());
    assert_eq!(user1_account.staked_balance, U128(ntoy(100)));

    assert_eq!(contract.total_staked, ntoy(110));
    assert_eq!(contract.total_stake_shares, ntoy(110));
    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(110));

    // Test when price > 1
    // price is 1.5
    contract.total_staked = ntoy(15);
    contract.total_stake_shares = ntoy(10);
    contract.user_amount_to_stake_in_epoch = ntoy(20);

    context.attached_deposit = ntoy(100);
    context.predecessor_account_id = user1.clone();
    testing_env!(context.clone());

    contract.deposit_and_stake();

    let user1_account = contract.get_account(user1.clone());

    assert_eq!(
        user1_account.staked_balance,
        U128(250000000000000000000000001)
    );

    assert_eq!(contract.total_staked, ntoy(115));
    assert_eq!(contract.total_stake_shares, 76666666666666666666666666);
    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(120));
}

#[test]
fn test_epoch_reconcilation() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 100;
    testing_env!(context);

    contract.last_reconcilation_epoch = 99;
    contract.user_amount_to_stake_in_epoch = ntoy(100);
    contract.user_amount_to_unstake_in_epoch = ntoy(150);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    contract.epoch_reconcilation();

    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(0));
    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(0));
    assert_eq!(contract.reconciled_epoch_unstake_amount, ntoy(50));
    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(0));
    assert_eq!(contract.last_reconcilation_epoch, 100);
}

#[test]
fn test_epoch_reconcilation_with_rewards_buffer() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 100;
    context.predecessor_account_id = operator_account();
    context.attached_deposit = ntoy(10);
    testing_env!(context.clone());

    contract.total_staked = ntoy(200);

    contract.update_rewards_buffer();

    assert_eq!(contract.total_staked, ntoy(210));
    assert_eq!(contract.rewards_buffer, ntoy(10));
    assert_eq!(contract.accumulated_rewards_buffer, ntoy(10));

    contract.last_reconcilation_epoch = 99;
    contract.user_amount_to_stake_in_epoch = ntoy(100);
    contract.user_amount_to_unstake_in_epoch = ntoy(150);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    contract.epoch_reconcilation();

    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(0));
    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(0));
    assert_eq!(contract.reconciled_epoch_unstake_amount, ntoy(40));
    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(0));
    assert_eq!(contract.last_reconcilation_epoch, 100);
    assert_eq!(contract.rewards_buffer, ntoy(0));
    assert_eq!(contract.accumulated_rewards_buffer, ntoy(10));

    contract.user_amount_to_unstake_in_epoch = ntoy(20);
    contract.user_amount_to_stake_in_epoch = ntoy(0);
    contract.reconciled_epoch_stake_amount = ntoy(0);
    contract.reconciled_epoch_unstake_amount = ntoy(0);

    context.epoch_height = 101;
    context.predecessor_account_id = operator_account();
    context.attached_deposit = ntoy(30);
    testing_env!(context);

    contract.update_rewards_buffer();

    assert_eq!(contract.rewards_buffer, ntoy(30));
    assert_eq!(contract.accumulated_rewards_buffer, ntoy(40));

    contract.epoch_reconcilation();

    assert_eq!(contract.rewards_buffer, ntoy(10));
    assert_eq!(contract.accumulated_rewards_buffer, ntoy(40));
    assert_eq!(contract.reconciled_epoch_unstake_amount, ntoy(0));
    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(0));
    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(0));
    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(0));
}

#[test]
#[should_panic]
fn test_epoch_stake_paused() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.operations_control.staking_epoch_paused = true;

    contract.staking_epoch();
}

#[test]
#[should_panic]
fn test_epoch_unstake_paused() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.operations_control.unstaking_epoch_paused = true;

    contract.unstaking_epoch();
}

#[test]
#[should_panic]
fn test_epoch_withdraw_paused() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.operations_control.withdraw_epoch_paused = true;

    contract.withdraw_epoch(AccountId::from_str("random_validator").unwrap());
}

#[test]
#[should_panic]
fn test_epoch_autocompounding_paused() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.operations_control.autocompounding_epoch_paused = true;

    contract.autocompounding_epoch(AccountId::from_str("random_validator").unwrap());
}

#[test]
#[should_panic]
fn test_stake_paused() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.operations_control.stake_paused = true;

    contract.deposit_and_stake();
}

#[test]
#[should_panic]
fn test_unstake_paused() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.operations_control.unstaked_paused = true;

    contract.unstake(U128(100));
}

#[test]
#[should_panic]
fn test_withdraw_paused() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.operations_control.withdraw_paused = true;

    contract.withdraw(U128(100));
}

#[test]
#[should_panic]
fn test_epoch_stake_no_validator() {
    let (mut _context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.last_reconcilation_epoch = 99;
    contract.total_staked = ntoy(350);
    contract.user_amount_to_stake_in_epoch = ntoy(150);
    contract.user_amount_to_unstake_in_epoch = ntoy(100);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    while contract.staking_epoch() {}

    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(0));
    assert_eq!(contract.last_reconcilation_epoch, 100);
}

#[test]
fn test_epoch_stake() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 100;
    context.attached_deposit = 1;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    testing_env!(context);

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    update_validator(&mut contract, validator2.clone(), &val2_info);

    contract.last_reconcilation_epoch = 99;
    contract.total_staked = ntoy(350);
    contract.user_amount_to_stake_in_epoch = ntoy(150);
    contract.user_amount_to_unstake_in_epoch = ntoy(100);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    while contract.staking_epoch() {}

    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(0));
    assert_eq!(contract.last_reconcilation_epoch, 100);
}

#[test]
fn test_on_validator_deposit_and_stake_failed() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone(), 10);

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.reconciled_epoch_stake_amount = ntoy(10);

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    contract.on_stake_pool_deposit_and_stake(validator1.clone(), ntoy(10));

    assert_eq!(contract.reconciled_epoch_stake_amount, ntoy(20));
}

#[test]
fn test_on_validator_unstake_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone(), 10);

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(90);
    val1_info.unstake_start_epoch = 10;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));

    contract.on_stake_pool_unstake(val1_info.account_id, ntoy(10));

    let mut val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.staked, ntoy(90));
    assert_eq!(val1_info.unstaked_amount, ntoy(10));
    assert_eq!(val1_info.unstake_start_epoch, 10);
}

#[test]
fn test_on_validator_unstake_fail() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone(), 10);

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(90);
    val1_info.unstake_start_epoch = 10;
    val1_info.last_unstake_start_epoch = 5;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.reconciled_epoch_unstake_amount = ntoy(10);

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    contract.on_stake_pool_unstake(val1_info.account_id, ntoy(10));

    let mut val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.staked, ntoy(100));
    assert_eq!(val1_info.unstaked_amount, ntoy(0));
    assert_eq!(val1_info.unstake_start_epoch, 5);

    assert_eq!(contract.reconciled_epoch_unstake_amount, ntoy(20));
}

#[test]
fn test_on_validator_deposit_and_stake_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone(), 10);

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));

    contract.on_stake_pool_deposit_and_stake(validator1.clone(), ntoy(10));

    let val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.staked, ntoy(110));
}

#[test]
fn test_get_unstake_release_epoch() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.epoch_height = 10;
    context.attached_deposit = 1;
    testing_env!(context.clone());

    // Enough amount available to unstake

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    update_validator(&mut contract, validator2.clone(), &val2_info);

    let mut val3_info = get_validator(&contract, validator3.clone());
    val3_info.staked = ntoy(300);
    update_validator(&mut contract, validator3.clone(), &val3_info);

    let wait_time = contract.get_unstake_release_epoch(ntoy(100));
    assert_eq!(wait_time, NUM_EPOCHS_TO_UNLOCK);

    context.epoch_height = 10;
    testing_env!(context.clone());

    // Not enough amount available to unstake
    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    val1_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    val2_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator2.clone(), &val2_info);

    let mut val3_info = get_validator(&contract, validator3.clone());
    val3_info.staked = ntoy(300);
    val3_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator3.clone(), &val3_info);

    let wait_time = contract.get_unstake_release_epoch(ntoy(300));
    assert_eq!(wait_time, 2 * NUM_EPOCHS_TO_UNLOCK);
}

#[test]
#[should_panic]
fn test_withdraw_fail_zero_deposit() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.withdraw(U128(0));
}

#[test]
#[should_panic]
fn test_withdraw_fail_not_enough_amount() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount += ntoy(100);
    update_account(&mut contract, user1.clone(), &user1_account);

    context.predecessor_account_id = user1;
    testing_env!(context.clone());

    contract.withdraw(U128(ntoy(200)));
}

#[test]
#[should_panic]
fn test_withdraw_fail_before_withdrawable_epoch() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount = ntoy(300);
    user1_account.withdrawable_epoch_height = 10;
    update_account(&mut contract, user1.clone(), &user1_account);

    context.epoch_height = 8;
    context.predecessor_account_id = user1;
    testing_env!(context.clone());

    contract.withdraw(U128(ntoy(200)));
}

#[test]
#[should_panic]
fn test_withdraw_fail_not_enough_storage_balance() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount = ntoy(200);
    user1_account.withdrawable_epoch_height = 10;
    update_account(&mut contract, user1.clone(), &user1_account);

    context.epoch_height = 12;
    context.predecessor_account_id = user1;
    context.account_balance = ntoy(230);
    contract.min_storage_reserve = ntoy(50);
    testing_env!(context.clone());

    contract.withdraw(U128(ntoy(200)));
}

#[test]
fn test_withdraw_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount += ntoy(300);
    user1_account.withdrawable_epoch_height = 10;
    update_account(&mut contract, user1.clone(), &user1_account);

    context.epoch_height = 12;
    context.predecessor_account_id = user1.clone();
    context.account_balance = ntoy(270);
    testing_env!(context.clone());

    contract.withdraw(U128(ntoy(200)));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.unstaked_amount, ntoy(100));
}

#[test]
fn test_withdraw_success_with_storage_balance_with_no_staked_amount() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount += ntoy(300);
    user1_account.withdrawable_epoch_height = 10;
    update_account(&mut contract, user1.clone(), &user1_account);

    contract.min_storage_reserve = ntoy(50);

    context.epoch_height = 12;
    context.predecessor_account_id = user1.clone();
    context.account_balance = ntoy(400);
    testing_env!(context.clone());

    contract.withdraw(U128(299999900000000000000000000));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.unstaked_amount, 0);
    assert_eq!(user1_account.stake_shares, 0);
}

#[test]
fn test_withdraw_success_with_storage_balance_with_staked_amount() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();

    let mut user1_account = Account::default();
    user1_account.unstaked_amount += ntoy(300);
    user1_account.stake_shares = ntoy(10);
    user1_account.withdrawable_epoch_height = 10;
    update_account(&mut contract, user1.clone(), &user1_account);

    contract.min_storage_reserve = ntoy(50);

    context.epoch_height = 12;
    context.predecessor_account_id = user1.clone();
    context.account_balance = ntoy(400);
    testing_env!(context.clone());

    contract.withdraw(U128(299999900000000000000000000));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.stake_shares, ntoy(10));
    assert_eq!(user1_account.unstaked_amount, ntoy(0));
}

#[test]
#[should_panic]
fn test_epoch_withdraw_fail_validator_in_unbonding() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 10;
    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.unstaked_amount = ntoy(100);
    val1_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.withdraw_epoch(validator1.clone());
}

#[test]
#[should_panic]
fn test_epoch_withdraw_fail_validator_paused() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 20;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.unstaked_amount = ntoy(100);
    val1_info.unstake_start_epoch = 9;
    val1_info.weight = 0;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.withdraw_epoch(validator1.clone());
}

#[test]
fn test_epoch_withdraw_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 4;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(50);
    val1_info.unstaked_amount = ntoy(100);
    val1_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.withdraw_epoch(validator1.clone());

    let val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.unstaked_amount, ntoy(0));
}

#[test]
fn test_on_stake_pool_withdraw_all_fail() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.epoch_height = 4;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    contract.add_validator(validator1.clone(), 10);

    context.predecessor_account_id = contract_account();
    testing_env!(context.clone());

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(50);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 9;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    contract.on_stake_pool_withdraw_all(val1_info, ntoy(100));

    let val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.unstaked_amount, ntoy(100));
}

#[test]
#[should_panic]
fn test_unstake_fail_zero_amount() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.unstake(U128(ntoy(0)));
}

#[test]
#[should_panic]
fn test_unstake_fail_greater_than_total_staked_amount() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.total_staked = ntoy(100);

    contract.unstake(U128(ntoy(200)));
}

#[test]
fn test_unstake_success_remaining_amount_less_than_storage_deposit() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();
    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    context.epoch_height = 10;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(500);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.total_staked = ntoy(500);
    contract.total_stake_shares = ntoy(500);
    contract.last_reconcilation_epoch = 8;
    contract.user_amount_to_unstake_in_epoch = ntoy(60);

    let mut user1_account = Account::default();
    user1_account.stake_shares = ntoy(50);
    user1_account.unstaked_amount = ntoy(0);
    update_account(&mut contract, user1.clone(), &user1_account);

    context.predecessor_account_id = user1.clone();
    testing_env!(context.clone());

    contract.unstake(U128(49999000000000000000000000));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.stake_shares, ntoy(0));
    assert_eq!(user1_account.unstaked_amount, ntoy(50));
    assert_eq!(user1_account.withdrawable_epoch_height, 14);

    assert_eq!(contract.total_staked, ntoy(450));
    assert_eq!(contract.total_stake_shares, ntoy(450));
    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(110));
}

#[test]
fn test_unstake_success_diff_epoch_than_reconcilation_epoch() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();
    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    context.epoch_height = 10;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(300);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.total_staked = ntoy(100);
    contract.total_stake_shares = ntoy(100);
    contract.last_reconcilation_epoch = 8;
    contract.user_amount_to_unstake_in_epoch = ntoy(60);

    let mut user1_account = Account::default();
    user1_account.stake_shares = ntoy(50);
    user1_account.unstaked_amount = ntoy(10);
    update_account(&mut contract, user1.clone(), &user1_account);

    context.predecessor_account_id = user1.clone();
    testing_env!(context.clone());

    contract.unstake(U128(ntoy(10)));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.stake_shares, ntoy(40));
    assert_eq!(user1_account.unstaked_amount, ntoy(20));
    assert_eq!(user1_account.withdrawable_epoch_height, 14);

    assert_eq!(contract.total_staked, ntoy(90));
    assert_eq!(contract.total_stake_shares, ntoy(90));
    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(70));
}

#[test]
fn test_unstake_success_same_epoch_as_reconcilation_epoch() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();
    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();

    context.epoch_height = 10;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(300);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.total_staked = ntoy(100);
    contract.total_stake_shares = ntoy(100);
    contract.last_reconcilation_epoch = 10;
    contract.user_amount_to_unstake_in_epoch = ntoy(60);

    let mut user1_account = Account::default();
    user1_account.stake_shares = ntoy(50);
    user1_account.unstaked_amount = ntoy(10);
    update_account(&mut contract, user1.clone(), &user1_account);

    context.predecessor_account_id = user1.clone();
    testing_env!(context.clone());

    contract.unstake(U128(ntoy(10)));

    let user1_account = get_account(&contract, user1.clone());
    assert_eq!(user1_account.stake_shares, ntoy(40));
    assert_eq!(user1_account.unstaked_amount, ntoy(20));
    assert_eq!(user1_account.withdrawable_epoch_height, 15);

    assert_eq!(contract.total_staked, ntoy(90));
    assert_eq!(contract.total_stake_shares, ntoy(90));
    assert_eq!(contract.user_amount_to_unstake_in_epoch, ntoy(70));
}

#[test]
fn test_epoch_unstake_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    val1_info.unstaked_amount = ntoy(0);
    val1_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    let mut val2_info = get_validator(&contract, validator2.clone());
    val2_info.staked = ntoy(200);
    val2_info.unstaked_amount = ntoy(0);
    val2_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator2.clone(), &val2_info);

    let mut val3_info = get_validator(&contract, validator3.clone());
    val3_info.staked = ntoy(300);
    val3_info.unstaked_amount = ntoy(0);
    val3_info.unstake_start_epoch = 3;
    update_validator(&mut contract, validator3.clone(), &val3_info);

    contract.last_reconcilation_epoch = 99;
    contract.user_amount_to_stake_in_epoch = ntoy(100);
    contract.user_amount_to_unstake_in_epoch = ntoy(150);
    contract.reconciled_epoch_stake_amount = ntoy(10);
    contract.reconciled_epoch_unstake_amount = ntoy(10);

    contract.unstaking_epoch();

    assert_eq!(contract.last_reconcilation_epoch, 100);
    let val3_info = get_validator(&contract, validator3.clone());
    assert_eq!(val3_info.staked, ntoy(250));
    assert_eq!(val3_info.unstake_start_epoch, 100);
    assert_eq!(val3_info.last_unstake_start_epoch, 3);
    assert_eq!(contract.reconciled_epoch_unstake_amount, 0);
}

#[test]
#[should_panic]
fn test_drain_unstake_fail_validator_not_paused() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    contract.drain_unstake(validator1);
}

#[test]
#[should_panic]
fn test_drain_unstake_fail_validator_pending_release() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.unstake_start_epoch = 99;
    val1_info.weight = 0;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.drain_unstake(validator1);
}

#[test]
#[should_panic]
fn test_drain_unstake_fail_validator_has_unstake() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.unstake_start_epoch = 33;
    val1_info.weight = 0;
    val1_info.unstaked_amount = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.drain_unstake(validator1);
}

#[test]
fn test_drain_unstake_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut val1_info = get_validator(&contract, validator1.clone());
    val1_info.staked = ntoy(100);
    val1_info.unstake_start_epoch = 33;
    val1_info.weight = 0;
    update_validator(&mut contract, validator1.clone(), &val1_info);

    contract.drain_unstake(validator1.clone());

    let val1_info = get_validator(&contract, validator1.clone());
    assert_eq!(val1_info.staked, ntoy(0));
    assert_eq!(val1_info.unstake_start_epoch, 100);
    assert_eq!(val1_info.last_unstake_start_epoch, 33);
}

#[test]
fn test_on_stake_pool_drain_unstake_promise_fail() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.last_unstake_start_epoch = 33;
    val1.unstake_start_epoch = 100;
    val1.staked = 0;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.on_stake_pool_drain_unstake(validator1.clone(), ntoy(100));

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 33;
    val1.staked = ntoy(100);
}

#[test]
fn test_on_stake_pool_drain_unstake_promise_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.last_unstake_start_epoch = 33;
    val1.unstake_start_epoch = 100;
    val1.staked = 0;
    val1.unstaked_amount = 0;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.on_stake_pool_drain_unstake(validator1.clone(), ntoy(100));

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 100;
    val1.staked = 0;
    val1.unstaked_amount = ntoy(100);
}

#[test]
#[should_panic]
fn test_drain_withdraw_fail_validator_not_paused() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    contract.drain_withdraw(validator1);
}

#[test]
#[should_panic]
fn test_drain_withdraw_fail_validator_has_non_zero_staked() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 100;
    val1.staked = ntoy(100);
    val1.unstaked_amount = 0;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    contract.drain_withdraw(validator1);
}

#[test]
#[should_panic]
fn test_drain_withdraw_fail_validator_pending_unstake() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 99;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(100);
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    contract.drain_withdraw(validator1);
}

#[test]
fn test_drain_withdraw_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 23;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(100);
    val1.weight = 0;
    update_validator(&mut contract, validator1.clone(), &val1);

    contract.drain_withdraw(validator1.clone());

    let mut val1 = get_validator(&contract, validator1.clone());
    assert_eq!(val1.unstaked_amount, 0);
}

#[test]
fn test_on_stake_pool_drain_withdraw_failure() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 88;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(0);
    update_validator(&mut contract, validator1.clone(), &val1);

    testing_env_with_promise_results(context.clone(), PromiseResult::Failed);

    contract.on_stake_pool_drain_withdraw(validator1.clone(), ntoy(100));

    let mut val1 = get_validator(&contract, validator1.clone());
    assert_eq!(val1.unstaked_amount, ntoy(100));
}

#[test]
fn test_on_stake_pool_drain_withdraw_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    contract.user_amount_to_stake_in_epoch = ntoy(100);

    let mut val1 = get_validator(&contract, validator1.clone());
    val1.unstake_start_epoch = 88;
    val1.staked = ntoy(0);
    val1.unstaked_amount = ntoy(0);
    update_validator(&mut contract, validator1.clone(), &val1);

    testing_env_with_promise_results(context.clone(), PromiseResult::Successful(Vec::default()));

    contract.on_stake_pool_drain_withdraw(validator1.clone(), ntoy(100));

    assert_eq!(contract.user_amount_to_stake_in_epoch, ntoy(200));
}

#[test]
#[should_panic]
fn test_sync_balance_from_validator_paused() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    contract.operations_control.sync_validator_balance_paused = true;

    contract.sync_balance_from_validator(AccountId::from_str("abc").unwrap());
}

#[test]
fn test_sync_balance_from_validator_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    contract.sync_balance_from_validator(validator1);
}

#[test]
#[should_panic]
fn test_on_stake_pool_get_account_total_balance_off() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut validator1_info = get_validator(&contract, validator1.clone());
    validator1_info.staked = 99000000000000000000000000;
    validator1_info.unstaked_amount = 9000000000000000000000000;
    update_validator(&mut contract, validator1.clone(), &validator1_info);

    contract.on_stake_pool_get_account(
        validator1.clone(),
        HumanReadableAccount {
            account_id: validator1.clone(),
            unstaked_balance: U128(9000000000000000000000008),
            staked_balance: U128(98999999999999999999999996),
            can_withdraw: false,
        },
    );

    let mut validator1_info = get_validator(&contract, validator1.clone());
    assert_eq!(validator1_info.staked, 98999999999999999999999996);
    assert_eq!(validator1_info.unstaked_amount, 9000000000000000000000004);
}

#[test]
fn test_on_stake_pool_get_account() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let validator1 = AccountId::from_str("stake_public_key_1").unwrap();
    let validator2 = AccountId::from_str("stake_public_key_2").unwrap();
    let validator3 = AccountId::from_str("stake_public_key_3").unwrap();

    context.epoch_height = 100;
    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.add_validator(validator1.clone(), 10);
    contract.add_validator(validator2.clone(), 10);
    contract.add_validator(validator3.clone(), 10);

    let mut validator1_info = get_validator(&contract, validator1.clone());
    validator1_info.staked = 99000000000000000000000000;
    validator1_info.unstaked_amount = 9000000000000000000000000;
    update_validator(&mut contract, validator1.clone(), &validator1_info);

    contract.on_stake_pool_get_account(
        validator1.clone(),
        HumanReadableAccount {
            account_id: validator1.clone(),
            unstaked_balance: U128(9000000000000000000000004),
            staked_balance: U128(98999999999999999999999996),
            can_withdraw: false,
        },
    );

    let mut validator1_info = get_validator(&contract, validator1.clone());
    assert_eq!(validator1_info.staked, 98999999999999999999999996);
    assert_eq!(validator1_info.unstaked_amount, 9000000000000000000000004);
}

#[test]
#[should_panic]
fn test_set_owner_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = AccountId::from_str("user").unwrap();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let new_owner = AccountId::from_str("new_owner").unwrap();

    contract.set_owner(new_owner);
}

#[test]
#[should_panic]
fn test_set_owner_same_as_operator() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let new_owner = AccountId::from_str("new_owner").unwrap();
    contract.operator_account_id = new_owner.clone();

    contract.set_owner(new_owner);
}

#[test]
#[should_panic]
fn test_set_owner_same_as_treasury() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let new_owner = AccountId::from_str("new_owner").unwrap();
    contract.treasury_account_id = new_owner.clone();

    contract.set_owner(new_owner);
}

#[test]
#[should_panic]
fn test_set_owner_same_as_current_contract() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_owner(context.current_account_id);
}

#[test]
fn test_set_owner() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let new_owner = AccountId::from_str("new_owner").unwrap();

    contract.set_owner(new_owner.clone());

    assert_eq!(contract.temp_owner, Some(new_owner));
}

#[test]
#[should_panic]
fn test_commit_owner_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let new_owner = AccountId::from_str("new_owner").unwrap();

    contract.temp_owner = Some(new_owner.clone());

    contract.commit_owner();
}

#[test]
fn test_commit_owner() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let new_owner = AccountId::from_str("new_owner").unwrap();

    context.predecessor_account_id = new_owner.clone();
    context.signer_account_id = new_owner.clone();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.temp_owner = Some(new_owner.clone());

    contract.commit_owner();

    assert_eq!(contract.owner_account_id, new_owner);
    assert_eq!(contract.temp_owner, None);
}

#[test]
fn set_commit_owner() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let new_owner = AccountId::from_str("new_owner").unwrap();

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let new_owner = AccountId::from_str("new_owner").unwrap();

    contract.set_owner(new_owner.clone());

    assert_eq!(contract.temp_owner, Some(new_owner.clone()));

    context.predecessor_account_id = new_owner.clone();
    context.signer_account_id = new_owner.clone();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.temp_owner = Some(new_owner.clone());

    contract.commit_owner();

    assert_eq!(contract.owner_account_id, new_owner);
    assert_eq!(contract.temp_owner, None);
}

#[test]
#[should_panic]
fn test_set_operator_account_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let new_operator_account = AccountId::from_str("new_operator").unwrap();

    context.predecessor_account_id = AccountId::from_str("user").unwrap();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_operator_id(new_operator_account);
}

#[test]
#[should_panic]
fn test_set_operator_account_same_as_owner() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_operator_id(owner_account());
}

#[test]
#[should_panic]
fn test_set_operator_account_same_as_treasury() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_operator_id(treasury_account());
}

#[test]
#[should_panic]
fn test_set_operator_account_same_as_current_account() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_operator_id(context.current_account_id);
}

#[test]
#[should_panic]
fn test_set_treasury_account_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let new_treasury_account = AccountId::from_str("new_treasury").unwrap();

    context.predecessor_account_id = AccountId::from_str("user").unwrap();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_treasury_id(new_treasury_account);
}

#[test]
#[should_panic]
fn test_set_treasury_account_same_as_owner() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_treasury_id(owner_account());
}

#[test]
#[should_panic]
fn test_set_treasury_account_same_as_operator() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_treasury_id(operator_account());
}

#[test]
#[should_panic]
fn test_set_treasury_account_same_as_current_account() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_treasury_id(context.current_account_id);
}

#[test]
fn test_set_operator_account() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let new_operator_account = AccountId::from_str("new_operator").unwrap();

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_operator_id(new_operator_account.clone());

    contract.operator_account_id = new_operator_account;
}

#[test]
#[should_panic]
fn test_commit_operator_account_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.commit_operator_id();
}

#[test]
#[should_panic]
fn test_commit_operator_account_not_set() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = operator_account();
    context.signer_account_id = operator_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.commit_operator_id();
}

#[test]
fn test_commit_operator_account_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let new_operator_account = AccountId::from_str("new_operator").unwrap();

    context.predecessor_account_id = new_operator_account.clone();
    context.signer_account_id = new_operator_account.clone();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.temp_operator = Some(new_operator_account.clone());

    contract.commit_operator_id();

    assert_eq!(contract.operator_account_id, new_operator_account);
    assert!(contract.temp_operator.is_none());
}

#[test]
fn test_set_treasury_account() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let new_treasury_account = AccountId::from_str("new_treasury").unwrap();

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_treasury_id(new_treasury_account);
}

#[test]
#[should_panic]
fn test_commit_treasury_account_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.commit_treasury_id();
}

#[test]
#[should_panic]
fn test_commit_treasury_account_not_set() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = treasury_account();
    context.signer_account_id = treasury_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.commit_treasury_id();
}

#[test]
fn test_commit_treasury_account_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let new_treasury_account = AccountId::from_str("new_treasury").unwrap();

    context.predecessor_account_id = new_treasury_account.clone();
    context.signer_account_id = new_treasury_account.clone();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.temp_treasury = Some(new_treasury_account.clone());

    contract.commit_treasury_id();

    assert_eq!(contract.treasury_account_id, new_treasury_account);
    assert!(contract.temp_treasury.is_none());
}

#[test]
#[should_panic]
fn test_set_min_deposit_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = AccountId::from_str("user").unwrap();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_min_deposit(U128(ntoy(10)));
}

#[test]
#[should_panic]
fn test_set_min_deposit_less_than_one_near() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_min_deposit(U128(10000));
}

#[test]
#[should_panic]
fn test_set_min_deposit_more_than_100_near() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_min_deposit(U128(ntoy(200)));
}

#[test]
fn test_set_min_deposit() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.set_min_deposit(U128(ntoy(50)));

    assert_eq!(contract.min_deposit_amount, ntoy(50));
}

#[test]
#[should_panic]
fn test_pause_validator_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    context.epoch_height = 100;
    testing_env!(context.clone());

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();
    let stake_public_key_3 = AccountId::from_str("stake_public_key_3").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);
    contract.add_validator(stake_public_key_2.clone(), 20);
    contract.add_validator(stake_public_key_3.clone(), 30);

    assert_eq!(contract.total_validator_weight, 60);

    context.predecessor_account_id = AccountId::from_str("user_1").unwrap();
    testing_env!(context.clone());

    contract.pause_validator(stake_public_key_1.clone());

    let val1 = get_validator(&contract, stake_public_key_1);
    assert_eq!(val1.weight, 0);

    assert_eq!(contract.total_validator_weight, 50);
}

#[test]
fn test_pause_validator() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.signer_account_id = owner_account();
    context.attached_deposit = 1;
    context.epoch_height = 100;
    testing_env!(context.clone());

    let stake_public_key_1 = AccountId::from_str("stake_public_key_1").unwrap();
    let stake_public_key_2 = AccountId::from_str("stake_public_key_2").unwrap();
    let stake_public_key_3 = AccountId::from_str("stake_public_key_3").unwrap();

    contract.add_validator(stake_public_key_1.clone(), 10);
    contract.add_validator(stake_public_key_2.clone(), 20);
    contract.add_validator(stake_public_key_3.clone(), 30);

    assert_eq!(contract.total_validator_weight, 60);

    contract.pause_validator(stake_public_key_1.clone());

    let val1 = get_validator(&contract, stake_public_key_1);
    assert_eq!(val1.weight, 0);

    assert_eq!(contract.total_validator_weight, 50);
}

#[test]
#[should_panic]
fn test_ft_transfer_same_user() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1_account_id = AccountId::from_str("user1").unwrap();

    let user1_account = Account {
        stake_shares: ntoy(10),
        unstaked_amount: 0,
        withdrawable_epoch_height: 0,
    };

    update_account(&mut contract, user1_account_id.clone(), &user1_account);

    context.predecessor_account_id = user1_account_id.clone();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.ft_transfer(user1_account_id.clone(), U128(ntoy(5)), None);
}

#[test]
fn test_ft_transfer() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1_account_id = AccountId::from_str("user1").unwrap();
    let user2_account_id = AccountId::from_str("user2").unwrap();

    let user1_account = Account {
        stake_shares: ntoy(10),
        unstaked_amount: 0,
        withdrawable_epoch_height: 0,
    };

    update_account(&mut contract, user1_account_id.clone(), &user1_account);

    context.predecessor_account_id = user1_account_id.clone();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.ft_transfer(user2_account_id.clone(), U128(ntoy(5)), None);

    let user1_account = get_account(&contract, user1_account_id.clone());
    assert_eq!(
        user1_account,
        Account {
            stake_shares: ntoy(5),
            unstaked_amount: 0,
            withdrawable_epoch_height: 0
        }
    );

    let user2_account = get_account(&contract, user2_account_id.clone());
    assert_eq!(
        user2_account,
        Account {
            stake_shares: ntoy(5),
            unstaked_amount: 0,
            withdrawable_epoch_height: 0
        }
    );
    update_account(
        &mut contract,
        user1_account_id.clone(),
        &Account {
            stake_shares: ntoy(0),
            unstaked_amount: ntoy(0),
            withdrawable_epoch_height: 100,
        },
    );

    contract.storage_unregister(None);

    let user1_account = get_account_option(&contract, user1_account_id);
    assert!(user1_account.is_none());
}

#[test]
fn test_storage_unregister_no_account() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1_account = AccountId::from_str("user1").unwrap();

    context.predecessor_account_id = user1_account;
    context.attached_deposit = 1;
    testing_env!(context.clone());

    assert!(!contract.storage_unregister(None));
}

#[test]
#[should_panic]
fn test_storage_unregister_account_non_empty_account() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1_account_id = AccountId::from_str("user1").unwrap();

    context.predecessor_account_id = user1_account_id.clone();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    update_account(
        &mut contract,
        user1_account_id.clone(),
        &Account {
            stake_shares: ntoy(10),
            unstaked_amount: ntoy(10),
            withdrawable_epoch_height: 100,
        },
    );

    contract.storage_unregister(None);
}

#[test]
fn test_storage_unregister_account_success() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1_account_id = AccountId::from_str("user1").unwrap();

    context.predecessor_account_id = user1_account_id.clone();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    update_account(
        &mut contract,
        user1_account_id.clone(),
        &Account {
            stake_shares: ntoy(0),
            unstaked_amount: ntoy(0),
            withdrawable_epoch_height: 100,
        },
    );

    contract.storage_unregister(None);

    let user1_account = get_account_option(&contract, user1_account_id);
    assert!(user1_account.is_none());
}

#[test]
#[should_panic]
fn test_update_user_state_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1_account_id = AccountId::from_str("user1").unwrap();
    context.predecessor_account_id = user1_account_id;
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.migrate_user_state(Vec::new());
}

#[test]
fn test_migrate_user_state() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let user1 = AccountId::from_str("user1").unwrap();
    let user2 = AccountId::from_str("user2").unwrap();

    contract.total_staked = ntoy(10);
    contract.total_stake_shares = ntoy(10);

    contract.migrate_user_state(vec![
        AccountUpdateRequest {
            account_id: user1.clone(),
            stake_shares: U128(ntoy(10)),
            staked_amount: U128(ntoy(10)),
            unstaked_amount: U128(ntoy(5)),
            withdrawable_epoch_height: 0,
        },
        AccountUpdateRequest {
            account_id: user2.clone(),
            stake_shares: U128(ntoy(20)),
            staked_amount: U128(ntoy(20)),
            unstaked_amount: U128(ntoy(5)),
            withdrawable_epoch_height: 0,
        },
    ]);

    let user1 = get_account(&contract, user1);
    let user2 = get_account(&contract, user2);

    assert_eq!(user1.stake_shares, ntoy(10));
    assert_eq!(user1.unstaked_amount, ntoy(5));
    assert_eq!(user2.stake_shares, ntoy(20));
    assert_eq!(user2.unstaked_amount, ntoy(5));

    assert_eq!(contract.total_staked, ntoy(40));
    assert_eq!(contract.total_stake_shares, ntoy(40));
}

#[test]
#[should_panic]
fn test_migrate_contract_state_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();
    
    context.predecessor_account_id = user1;
    context.attached_deposit = 1;
    testing_env!(context.clone());
    
    contract.migrate_contract_state(ContractStateUpdateRequest {
        total_staked: None,
        total_stake_shares: None,
        accumulated_staked_rewards: None
    });
}

#[test]
fn test_migrate_contract_state() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    contract.total_staked = ntoy(50);
    contract.total_stake_shares = ntoy(25);
    contract.accumulated_staked_rewards = ntoy(10);

    contract.migrate_contract_state(ContractStateUpdateRequest {
        total_staked: Some(U128(ntoy(100))),
        total_stake_shares: Some(U128(ntoy(50))),
        accumulated_staked_rewards: Some(U128(ntoy(10)))
    });

    assert_eq!(contract.total_staked, ntoy(100));
    assert_eq!(contract.total_stake_shares, ntoy(50));
    assert_eq!(contract.accumulated_staked_rewards, ntoy(10));
}

#[test]
#[should_panic]
fn test_migrate_validator_state_unauthorized() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());

    let user1 = AccountId::from_str("user1").unwrap();

    context.predecessor_account_id = user1;
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1_id = AccountId::from_str("validator1").unwrap();

    contract.migrate_validator_state(ValidatorUpdateRequest {
        validator_account_id: validator1_id,
        staked_amount: None,
        unstaked_amount: None
    });
}

#[test]
fn test_migrate_validator_state() {
    let (mut context, mut contract) =
        contract_setup(owner_account(), operator_account(), treasury_account());


    context.predecessor_account_id = owner_account();
    context.attached_deposit = 1;
    testing_env!(context.clone());

    let validator1_id = AccountId::from_str("validator1").unwrap();
    contract.add_validator(validator1_id.clone(), 10);

    contract.migrate_validator_state(ValidatorUpdateRequest {
        validator_account_id: validator1_id.clone(),
        staked_amount: Some(U128(ntoy(30))),
        unstaked_amount: Some(U128(ntoy(10)))
    });

    let validator1_info = get_validator(&contract, validator1_id);
    assert_eq!(validator1_info.staked, ntoy(30));
    assert_eq!(validator1_info.unstaked_amount, ntoy(10));
}