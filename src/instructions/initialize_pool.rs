use pinocchio::{
    account_info::AccountInfo, 
    instruction::Seed, 
    program_error::ProgramError, 
    pubkey::find_program_address, 
    ProgramResult
};

use crate::{
    AssociatedTokenAccount, AssociatedTokenAccountInit, 
    Pool, MintInterface, ProgramAccount, SignerAccount, ProgramAccountInit, PinocchioError,
};

pub struct InitializePoolAccounts<'a> {
    pub authority: &'a AccountInfo,
    pub pool: &'a AccountInfo,
    pub token_a_mint: &'a AccountInfo,
    pub token_b_mint: &'a AccountInfo,

    pub token_a_vault: &'a AccountInfo,
    pub token_b_vault: &'a AccountInfo,

    pub lp_mint: &'a AccountInfo,

    pub token_a_program: &'a AccountInfo,
    pub token_b_program: &'a AccountInfo,

    pub token_program: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub associated_token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for InitializePoolAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            authority,
            pool,
            token_a_mint,
            token_b_mint,
            token_a_vault,
            token_b_vault,
            lp_mint,
            token_a_program,
            token_b_program,
            token_program,
            system_program,
            associated_token_program,
            _
        ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        <&AccountInfo as SignerAccount>::check(authority)?;
        <&AccountInfo as MintInterface>::check(token_a_mint)?;
        <&AccountInfo as MintInterface>::check(token_b_mint)?;

        if token_a_mint.key() == token_b_mint.key() {
            return Err(PinocchioError::IdenticalMints.into());
        }

        Ok(Self { 
            authority, 
            pool, 
            token_a_mint, 
            token_b_mint, 
            token_a_vault, 
            token_b_vault, 
            lp_mint, 
            token_a_program, 
            token_b_program, 
            token_program, 
            system_program, 
            associated_token_program 
        })
    }
}

pub struct InitializePoolData {
    pub fee_rate: u16,
}

impl<'a> TryFrom<&'a [u8]> for InitializePoolData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 2 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let fee_rate = u16::from_le_bytes([data[0], data[1]]);

        if fee_rate > 10000 {
            return Err(PinocchioError::InvalidFeeRate.into());
        }

        Ok(Self { fee_rate })
    }
}

pub struct InitializePool<'a> {
    pub accounts: InitializePoolAccounts<'a>,
    pub data: InitializePoolData,
    pub pool_bump: u8,
    pub lp_mint_bump: u8,
}

impl<'a> TryFrom<(&'a [AccountInfo], &'a [u8])> for InitializePool<'a> {
    type Error = ProgramError;

    fn try_from((accounts, data): (&'a [AccountInfo], &'a [u8])) -> Result<Self, Self::Error> {
        let accounts = InitializePoolAccounts::try_from(accounts)?;
        let data = InitializePoolData::try_from(data)?;

        let (_, pool_bump) = find_program_address(
            &[b"pool", accounts.token_a_mint.key().as_ref(), accounts.token_b_mint.key().as_ref()], 
            &crate::ID
        );
        let (_, lp_mint_bump) = find_program_address(
            &[b"lp_mint", accounts.pool.key().as_ref()], 
            &crate::ID
        );

        let pool_bump_binding = [pool_bump];
        let pool_seeds = [
            Seed::from(b"pool"),
            Seed::from(accounts.token_a_mint.key().as_ref()),
            Seed::from(accounts.token_b_mint.key().as_ref()),
            Seed::from(&pool_bump_binding),
        ];

        let lp_mint_bump_binding = [lp_mint_bump];
        let lp_mint_seeds = [
            Seed::from(b"lp_mint"),
            Seed::from(accounts.pool.key().as_ref()),
            Seed::from(&lp_mint_bump_binding),
        ];

        ProgramAccount::init::<Pool>(
            accounts.authority,
            accounts.pool,
            &pool_seeds,
            Pool::LEN,
        )?;

        ProgramAccount::init::<pinocchio_token::state::Mint>(
            accounts.authority,
            accounts.lp_mint,
            &lp_mint_seeds,
            pinocchio_token::state::Mint::LEN,
        )?;

        AssociatedTokenAccount::init(
            accounts.token_a_vault,
            accounts.token_a_mint,
            accounts.pool,
            accounts.authority,
            accounts.system_program,
            accounts.token_program,
        )?;

        AssociatedTokenAccount::init(
            accounts.token_b_vault,
            accounts.token_b_mint,
            accounts.pool,
            accounts.authority,
            accounts.system_program,
            accounts.token_program,
        )?;

        Ok(Self { 
            accounts, 
            data, 
            pool_bump,
            lp_mint_bump,
        })
    }
}

impl<'a> InitializePool<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;

    pub fn process(&mut self) -> ProgramResult {
        let mut pool_data = self.accounts.pool.try_borrow_mut_data()?;
        let pool = Pool::load_mut(pool_data.as_mut())?;

        pool.set_inner_full(
            *self.accounts.authority.key(),
            *self.accounts.token_a_mint.key(),
            *self.accounts.token_b_mint.key(),
            *self.accounts.token_a_vault.key(),
            *self.accounts.token_b_vault.key(),
            *self.accounts.lp_mint.key(),
            self.data.fee_rate,
            self.pool_bump,
            self.lp_mint_bump,
        );

        Ok(())
    }
}

