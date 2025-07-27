use pinocchio::pubkey::Pubkey;

#[repr(C)]
pub struct Pool {
    pub authority: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub fee_rate: u16,
    pub bump: u8,
    pub lp_mint_bump: u8,
}

impl Pool {
    pub const LEN: usize = 32 + 32 + 32 + 32 + 32 + 32 + 2 + 1 + 1; // 6 Pubkeys + u16 + 2 u8s

    pub fn set_inner_full(
        &mut self,
        authority: Pubkey,
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        token_a_vault: Pubkey,
        token_b_vault: Pubkey,
        lp_mint: Pubkey,
        fee_rate: u16,
        bump: u8,
        lp_mint_bump: u8,
    ) {
        self.authority = authority;
        self.token_a_mint = token_a_mint;
        self.token_b_mint = token_b_mint;
        self.token_a_vault = token_a_vault;
        self.token_b_vault = token_b_vault;
        self.lp_mint = lp_mint;
        self.fee_rate = fee_rate;
        self.bump = bump;
        self.lp_mint_bump = lp_mint_bump;
    }

    pub fn load_mut(data: &mut [u8]) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        
        let pool = unsafe { &mut *(data.as_mut_ptr() as *mut Self) };
        Ok(pool)
    }
}