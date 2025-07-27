use pinocchio::{account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError, pubkey::find_program_address, ProgramResult};
use pinocchio_token::instructions::Transfer;

use crate::{AssociatedTokenAccount, AssociatedTokenAccountCheck, MintInterface, PinocchioError, Pool, SignerAccount};

pub struct SwapAccounts<'a> {
    pub user: &'a AccountInfo, 
    pub pool: &'a AccountInfo,

    pub token_a_vault: &'a AccountInfo,
    pub token_b_vault: &'a AccountInfo,

    pub user_token_a: &'a AccountInfo,
    pub user_token_b: &'a AccountInfo,

    pub token_a_mint: &'a AccountInfo,
    pub token_b_mint: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for SwapAccounts<'a> {
    type Error = ProgramError;
    
    fn try_from(value: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            user,
            pool,
            token_a_vault,
            token_b_vault,
            user_token_a,
            user_token_b,
            token_a_mint,
            token_b_mint,
            token_program,
            _
        ] = value else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        <&AccountInfo as SignerAccount>::check(user)?;
        <&AccountInfo as MintInterface>::check(token_a_mint)?;
        <&AccountInfo as MintInterface>::check(token_b_mint)?;

        AssociatedTokenAccount::check(token_a_vault, pool, token_a_mint)?;
        AssociatedTokenAccount::check(token_b_vault, pool, token_b_mint)?;

        AssociatedTokenAccount::check(user_token_a, user, token_a_mint)?;
        AssociatedTokenAccount::check(user_token_b, user, token_b_mint)?;

        Ok(Self {
            user,
            pool,
            token_a_vault,
            token_b_vault,
            user_token_a,
            user_token_b,
            token_a_mint,
            token_b_mint,
            token_program
        })
    }
}

pub struct SwapData {
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub swap_direction: bool, // true for A to B, false for B to A
}

impl TryFrom<&[u8]> for SwapData {
    type Error = ProgramError;
    
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 17 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount_in = u64::from_le_bytes(value[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?);
        let min_amount_out = u64::from_le_bytes(value[8..16].try_into().map_err(|_| ProgramError::InvalidInstructionData)?);
        let swap_direction = value[16] != 0;

        Ok(Self {
            amount_in,
            min_amount_out,
            swap_direction,
        })
    }
}

pub struct Swap<'a> {
    pub accounts: SwapAccounts<'a>,
    pub data: SwapData,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for Swap<'a> {
    type Error = ProgramError;

    fn try_from((accounts, instruction_data): (&'a [AccountInfo], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = SwapAccounts::try_from(accounts)?;
        let data = SwapData::try_from(instruction_data)?;

        Ok(Self { accounts, data })
    }
}

impl<'a> Swap<'a> {
    pub const DISCRIMINATOR: &'a u8 = &3;

    pub fn process(&mut self) -> ProgramResult {
        let token_a_vault_data = self.accounts.token_a_vault.try_borrow_data()?;
        let token_a_vault = unsafe { pinocchio_token::state::TokenAccount::from_bytes(&token_a_vault_data) };
        
        let token_b_vault_data = self.accounts.token_b_vault.try_borrow_data()?;
        let token_b_vault = unsafe { pinocchio_token::state::TokenAccount::from_bytes(&token_b_vault_data) };

        if token_a_vault.owner() != self.accounts.pool.key() || token_a_vault.mint() != self.accounts.token_a_mint.key() {
            return Err(ProgramError::InvalidAccountData);
        }
        if token_b_vault.owner() != self.accounts.pool.key() || token_b_vault.mint() != self.accounts.token_b_mint.key() {
            return Err(ProgramError::InvalidAccountData);
       }

        let (reserve_a, reserve_b) = if self.data.swap_direction {
                (token_a_vault.amount(), token_b_vault.amount())
            } else {
                (token_b_vault.amount(), token_a_vault.amount())
            };

        let mut pool_data = self.accounts.pool.try_borrow_mut_data()?;
        let pool = Pool::load_mut(&mut pool_data)?;
        let fee_rate = pool.fee_rate as u64;

        let amount_in_with_fee = self.data.amount_in
            .checked_mul(10000 - fee_rate)
            .ok_or(PinocchioError::MathOverflow)?
            .checked_div(10000)
            .ok_or(PinocchioError::MathOverflow)?;

        let amount_out = amount_in_with_fee
            .checked_mul(reserve_b)
            .ok_or(PinocchioError::MathOverflow)?
            .checked_div(reserve_a + amount_in_with_fee)
            .ok_or(PinocchioError::MathOverflow)?;

        if amount_out < self.data.min_amount_out {
            return Err(PinocchioError::SlippageExceeded.into());
        }

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

        if self.data.swap_direction {
            Transfer {
                from: self.accounts.user_token_a,
                to: self.accounts.token_b_vault,
                authority: self.accounts.user,
                amount: self.data.amount_in,
            }.invoke()?;

            Transfer {
                from: self.accounts.token_b_vault,
                to: self.accounts.user_token_b,
                authority: self.accounts.pool,
                amount: amount_out,
            }.invoke_signed(&signers)?;
        }else {
            Transfer {
                from: self.accounts.user_token_b,
                to: self.accounts.token_a_vault,
                authority: self.accounts.user,
                amount: self.data.amount_in,
            }.invoke()?;

            Transfer {
                from: self.accounts.token_a_vault,
                to: self.accounts.user_token_a,
                authority: self.accounts.pool,
                amount: amount_out,
            }.invoke_signed(&signers)?;
        }

        Ok(())
    }
}