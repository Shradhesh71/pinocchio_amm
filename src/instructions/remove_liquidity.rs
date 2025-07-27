use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    ProgramResult,
};
use pinocchio_token::instructions::{Burn, Transfer};

use crate::{AssociatedTokenAccount, AssociatedTokenAccountCheck, MintInterface, PinocchioError, SignerAccount };

pub struct RemoveLiquidityAccounts<'a> {
    pub user: &'a AccountInfo,
    pub pool: &'a AccountInfo,
    pub lp_mint: &'a AccountInfo,

    pub token_a_vault: &'a AccountInfo,
    pub token_b_vault: &'a AccountInfo,

    pub user_token_a: &'a AccountInfo,
    pub user_token_b: &'a AccountInfo,
    pub user_lp_token: &'a AccountInfo,

    pub token_a_mint: &'a AccountInfo,
    pub token_b_mint: &'a AccountInfo,
    pub token_program: &'a AccountInfo
}

impl<'a> TryFrom<&'a [AccountInfo]> for RemoveLiquidityAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error>{
        let [
            user,
            pool,
            lp_mint,
            token_a_vault,
            token_b_vault,
            user_token_a,
            user_token_b,
            user_lp_token,
            token_a_mint,
            token_b_mint,
            token_program,
            _
        ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        <&AccountInfo as SignerAccount>::check(user)?;
        <&AccountInfo as MintInterface>::check(token_a_mint)?;
        <&AccountInfo as MintInterface>::check(token_b_mint)?;

        AssociatedTokenAccount::check(token_a_vault, pool, token_a_mint)?;
        AssociatedTokenAccount::check(token_b_vault, pool, token_b_mint)?;

        AssociatedTokenAccount::check(user_token_a, user, token_a_mint)?;
        AssociatedTokenAccount::check(user_token_b, user, token_b_mint)?;
        AssociatedTokenAccount::check(user_lp_token, user, lp_mint)?;


        Ok(Self{
            user,
            pool,
            lp_mint,
            token_a_vault,
            token_b_vault,
            user_token_a,
            user_token_b,
            user_lp_token,
            token_a_mint,
            token_b_mint,
            token_program
        })
    }
}

pub struct RemoveLiquidityData {
    pub lp_tokens: u64,
    pub min_amount_a: u64,
    pub min_amount_b: u64
}

impl TryFrom<&[u8]> for RemoveLiquidityData{
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() < 24 {
            return Err(ProgramError::InvalidAccountData);
        }

        let lp_tokens = u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        let min_amount_a = u64::from_le_bytes([data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15]]);
        let min_amount_b = u64::from_le_bytes([data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23]]);

        Ok(Self { 
            lp_tokens, 
            min_amount_a, 
            min_amount_b 
        })
    }
}

pub struct RemoveLiquidity<'a> {
    pub accounts: RemoveLiquidityAccounts<'a>,
    pub data: RemoveLiquidityData,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for RemoveLiquidity<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountInfo], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = RemoveLiquidityAccounts::try_from(accounts)?;
        let data = RemoveLiquidityData::try_from(data)?;

        Ok(Self { 
            accounts, 
            data
        })
    }
}

impl<'a> RemoveLiquidity<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        let token_a_vault_data = self.accounts.token_a_vault.try_borrow_data()?;
        let token_a_vault = unsafe {
            pinocchio_token::state::TokenAccount::from_bytes(&token_a_vault_data)
        };
        if token_a_vault.owner() != self.accounts.pool.key() {
            return Err(ProgramError::InvalidAccountData);
        }
        if token_a_vault.mint() != self.accounts.token_a_mint.key() {
            return Err(ProgramError::InvalidAccountData);
        }

        let reserve_a = token_a_vault.amount();

        let token_b_vault_data = self.accounts.token_b_vault.try_borrow_data()?;
        let token_b_vault = unsafe {
            pinocchio_token::state::TokenAccount::from_bytes(&token_b_vault_data)
        };
        if token_b_vault.owner() != self.accounts.pool.key() {
            return Err(ProgramError::InvalidAccountData);
        }
        if token_b_vault.mint() != self.accounts.token_b_mint.key() {
            return Err(ProgramError::InvalidAccountData);
        }

        let reserve_b = token_b_vault.amount();

        let lp_mint_data = self.accounts.lp_mint.try_borrow_data()?;
        let lp_mint= unsafe {
            pinocchio_token::state::Mint::from_bytes(&lp_mint_data)
        };
        if lp_mint.mint_authority() != Some(self.accounts.pool.key()) {
            return Err(ProgramError::InvalidAccountData);
        }

        let lp_suppy = lp_mint.supply();

        if lp_suppy == 0 {
            return Err(ProgramError::InsufficientFunds);
        }
        if lp_suppy <= self.data.lp_tokens {
            return Err(ProgramError::InvalidArgument);
        }

        let lp_tokens = self.data.lp_tokens;
        let amount_a = lp_tokens
            .checked_mul(reserve_a)
            .ok_or(PinocchioError::MathOverflow)?
            .checked_div(lp_suppy)
            .ok_or(PinocchioError::MathOverflow)? as u64;

        let amount_b = lp_tokens
            .checked_mul(reserve_b) 
            .ok_or(PinocchioError::MathOverflow)?
            .checked_div(lp_suppy)
            .ok_or(PinocchioError::MathOverflow)? as u64;

        let min_amount_a = self.data.min_amount_a;
        let min_amount_b = self.data.min_amount_b;

        if amount_a < min_amount_a {
            return Err(PinocchioError::SlippageExceeded.into());
        }
        if amount_b < min_amount_b {
            return Err(PinocchioError::SlippageExceeded.into());
        }
        if amount_a == 0 && amount_b == 0 {
            return Err(ProgramError::InvalidArgument);
        }

        Burn {
            mint: self.accounts.lp_mint,
            authority: self.accounts.user,
            amount: lp_tokens,
            account: self.accounts.user_lp_token
        }.invoke()?;

        let (_, pool_bump) = find_program_address(
            &[b"pool", self.accounts.token_a_mint.key().as_ref(), self.accounts.token_b_mint.key().as_ref()], 
            &crate::ID
        );
        
        let pool_bump_binding = [pool_bump];
        let seeds = [
            Seed::from("pool".as_bytes()),
            Seed::from(self.accounts.token_a_mint.key().as_ref()),
            Seed::from(self.accounts.token_b_mint.key().as_ref()),
            Seed::from(pool_bump_binding.as_ref()),
        ];

        let signers = [Signer::from(&seeds)];
        Transfer {
            from: self.accounts.token_a_vault,
            to: self.accounts.user_token_a,
            authority: self.accounts.pool,
            amount: amount_a,
        }.invoke_signed(&signers)?;

        Transfer {
            from: self.accounts.token_b_vault,
            to: self.accounts.user_token_b,
            authority: self.accounts.pool,
            amount: amount_b,
        }.invoke_signed(&signers)?;

        Ok(())
    }
}
