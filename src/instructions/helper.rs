use pinocchio::{
    account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError, pubkey::find_program_address, sysvars::Sysvar
};
use pinocchio_token::state::Mint;

pub trait SignerAccount {
    fn check(account: &AccountInfo) -> Result<(), ProgramError>;
}

impl SignerAccount for &AccountInfo {
    fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }
}

pub trait MintInterface {
    fn check(account: &AccountInfo) -> Result<(), ProgramError>;
}

impl MintInterface for &AccountInfo {
    fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if account.data_len() != Mint::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

pub trait AccountCheck {
    fn check(account: &AccountInfo) -> Result<(), ProgramError>;
}

pub trait ProgramAccountInit {
    fn init<'a, T: Sized>(
        payer: &AccountInfo,
        account: &AccountInfo,
        seeds: &[Seed<'a>],
        space: usize,
    ) -> Result<(), ProgramError>;
}

pub struct ProgramAccount;

impl ProgramAccountInit for ProgramAccount {
    fn init<'a, T: Sized>(
        payer: &AccountInfo,
        account: &AccountInfo,
        seeds: &[Seed<'a>],
        space: usize,
    ) -> Result<(), ProgramError> {
        let signers = [Signer::from(seeds)];

        pinocchio_system::instructions::CreateAccount {
            from: payer,
            to: account,
            owner: &crate::ID,
            lamports: pinocchio::sysvars::rent::Rent::get()?.minimum_balance(space),
            space: space as u64,
        }.invoke_signed(&signers)?;
        
        Ok(())
    }
}


pub struct TokenAccount;
 
impl AccountCheck for TokenAccount {
    fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::IllegalOwner.into());
        }
        if account.data_len().ne(&pinocchio_token::state::TokenAccount::LEN) {
            return Err(ProgramError::InvalidAccountData.into());
        }
 
        Ok(())
    }
}

pub trait AssociatedTokenAccountCheck {
    fn check(
        account: &AccountInfo,
        authority: &AccountInfo,
        mint: &AccountInfo,
    ) -> Result<(), ProgramError>;
}

pub trait AssociatedTokenAccountInit {
    fn init(
        ata: &AccountInfo,
        mint: &AccountInfo,
        authority: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> Result<(), ProgramError>;

    fn init_if_needed(
        ata: &AccountInfo,
        mint: &AccountInfo,
        authority: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo
    ) -> Result<(), ProgramError>;
}

pub struct AssociatedTokenAccount;

impl AssociatedTokenAccountCheck for AssociatedTokenAccount {
    fn check(
        account: &AccountInfo,
        authority: &AccountInfo,
        mint: &AccountInfo,
    ) -> Result<(), ProgramError> {
        TokenAccount::check(account)?;

        let seeds : &[&[u8]] = &[authority.key(), &pinocchio_token::ID,mint.key()];

        if find_program_address(seeds, &pinocchio_associated_token_account::ID).0.ne(account.key()) {
            return Err(ProgramError::InvalidAccountData);
        }
        
        Ok(())
    }
}

impl AssociatedTokenAccountInit for AssociatedTokenAccount {
    fn init(
        ata: &AccountInfo,
        mint: &AccountInfo,
        authority: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> Result<(), ProgramError> {
        pinocchio_associated_token_account::instructions::Create {
            funding_account: authority,
            account: ata,
            wallet: owner,
            mint,
            system_program: system_program,
            token_program: token_program,
        }.invoke()?;
        
        Ok(())
    }
    fn init_if_needed(
            ata: &AccountInfo,
            mint: &AccountInfo,
            authority: &AccountInfo,
            owner: &AccountInfo,
            system_program: &AccountInfo,
            token_program: &AccountInfo
        ) -> Result<(), ProgramError> {
            match Self::check(ata, authority, mint) {
            Ok(_) => Ok(()),
            Err(_) => Self::init(ata, mint, authority, owner, system_program, token_program)
        }
    }
}