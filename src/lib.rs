#[cfg(not(test))]
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult};

#[cfg(not(test))]
use pinocchio::entrypoint;

#[cfg(not(test))]
entrypoint!(process_instruction);

pub mod error;
pub mod states;
pub mod instructions;

pub use instructions::{
    SignerAccount, MintInterface, AccountCheck, ProgramAccount, ProgramAccountInit,
    AssociatedTokenAccount, AssociatedTokenAccountCheck, AssociatedTokenAccountInit,
};
pub use states::Pool;
pub use error::PinocchioError;

#[cfg(not(test))]
use crate::instructions::{AddLiquidity, InitializePool, RemoveLiquidity, Swap};

// pub const ID: Pubkey = [
//     0x1f, 0x2e, 0x3d, 0x4c, 0x5b, 0x6a, 0x7b, 0x8c,
//     0x9d, 0xae, 0xbf, 0xc0, 0xd1, 0xe2, 0xf3, 0x04,
//     0x15, 0x26, 0x37, 0x48, 0x59, 0x6a, 0x7b, 0x8c,
//     0x9d, 0xae, 0xbf, 0xc1, 0xd2, 0xe3, 0xf4, 0x05,
// ];
pinocchio_pubkey::declare_id!("jpJB1eJKD1rzvMkchc8Czzx8yx1wxJYBe3uDdUVF99K");

#[cfg(not(test))]
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.split_first() {
        Some((InitializePool::DISCRIMINATOR, _)) =>  InitializePool::try_from((accounts, instruction_data))?.process()?,
        Some((AddLiquidity::DISCRIMINATOR, _)) =>  AddLiquidity::try_from((accounts, instruction_data))?.process()?,
        Some((RemoveLiquidity::DISCRIMINATOR, _)) => RemoveLiquidity::try_from((accounts,instruction_data))?.process()?,
        Some((Swap::DISCRIMINATOR, _)) => Swap::try_from((accounts, instruction_data))?.process()?,
        _ => Err(ProgramError::InvalidInstructionData)?,
    }
    Ok(())
}
