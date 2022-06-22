use crate::constants::*;
use crate::errors::*;
use crate::events::Event;
use crate::{
    constants::{gas, NO_DEPOSIT},
    contract::*,
    state::*,
    utils::assert_callback_calling,
};
use near_sdk::{is_promise_success, log, require, AccountId, Balance, Promise, PromiseOrValue};

#[near_bindgen]
impl NearxPool {
    /// mints NearX based on user's deposited amount and current NearX price
    pub(crate) fn internal_deposit_and_stake_direct_stake(&mut self, user_amount: Balance) {
        self.assert_min_deposit_amount(user_amount);

        self.assert_staking_not_paused();

        let account_id = env::predecessor_account_id();

        // Calculate the number of nearx (stake shares) that the account will receive for staking the given amount.
        let num_shares = self.num_shares_from_staked_amount_rounded_down(user_amount);
        require!(num_shares > 0, ERROR_NON_POSITIVE_STAKE_SHARES);

        let selected_validator = self.get_validator_to_stake();
        require!(selected_validator.is_some(), ERROR_ALL_VALIDATORS_ARE_BUSY);

        let selected_validator = selected_validator.unwrap();

        //schedule async deposit_and_stake on that pool
        ext_staking_pool::ext(selected_validator.account_id.clone())
            .with_static_gas(gas::DEPOSIT_AND_STAKE)
            .with_attached_deposit(user_amount)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE)
                    .on_stake_pool_deposit_and_stake_direct(
                        selected_validator,
                        user_amount,
                        num_shares,
                        account_id,
                    ),
            );
    }

    #[private]
    pub fn internal_manager_deposit_and_stake(&mut self, user_amount: Balance) {
        let account_id = env::predecessor_account_id();

        // Calculate the number of nearx (stake shares) that the account will receive for staking the given amount.
        let num_shares = self.num_shares_from_staked_amount_rounded_down(user_amount);
        require!(num_shares > 0, ERROR_NON_POSITIVE_STAKE_SHARES);

        let selected_validator = self.get_validator_to_stake();
        require!(selected_validator.is_some(), ERROR_ALL_VALIDATORS_ARE_BUSY);

        let selected_validator = selected_validator.unwrap();

        //schedule async deposit_and_stake on that pool
        ext_staking_pool::ext(selected_validator.account_id.clone())
            .with_static_gas(gas::DEPOSIT_AND_STAKE)
            .with_attached_deposit(user_amount)
            .deposit_and_stake()
            .then(
                ext_staking_pool_callback::ext(env::current_account_id())
                    .with_attached_deposit(NO_DEPOSIT)
                    .with_static_gas(gas::ON_STAKE_POOL_DEPOSIT_AND_STAKE)
                    .on_stake_pool_deposit_and_stake_manager(
                        selected_validator,
                        user_amount,
                        num_shares,
                        account_id,
                    ),
            );
    }

    #[private]
    pub fn on_stake_pool_deposit_and_stake_manager(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool> {
        let mut acc = &mut self.accounts.get(&user).unwrap_or_default();

        if is_promise_success() {
            validator_info.staked += amount;
            acc.stake_shares += shares;
            self.total_stake_shares += shares;
            self.total_staked += amount;
            log!(
                "Successfully staked {} into {}",
                amount,
                validator_info.account_id
            );
            self.internal_update_validator(&validator_info.account_id, &validator_info);
            self.internal_update_account(&user, acc);
            PromiseOrValue::Value(true)
        } else {
            log!(
                "Failed to stake {} into {}",
                amount,
                validator_info.account_id
            );
            log!("Transfering back {} to {} after stake failed", amount, user);
            PromiseOrValue::Promise(Promise::new(user).transfer(amount))
        }
    }

    // TODO - bchain - I think this is better than the direct stake
    pub(crate) fn internal_deposit_and_stake(&mut self, amount: u128) {
        self.assert_staking_not_paused();

        self.assert_min_deposit_amount(amount);

        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);

        // Calculate the number of "stake" shares that the account will receive for staking the
        // given amount.
        let num_shares = self.num_shares_from_staked_amount_rounded_down(amount);
        require!(num_shares > 0, ERROR_NON_POSITIVE_STAKE_SHARES);

        account.stake_shares += num_shares;
        self.internal_update_account(&account_id, &account);

        self.total_staked += amount;
        self.total_stake_shares += num_shares;

        // Increase requested stake amount within the current epoch
        self.user_amount_to_stake_in_epoch += amount;

        Event::DepositAndStake {
            account_id,
            amount: U128(amount),
            minted_stake_shares: U128(num_shares),
            new_stake_shares: U128(account.stake_shares),
        }
        .emit();
    }

    pub(crate) fn internal_unstake(&mut self, amount: u128) {
        require!(amount > 0, ERROR_NON_POSITIVE_UNSTAKE_AMOUNT);

        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);

        require!(
            self.total_staked > 0,
            ERROR_NOT_ENOUGH_CONTRACT_STAKED_AMOUNT
        );

        let num_shares = self.num_shares_from_staked_amount_rounded_up(amount);
        require!(num_shares > 0, ERROR_NON_POSITIVE_UNSTAKING_SHARES);
        require!(
            account.stake_shares >= num_shares,
            ERROR_NOT_ENOUGH_STAKED_AMOUNT_TO_UNSTAKE
        );

        let receive_amount = self.staked_amount_from_num_shares_rounded_up(num_shares);
        require!(
            receive_amount > 0,
            ERROR_NON_POSITIVE_UNSTAKE_RECEVIE_AMOUNT
        );

        account.stake_shares -= num_shares;
        account.unstaked_amount += receive_amount;
        account.withdrawable_epoch_height =
            env::epoch_height() + self.get_unstake_release_epoch(amount);
        if self.last_reconcilation_epoch == env::epoch_height() {
            // The unstake request is received after epoch_reconcilation
            // so actual unstake will happen in the next epoch,
            // which will put withdraw off for one more epoch.
            account.withdrawable_epoch_height += 1;
        }

        self.internal_update_account(&account_id, &account);

        self.total_staked -= receive_amount;
        self.total_stake_shares -= num_shares;

        // Increase requested unstake amount within the current epoch
        self.user_amount_to_unstake_in_epoch += receive_amount;

        Event::Unstake {
            account_id,
            unstaked_amount: U128(amount),
            burnt_stake_shares: U128(num_shares),
            new_unstaked_balance: U128(account.unstaked_amount),
            new_stake_shares: U128(account.stake_shares),
            unstaked_available_epoch_height: account.withdrawable_epoch_height,
        }
        .emit();
    }

    pub(crate) fn internal_withdraw(&mut self, amount: Balance) {
        let account_id = env::predecessor_account_id();

        require!(amount > 0, ERROR_NON_POSITIVE_WITHDRAWAL);

        let account = self.internal_get_account(&account_id);
        require!(
            account.unstaked_amount >= amount,
            ERROR_NOT_ENOUGH_UNSTAKED_AMOUNT_TO_WITHDRAW
        );
        require!(
            account.withdrawable_epoch_height <= env::epoch_height(),
            ERROR_UNSTAKED_AMOUNT_IN_UNBONDING_PERIOD
        );

        require!(
            env::account_balance().saturating_sub(MIN_BALANCE_FOR_STORAGE) >= amount,
            ERROR_NOT_ENOUGH_BALANCE_FOR_STORAGE
        );

        let mut account = self.internal_get_account(&account_id);
        account.unstaked_amount -= amount;
        self.internal_update_account(&account_id, &account);

        Event::Withdraw {
            account_id: account_id.clone(),
            amount: U128(amount),
            new_unstaked_balance: U128(account.unstaked_amount),
        }
        .emit();

        Promise::new(account_id).transfer(amount);
    }

    pub fn on_stake_pool_deposit_and_stake_direct(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        amount: u128,
        shares: u128,
        user: AccountId,
    ) -> PromiseOrValue<bool> {
        assert_callback_calling();

        let mut acc = &mut self.accounts.get(&user).unwrap_or_default();
        let mut transfer_funds = false;

        if is_promise_success() {
            validator_info.staked += amount;
            acc.stake_shares += shares;
            self.total_stake_shares += shares;
            self.total_staked += amount;
            log!(
                "Successfully staked {} into {}",
                amount,
                validator_info.account_id
            );
        } else {
            log!(
                "Failed to stake {} into {}",
                amount,
                validator_info.account_id
            );
            transfer_funds = true;
        }

        self.internal_update_validator(&validator_info.account_id, &validator_info);
        self.internal_update_account(&user, acc);

        if transfer_funds {
            log!("Transfering back {} to {} after stake failed", amount, user);
            PromiseOrValue::Promise(Promise::new(user).transfer(amount))
        } else {
            log!("Reconciling total staked balance");
            // Reconcile the total staked amount to the right value
            ext_staking_pool::ext(validator_info.account_id.clone())
                .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                .with_attached_deposit(NO_DEPOSIT)
                .get_account_staked_balance(env::current_account_id())
                .then(
                    ext_staking_pool_callback::ext(env::current_account_id())
                        .with_attached_deposit(NO_DEPOSIT)
                        .with_static_gas(gas::ON_GET_SP_STAKED_BALANCE_TO_RECONCILE)
                        .on_get_sp_staked_balance_reconcile(validator_info, amount),
                );
            PromiseOrValue::Value(true)
        }
    }

    pub fn on_get_sp_staked_balance_reconcile(
        &mut self,
        #[allow(unused_mut)] mut validator_info: ValidatorInfo,
        amount_actually_staked: u128,
        #[callback] total_staked_balance: U128,
    ) {
        assert_callback_calling();

        log!("Actual staked amount is {}", amount_actually_staked);

        // difference in staked amount and actual staked amount
        let difference_in_amount = validator_info.staked.saturating_sub(total_staked_balance.0);
        // Reconcile the total staked with the actual total staked amount
        self.total_staked -= difference_in_amount;
        log!("Reconciled total staked to {}", self.total_staked);

        // Reconcile the stake pools total staked with the total staked balance
        validator_info.staked = total_staked_balance.0;

        self.internal_update_validator(&validator_info.account_id, &validator_info);
    }

    pub(crate) fn internal_get_validator(&self, validator: &AccountId) -> ValidatorInfo {
        if let Some(val_info) = self.validator_info_map.get(validator) {
            val_info
        } else {
            panic!("{}", ERROR_VALIDATOR_IS_NOT_PRESENT);
        }
    }

    pub(crate) fn internal_update_validator(
        &mut self,
        validator: &AccountId,
        validator_info: &ValidatorInfo,
    ) {
        self.validator_info_map.insert(validator, validator_info);
    }

    pub(crate) fn num_shares_from_staked_amount_rounded_down(&self, amount: Balance) -> u128 {
        // At this point the er will be 1
        if self.total_stake_shares == 0 || self.total_staked == 0 {
            return amount;
        }

        (U256::from(self.total_stake_shares) * U256::from(amount) / U256::from(self.total_staked))
            .as_u128()
    }

    pub(crate) fn num_shares_from_staked_amount_rounded_up(&self, amount: Balance) -> u128 {
        if self.total_stake_shares == 0 || self.total_staked == 0 {
            return amount;
        }

        ((U256::from(self.total_stake_shares) * U256::from(amount)
            + U256::from(self.total_staked - 1))
            / U256::from(self.total_staked))
        .as_u128()
    }

    pub(crate) fn staked_amount_from_num_shares_rounded_down(&self, num_shares: u128) -> Balance {
        if self.total_staked == 0 || self.total_stake_shares == 0 {
            return num_shares;
        }

        (U256::from(self.total_staked) * U256::from(num_shares)
            / U256::from(self.total_stake_shares))
        .as_u128()
    }

    pub(crate) fn staked_amount_from_num_shares_rounded_up(&self, num_shares: u128) -> Balance {
        if self.total_staked == 0 || self.total_stake_shares == 0 {
            return num_shares;
        }

        ((U256::from(self.total_staked) * U256::from(num_shares)
            + U256::from(self.total_stake_shares - 1))
            / U256::from(self.total_stake_shares))
        .as_u128()
    }

    pub(crate) fn internal_get_account(&self, account_id: &AccountId) -> Account {
        self.accounts.get(account_id).unwrap_or_default()
    }

    pub(crate) fn internal_update_account(&mut self, account_id: &AccountId, account: &Account) {
        if account.is_empty() {
            self.accounts.remove(account_id);
        } else {
            self.accounts.insert(account_id, account); //insert_or_update
        }
    }

    #[private]
    pub fn get_validator_to_stake(&self) -> Option<ValidatorInfo> {
        self.validator_info_map
            .values()
            .filter(|v| !v.paused())
            .min_by_key(|v| v.staked)
    }

    #[private]
    pub fn get_validator_to_unstake(&self) -> Option<ValidatorInfo> {
        let mut max_validator_stake_amount: u128 = 0;
        let mut current_validator: Option<ValidatorInfo> = None;

        for validator in self.validator_info_map.values() {
            if !validator.pending_unstake_release()
                && !validator.paused()
                && validator.staked.gt(&max_validator_stake_amount)
            {
                max_validator_stake_amount = validator.staked;
                current_validator = Some(validator)
            }
        }

        current_validator
    }

    #[private]
    pub fn get_unstake_release_epoch(&self, amount: u128) -> EpochHeight {
        let mut available_amount: Balance = 0;
        let mut total_staked_amount: Balance = 0;
        for validator in self.validator_info_map.values() {
            total_staked_amount += validator.staked;

            if !validator.paused() && !validator.pending_unstake_release() && validator.staked > 0 {
                available_amount += validator.staked;
            }

            // found enough balance to unstake from available validators
            if available_amount >= amount {
                return NUM_EPOCHS_TO_UNLOCK;
            }
        }

        // nothing is actually staked, all balance should be available now
        // still leave a buffer for the user
        if total_staked_amount == 0 {
            return NUM_EPOCHS_TO_UNLOCK;
        }

        // no enough available validators to unstake
        // double the unstake wating time
        2 * NUM_EPOCHS_TO_UNLOCK
    }
}
