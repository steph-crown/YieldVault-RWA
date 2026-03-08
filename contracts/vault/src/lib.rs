#![no_std]

mod test;

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Token,
    TotalShares,
    TotalAssets,
    Admin,
    ShareBalance(Address),
}

#[contract]
pub struct YieldVault;

#[contractimpl]
impl YieldVault {
    /// Initialize the vault with the underlying asset (USDC) and an admin who controls the strategy.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::TotalShares, &0i128);
        env.storage().instance().set(&DataKey::TotalAssets, &0i128);
    }

    /// Read the underlying token address.
    pub fn token(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Token).unwrap()
    }

    /// Read the total minted shares.
    pub fn total_shares(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0)
    }

    /// Read the total underlying assets.
    pub fn total_assets(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalAssets).unwrap_or(0)
    }

    /// Read a user's share balance.
    pub fn balance(env: Env, user: Address) -> i128 {
        env.storage().instance().get(&DataKey::ShareBalance(user)).unwrap_or(0)
    }

    /// Calculates the number of shares given an asset amount based on the current exchange rate.
    pub fn calculate_shares(env: Env, assets: i128) -> i128 {
        let ts = Self::total_shares(env.clone());
        let ta = Self::total_assets(env.clone());
        if ta == 0 || ts == 0 {
            assets
        } else {
            assets * ts / ta
        }
    }

    /// Calculates the underlying asset value given an amount of shares.
    pub fn calculate_assets(env: Env, shares: i128) -> i128 {
        let ts = Self::total_shares(env.clone());
        let ta = Self::total_assets(env.clone());
        if ts == 0 {
            0
        } else {
            shares * ta / ts
        }
    }

    /// Deposits USDC into the vault and mints proportional shares to the user.
    pub fn deposit(env: Env, user: Address, amount: i128) -> i128 {
        user.require_auth();
        if amount <= 0 { panic!("deposit must be > 0"); }

        let token_addr = Self::token(env.clone());
        let token_client = token::Client::new(&env, &token_addr);

        let shares_to_mint = Self::calculate_shares(env.clone(), amount);
        
        // Transfer assets from user to vault
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        // Update state
        let ta = Self::total_assets(env.clone());
        env.storage().instance().set(&DataKey::TotalAssets, &(ta + amount));
        
        let ts = Self::total_shares(env.clone());
        env.storage().instance().set(&DataKey::TotalShares, &(ts + shares_to_mint));

        let user_shares = Self::balance(env.clone(), user.clone());
        env.storage().instance().set(&DataKey::ShareBalance(user), &(user_shares + shares_to_mint));

        shares_to_mint
    }

    /// Withdraws USDC backed by burned shares from the user.
    pub fn withdraw(env: Env, user: Address, shares: i128) -> i128 {
        user.require_auth();
        if shares <= 0 { panic!("withdraw must be > 0"); }

        let user_shares = Self::balance(env.clone(), user.clone());
        if user_shares < shares { panic!("insufficient shares"); }

        let assets_to_return = Self::calculate_assets(env.clone(), shares);

        let token_addr = Self::token(env.clone());
        let token_client = token::Client::new(&env, &token_addr);

        // Transfer assets from vault to user
        token_client.transfer(&env.current_contract_address(), &user, &assets_to_return);

        // Update state
        let ta = Self::total_assets(env.clone());
        env.storage().instance().set(&DataKey::TotalAssets, &(ta - assets_to_return));
        
        let ts = Self::total_shares(env.clone());
        env.storage().instance().set(&DataKey::TotalShares, &(ts - shares));

        env.storage().instance().set(&DataKey::ShareBalance(user), &(user_shares - shares));

        assets_to_return
    }

    /// Admin function to artificially accrue yield, simulating returns from an RWA strategy.
    /// This simply bumps the `total_assets` tracked by the vault, immediately increasing the
    /// exchange rate for all share holders. Real implementation would pull this from an RWA protocol.
    pub fn accrue_yield(env: Env, amount: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let token_addr = Self::token(env.clone());
        let token_client = token::Client::new(&env, &token_addr);

        // Transfer the generated yield from the admin (or strategy contract) into the vault.
        token_client.transfer(&admin, &env.current_contract_address(), &amount);

        let ta = Self::total_assets(env.clone());
        env.storage().instance().set(&DataKey::TotalAssets, &(ta + amount));
    }
}
