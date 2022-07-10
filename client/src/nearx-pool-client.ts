import { readFileSync } from 'fs';
import * as nearjs from 'near-api-js';
import * as os from 'os';
import { Balance, Epoch, NearxPoolClient as Iface, Network, SnapshotUser, ValidatorInfo } from '.';
import { createContract, NearxContract } from './contract';
import { isBrowser, range } from './utils';
//import * as bn from 'bn';

const gas = '300000000000000';
const EPOCH_TO_UNBOUND = BigInt(4);

type NearxPoolClient = Iface;
export const NearxPoolClient = {
  async new(
    networkId: 'testnet' | 'mainnet',
    contractName: string,
    // Local account:
    accountId?: string,
  ): Promise<NearxPoolClient> {
    // Depending on being in the browser or not,
    // the config is set from a local keystore or the browser wallet:
    const config = configFromNetwork(networkId);
    // Connect to NEAR:
    const near = await nearjs.connect(config);

    let contract: NearxContract;

    //console.debug('Client created from the', isBrowser() ? 'browser' : 'CLI');
    if (isBrowser()) {
      const wallet = new nearjs.WalletAccount(near, null);

      contract = createContract(wallet.account(), contractName);
      accountId = wallet.getAccountId();
    } else {
      if (accountId == null) {
        throw new Error('When used in a CLI, the accountId must be specified');
      }
      // Use the previously set keystore:
      const account = new nearjs.Account(near.connection, accountId);

      contract = createContract(account, contractName);
    }

    async function getValidators(): Promise<ValidatorInfo[]> {
      return await contract.get_validators({});
    }

    const client = {
      near,
      config,
      contract,

      // View methods:
      async stakedBalance(): Promise<Balance> {
        return BigInt(
          await contract.get_account_staked_balance({
            account_id: accountId,
          }),
        );
      },
      async totalBalance(): Promise<Balance> {
        return BigInt(
          await contract.get_account_total_balance({
            account_id: accountId,
          }),
        );
      },

      async validators(): Promise<ValidatorInfo[]> {
        return contract.get_validators({});
      },

      async currentEpoch(): Promise<Epoch> {
        return contract.get_current_epoch({});
      },

      async userAccounts(usersPerCall: number = 200): Promise<SnapshotUser[]> {
        const nAccounts = await contract.get_number_of_accounts({});

        // Impl with snapshot method:
        //return Promise.all(
        //  range(0, nAccounts, usersPerCall).map((i) =>
        //    contract.get_snapshot_users({ from: i, length: usersPerCall }),
        //  ),
        //).then((arrayOfUsers) => arrayOfUsers.flat());

        // Temporary impl with `ft_balance_of` call for each account:
        const accounts = await Promise.all(
          range(0, nAccounts, usersPerCall).map((i) =>
            contract.get_accounts({ from_index: i, limit: i + usersPerCall }),
          ),
        ).then((arrayOfUsers) => arrayOfUsers.flat());

        const accounts_ = await Promise.all(
          accounts.map((account) =>
            contract
              .ft_balance_of({ account_id: account.account_id })
              .then((balance) => [balance, account.account_id]),
          ),
        );

        return accounts_.map(([balance, accountId]) => ({
          accountId,
          nearxBalance: BigInt(balance),
        }));
      },

      // User-facing methods:
      async stake(amount: string): Promise<string> {
        return contract.deposit_and_stake({ args: {}, amount });
      },

      async unstake(amount: string): Promise<string> {
        throw new Error('Not implemented');
      },

      async unstakeAll(): Promise<string> {
        throw new Error('Not implemented');
      },

      async withdraw(amount: string): Promise<string> {
        throw new Error('Not implemented');
      },

      async withdrawAll(): Promise<string> {
        throw new Error('Not implemented');
      },

      // Operator methods:
      async epochStake(): Promise<string> {
        return contract.epoch_stake({ args: {}, gas });
      },

      async epochAutocompoundRewards(): Promise<any[]> {
        const validators = await getValidators();

        return promiseAllSettledErrors(
          validators.map((v) =>
            contract.epoch_autocompound_rewards({ args: { validator: v.account_id }, gas }),
          ),
        );
      },

      async epochUnstake(): Promise<void> {
        let n = 0;

        while (await contract.epoch_unstake({ args: {}, gas })) {
          n += 1;
        }
        console.debug(`Epoch unstake has unstaked ${n} times.`);
      },

      async epochWithdraw(): Promise<any[]> {
        const currentEpoch = await this.currentEpoch();
        const validators = await getValidators().then((a) =>
          a.filter(
            (v) =>
              v.unstaked !== BigInt(0) &&
              v.last_unstake_start_epoch + EPOCH_TO_UNBOUND <= currentEpoch,
          ),
        );

        return promiseAllSettledErrors(
          validators.map((v) =>
            contract.epoch_withdraw({ args: { validator: v.account_id }, gas }),
          ),
        );
      },

      async syncBalances(): Promise<any[]> {
        const validators = await getValidators();

        return promiseAllSettledErrors(
          validators.map((v) =>
            contract.sync_balance_from_validator({ args: { validator_id: v.account_id }, gas }),
          ),
        );
      },

      async contractUpgrade(fileName: string): Promise<any> {
        const code = readFileSync(fileName);
        return contract.upgrade(code, gas);
      },
    };

    return client;
  },
};

function localAccountPath(): string {
  return `${os.homedir()}/.near-credentials`;
}

function configFromNetwork(networkId: Network): nearjs.ConnectConfig {
  const keyStore = isBrowser()
    ? new nearjs.keyStores.BrowserLocalStorageKeyStore()
    : new nearjs.keyStores.UnencryptedFileSystemKeyStore(localAccountPath());
  const config = {
    keyStore,
    networkId,
    headers: {},
  };

  switch (networkId) {
    case 'testnet':
      return {
        ...config,
        helperUrl: 'https://helper.testnet.near.org',
        nodeUrl: 'https://rpc.testnet.near.org',
        walletUrl: 'https://wallet.testnet.near.org',
      };
    case 'mainnet':
      return {
        ...config,
        helperUrl: 'https://helper.near.org',
        nodeUrl: 'https://rpc.mainnet.near.org',
        walletUrl: 'https://wallet.near.org',
      };
    default:
      throw new Error('Invalid network: ' + networkId);
  }
}

/**
 * Returns the errors after calling `Promise.allSettled`.
 */
async function promiseAllSettledErrors<T>(promises: Promise<T>[]): Promise<any[]> {
  return (await Promise.allSettled(promises))
    .filter((r) => r.status === 'rejected')
    .map((r) => (r as PromiseRejectedResult).reason);
}
