use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::find_program_address, ProgramResult};
use pinocchio_token::instructions::{MintTo, Transfer};
use crate::{AssociatedTokenAccount, AssociatedTokenAccountCheck, AssociatedTokenAccountInit, MintInterface, SignerAccount, PinocchioError};

pub struct AddLiquidityAccounts<'a> {
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

    pub token_program: &'a AccountInfo,
    pub associated_token_program: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a>TryFrom<&'a [AccountInfo]> for AddLiquidityAccounts<'a> {
    type Error = ProgramError;
    
    fn try_from(value: &'a [AccountInfo]) -> Result<Self, Self::Error> {
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
            associated_token_program,
            system_program,
            _
        ] = value else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        <&AccountInfo as SignerAccount>::check(user)?;
        <&AccountInfo as MintInterface>::check(token_a_mint)?;
        <&AccountInfo as MintInterface>::check(token_b_mint)?;
        
        AssociatedTokenAccount::check(token_a_vault, pool, token_a_mint)?;
        AssociatedTokenAccount::check(token_b_vault, pool, token_b_mint)?;

        let (expected_lp_mint, _) = find_program_address(
            &[b"lp_mint", pool.key().as_ref()], 
            &crate::ID
        );
        if lp_mint.key() != &expected_lp_mint {
            return Err(ProgramError::InvalidAccountData);
        }
        
        let (expected_pool, _) = find_program_address(
            &[b"pool", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()], 
            &crate::ID
        );
        if pool.key() != &expected_pool {
            return Err(ProgramError::InvalidAccountData);
        }

        if token_a_mint.key() == token_b_mint.key() {
            return Err(PinocchioError::IdenticalMints.into());
        }

        Ok(Self {
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
            associated_token_program,
            system_program
        })
    }
}

pub struct AddLiquidityData {
    pub amount_a: u64,
    pub amount_b: u64,
    pub min_lp_amount: u64,
}

impl TryFrom<&[u8]> for AddLiquidityData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() < 24 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount_a = u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
        let amount_b = u64::from_le_bytes([data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15]]);
        let min_lp_amount = u64::from_le_bytes([data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23]]);

        if amount_a == 0 || amount_b == 0 {
            return Err(PinocchioError::InvalidAmount.into());
        }

        Ok(Self {
            amount_a,
            amount_b,
            min_lp_amount,
        })
    }
}

pub struct AddLiquidity<'a> {
    pub accounts: AddLiquidityAccounts<'a>,
    pub data: AddLiquidityData,
    pub pool_bump: u8,
    pub lp_mint_bump: u8,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for AddLiquidity<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountInfo], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = AddLiquidityAccounts::try_from(accounts)?;
        let data = AddLiquidityData::try_from(data)?;

        AssociatedTokenAccount::init_if_needed(
            accounts.user_token_a, 
            accounts.token_a_mint, 
            accounts.user, 
            accounts.user, 
            accounts.system_program, 
            accounts.token_program
        )?;
        AssociatedTokenAccount::init_if_needed(
            accounts.user_token_b, 
            accounts.token_b_mint, 
            accounts.user, 
            accounts.user, 
            accounts.system_program, 
            accounts.token_program
        )?;
        AssociatedTokenAccount::init_if_needed(
            accounts.user_lp_token, 
            accounts.lp_mint, 
            accounts.user, 
            accounts.user, 
            accounts.system_program, 
            accounts.token_program
        )?;

        let (_, pool_bump) = find_program_address(
            &[b"pool", accounts.token_a_mint.key().as_ref(), accounts.token_b_mint.key().as_ref()], 
            &crate::ID
        );
        let (_, lp_mint_bump) = find_program_address(
            &[b"lp_mint", accounts.pool.key().as_ref()], 
            &crate::ID
        );

        Ok(Self {
            accounts,
            data,
            pool_bump,
            lp_mint_bump,
        })
    }
}

impl<'a> AddLiquidity<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        let token_a_vault_data = self.accounts.token_a_vault.try_borrow_data()?;
        let token_a_vault = unsafe { pinocchio_token::state::TokenAccount::from_bytes(&token_a_vault_data) };
        
        let token_b_vault_data = self.accounts.token_b_vault.try_borrow_data()?;
        let token_b_vault = unsafe { pinocchio_token::state::TokenAccount::from_bytes(&token_b_vault_data) };

        let lp_mint_data = self.accounts.lp_mint.try_borrow_data()?;
        let lp_mint = unsafe { pinocchio_token::state::Mint::from_bytes(&lp_mint_data) };

        if token_a_vault.owner() != self.accounts.pool.key() {
            return Err(ProgramError::InvalidAccountData);
        }
        if token_b_vault.owner() != self.accounts.pool.key() {
            return Err(ProgramError::InvalidAccountData);
        }
        if lp_mint.mint_authority() != Some(self.accounts.pool.key()) {
            return Err(ProgramError::InvalidAccountData);
        }

        if token_a_vault.mint() != self.accounts.token_a_mint.key() {
            return Err(ProgramError::InvalidAccountData);
        }
        if token_b_vault.mint() != self.accounts.token_b_mint.key() {
            return Err(ProgramError::InvalidAccountData);
        }

        let reserve_a = token_a_vault.amount();
        let reserve_b = token_b_vault.amount();

        let lp_tokens_to_mint = if reserve_a == 0 && reserve_b == 0 {
            let product = (self.data.amount_a as f64) * (self.data.amount_b as f64);
            if product <= 0.0 {
                return Err(PinocchioError::InvalidAmount.into());
            }
            let sqrt_result = product.sqrt() as u64;
            if sqrt_result < 1000 {
                return Err(PinocchioError::InsufficientLiquidity.into());
            }
            sqrt_result
        } else {
            let lp_supply = lp_mint.supply();
            
            if reserve_a == 0 || reserve_b == 0 || lp_supply == 0 {
                return Err(PinocchioError::InvalidPoolState.into());
            }

            let lp_from_a = (self.data.amount_a as u128)
                .checked_mul(lp_supply as u128)
                .ok_or(PinocchioError::MathOverflow)?
                .checked_div(reserve_a as u128)
                .ok_or(PinocchioError::MathOverflow)? as u64;
                
            let lp_from_b = (self.data.amount_b as u128)
                .checked_mul(lp_supply as u128)
                .ok_or(PinocchioError::MathOverflow)?
                .checked_div(reserve_b as u128)
                .ok_or(PinocchioError::MathOverflow)? as u64;
                
            std::cmp::min(lp_from_a, lp_from_b)
        };

        if lp_tokens_to_mint == 0 {
            return Err(PinocchioError::InvalidAmount.into());
        }
        
        if lp_tokens_to_mint < self.data.min_lp_amount {
            return Err(PinocchioError::SlippageExceeded.into());
        }

        Transfer {
            from: self.accounts.user_token_a,
            to: self.accounts.token_a_vault,
            authority: self.accounts.user,
            amount: self.data.amount_a,
        }.invoke()?;

        Transfer {
            from: self.accounts.user_token_b,
            to: self.accounts.token_b_vault,
            authority: self.accounts.user,
            amount: self.data.amount_b,
        }.invoke()?;

        MintTo {
            mint: self.accounts.lp_mint,
            amount: lp_tokens_to_mint,
            mint_authority: self.accounts.pool,
            account: self.accounts.user_lp_token,
        }.invoke()?;

        Ok(())
    }
}