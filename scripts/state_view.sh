near view $CONTRACT_NAME get_nearx_pool_state

near view $CONTRACT_NAME get_nearx_price

near view $CONTRACT_NAME get_total_staked

near view $CONTRACT_NAME get_validators

near view $CONTRACT_NAME get_total_validator_weight

near view $CONTRACT_NAME get_validator_info '{"validator": "'"$STAKE_POOL_0"'"}'

near view $CONTRACT_NAME get_account '{"account_id":  "'"$ID"'"}'
near view $CONTRACT_NAME get_account '{"account_id": "ae97284e7ebd60700cafb84b6fb1fdeeb0f6558b130bdcf6f25b3caa89c6ecc5"}'

near view $CONTRACT_NAME get_current_epoch

near view $CONTRACT_NAME get_reward_fee_fraction

near view $CONTRACT_NAME get_roles

near view $CONTRACT_NAME ft_balance_of '{"account_id": "'"$ID"'"}'

near view $CONTRACT_NAME ft_total_supply

near view $STAKE_POOL_0 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_total_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_2 get_account_staked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'

near view $STAKE_POOL_0 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_1 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
near view $STAKE_POOL_2 get_account_unstaked_balance '{"account_id": "'"$CONTRACT_NAME"'"}'
