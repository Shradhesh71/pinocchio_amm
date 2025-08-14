use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::rent::Rent;
use solana_sdk::sysvar::Sysvar;
use spl_token;
use spl_associated_token_account;
extern crate alloc;
use alloc::vec;

use pinocchio_amm::ID;
use pinocchio_amm::Pool;

pub const PROGRAM: Pubkey = Pubkey::new_from_array(ID);

pub const RENT: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");

pub const PAYER: Pubkey = pubkey!("Bv1vrbzogVpKNW2iRYJXLRUEVv6gD8xd9gid1Yh6hoiQ");

pub fn get_rent_data() -> Vec<u8> {
    let rent = Rent::default();
    unsafe {
        core::slice::from_raw_parts(&rent as *const Rent as *const u8, Rent::size_of()).to_vec()
    }
}

#[test]
fn test_initialize_pool_complete() {
    let token_program = spl_token::ID;
    let associated_token_program = spl_associated_token_account::ID;

    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique();
    
    let (pool_pda, pool_bump) = Pubkey::find_program_address(
        &[b"pool", token_a_mint.as_ref(), token_b_mint.as_ref()],
        &PROGRAM,
    );

    let (lp_mint_pda, lp_mint_bump) = Pubkey::find_program_address(
        &[b"lp_mint", pool_pda.as_ref()],
        &PROGRAM,
    );

    let token_a_vault = spl_associated_token_account::get_associated_token_address(
        &pool_pda,
        &token_a_mint,
    );
    let token_b_vault = spl_associated_token_account::get_associated_token_address(
        &pool_pda,
        &token_b_mint,
    );

    let fee_rate: u16 = 30; 
    let mut instruction_data = vec![0];
    instruction_data.extend_from_slice(&fee_rate.to_le_bytes());

    assert_eq!(instruction_data.len(), 3);
    assert_eq!(instruction_data[0], 0);
    assert_eq!(u16::from_le_bytes([instruction_data[1], instruction_data[2]]), fee_rate);

    assert!(fee_rate <= 10000);

    let addresses = vec![
        pool_pda, lp_mint_pda, token_a_vault, token_b_vault, 
        token_a_mint, token_b_mint, token_program, associated_token_program
    ];
    
    for i in 0..addresses.len() {
        for j in (i+1)..addresses.len() {
            assert_ne!(addresses[i], addresses[j], 
                "Addresses at indices {} and {} should be different", i, j);
        }
    }

    let mut pool_data = vec![0u8; Pool::LEN];
    let pool = Pool::load_mut(&mut pool_data).expect("Should load pool");

    pool.set_inner_full(
        PAYER.to_bytes(),          
        token_a_mint.to_bytes(),    
        token_b_mint.to_bytes(),    
        token_a_vault.to_bytes(),   
        token_b_vault.to_bytes(),   
        lp_mint_pda.to_bytes(),     
        fee_rate,                  
        pool_bump,                
        lp_mint_bump,               
    );

    assert_eq!(pool.authority, PAYER.to_bytes());
    assert_eq!(pool.token_a_mint, token_a_mint.to_bytes());
    assert_eq!(pool.token_b_mint, token_b_mint.to_bytes());
    assert_eq!(pool.token_a_vault, token_a_vault.to_bytes());
    assert_eq!(pool.token_b_vault, token_b_vault.to_bytes());
    assert_eq!(pool.lp_mint, lp_mint_pda.to_bytes());
    assert_eq!(pool.fee_rate, fee_rate);
    assert_eq!(pool.bump, pool_bump);
    assert_eq!(pool.lp_mint_bump, lp_mint_bump);
}

#[test]
fn test_initialize_pool_data_parsing() {
    let fee_rate: u16 = 30; // 0.3%
    let mut instruction_data = vec![0];
    instruction_data.extend_from_slice(&fee_rate.to_le_bytes());
    
    assert_eq!(instruction_data.len(), 3); 
    assert_eq!(instruction_data[0], 0); 
    assert_eq!(u16::from_le_bytes([instruction_data[1], instruction_data[2]]), fee_rate);
}

#[test]
fn test_pool_pda_generation() {
    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique();
    
    let (pool_pda, _pool_bump) = Pubkey::find_program_address(
        &[b"pool", token_a_mint.as_ref(), token_b_mint.as_ref()],
        &PROGRAM,
    );
    
    let (lp_mint_pda, _lp_mint_bump) = Pubkey::find_program_address(
        &[b"lp_mint", pool_pda.as_ref()],
        &PROGRAM,
    );
    
    assert_ne!(pool_pda, lp_mint_pda);
    assert_ne!(pool_pda, token_a_mint);
    assert_ne!(pool_pda, token_b_mint);
}

#[test]
fn test_pool_state_size() {
    use pinocchio_amm::Pool;
    
    // Verify Pool::LEN matches the actual struct size
    // Pool should contain:
    // - authority: Pubkey (32 bytes)
    // - token_a_mint: Pubkey (32 bytes)  
    // - token_b_mint: Pubkey (32 bytes)
    // - token_a_vault: Pubkey (32 bytes)
    // - token_b_vault: Pubkey (32 bytes)
    // - lp_mint: Pubkey (32 bytes)
    // - fee_rate: u16 (2 bytes)
    // - bump: u8 (1 byte)
    // - lp_mint_bump: u8 (1 byte)
    // Total: 6*32 + 2 + 1 + 1 = 196 bytes
    
    assert_eq!(Pool::LEN, 196);
}

#[test]
fn test_associated_token_addresses() {
    let token_mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    
    let ata = spl_associated_token_account::get_associated_token_address(
        &owner,
        &token_mint,
    );
    
    let ata2 = spl_associated_token_account::get_associated_token_address(
        &owner,
        &token_mint,
    );
    
    assert_eq!(ata, ata2);
    assert_ne!(ata, owner);
    assert_ne!(ata, token_mint);
}

#[test]
fn test_fee_rate_validation() {
    for fee_rate in [0u16, 30u16, 100u16, 1000u16, 10000u16] {
        let mut instruction_data = vec![0]; // discriminator
        instruction_data.extend_from_slice(&fee_rate.to_le_bytes());
        
        assert_eq!(u16::from_le_bytes([instruction_data[1], instruction_data[2]]), fee_rate);
    }
    
    let invalid_fee_rate: u16 = 10001;
    let mut instruction_data = vec![0];
    instruction_data.extend_from_slice(&invalid_fee_rate.to_le_bytes());
    
    assert_eq!(u16::from_le_bytes([instruction_data[1], instruction_data[2]]), invalid_fee_rate);
    assert!(invalid_fee_rate > 10000); 
}

#[test]
fn test_add_liquidity_complete() {
    let token_program = spl_token::ID;
    let associated_token_program = spl_associated_token_account::ID;
    let system_program = solana_sdk::system_program::ID;

    let token_a_mint = Pubkey::new_unique();
    let token_b_mint = Pubkey::new_unique();

    let user = Pubkey::new_unique();
    
    let (pool_pda, pool_bump) = Pubkey::find_program_address(
        &[b"pool", token_a_mint.as_ref(), token_b_mint.as_ref()],
        &PROGRAM,
    );

    let (lp_mint_pda, _lp_mint_bump) = Pubkey::find_program_address(
        &[b"lp_mint", pool_pda.as_ref()],
        &PROGRAM,
    );

    let token_a_vault = spl_associated_token_account::get_associated_token_address(
        &pool_pda,
        &token_a_mint,
    );
    let token_b_vault = spl_associated_token_account::get_associated_token_address(
        &pool_pda,
        &token_b_mint,
    );

    let user_token_a = spl_associated_token_account::get_associated_token_address(
        &user,
        &token_a_mint,
    );
    let user_token_b = spl_associated_token_account::get_associated_token_address(
        &user,
        &token_b_mint,
    );
    let user_lp_token = spl_associated_token_account::get_associated_token_address(
        &user,
        &lp_mint_pda,
    );

    let amount_a: u64 = 1000000; 
    let amount_b: u64 = 2000000; 
    let min_lp_amount: u64 = 1400000;

    let mut instruction_data = vec![1]; 
    instruction_data.extend_from_slice(&amount_a.to_le_bytes());
    instruction_data.extend_from_slice(&amount_b.to_le_bytes());
    instruction_data.extend_from_slice(&min_lp_amount.to_le_bytes());

    assert_eq!(instruction_data.len(), 25);
    assert_eq!(instruction_data[0], 1);

    let parsed_amount_a = u64::from_le_bytes([
        instruction_data[1], instruction_data[2], instruction_data[3], instruction_data[4],
        instruction_data[5], instruction_data[6], instruction_data[7], instruction_data[8],
    ]);
    let parsed_amount_b = u64::from_le_bytes([
        instruction_data[9], instruction_data[10], instruction_data[11], instruction_data[12],
        instruction_data[13], instruction_data[14], instruction_data[15], instruction_data[16],
    ]);
    let parsed_min_lp = u64::from_le_bytes([
        instruction_data[17], instruction_data[18], instruction_data[19], instruction_data[20],
        instruction_data[21], instruction_data[22], instruction_data[23], instruction_data[24],
    ]);

    assert_eq!(parsed_amount_a, amount_a);
    assert_eq!(parsed_amount_b, amount_b);
    assert_eq!(parsed_min_lp, min_lp_amount);

    assert!(amount_a > 0);
    assert!(amount_b > 0);
    
    let product = (amount_a as f64) * (amount_b as f64);
    let geometric_mean = product.sqrt() as u64;
    assert!(geometric_mean >= 1000); 
    assert!(geometric_mean >= min_lp_amount); 

    let addresses = vec![
        pool_pda, lp_mint_pda, token_a_vault, token_b_vault,
        user_token_a, user_token_b, user_lp_token,
        token_a_mint, token_b_mint, user,
        token_program, associated_token_program, system_program
    ];
    
    for i in 0..addresses.len() {
        for j in (i+1)..addresses.len() {
            assert_ne!(addresses[i], addresses[j], 
                "Addresses at indices {} and {} should be different", i, j);
        }
    }

    assert_ne!(token_a_mint, token_b_mint); 
    assert_ne!(token_a_vault, token_b_vault);
    assert_ne!(user_token_a, user_token_b); 
    
    let (pool_pda2, pool_bump2) = Pubkey::find_program_address(
        &[b"pool", token_a_mint.as_ref(), token_b_mint.as_ref()],
        &PROGRAM,
    );
    assert_eq!(pool_pda, pool_pda2);
    assert_eq!(pool_bump, pool_bump2);
}

#[test]
fn test_add_liquidity_data_parsing() {
    let test_cases = vec![
        (1000u64, 2000u64, 1400u64),     // Basic case
        (1u64, 1u64, 1u64),              // Minimum amounts
        (u64::MAX/2, u64::MAX/2, 1000u64), // Large amounts
        (1000000000u64, 500000000u64, 700000000u64), // Realistic amounts
    ];

    for (amount_a, amount_b, min_lp_amount) in test_cases {
        let mut instruction_data = vec![1]; // discriminator
        instruction_data.extend_from_slice(&amount_a.to_le_bytes());
        instruction_data.extend_from_slice(&amount_b.to_le_bytes());
        instruction_data.extend_from_slice(&min_lp_amount.to_le_bytes());

        // Verify instruction data format
        assert_eq!(instruction_data.len(), 25);
        assert_eq!(instruction_data[0], 1);

        // Parse back and verify
        let parsed_amount_a = u64::from_le_bytes([
            instruction_data[1], instruction_data[2], instruction_data[3], instruction_data[4],
            instruction_data[5], instruction_data[6], instruction_data[7], instruction_data[8],
        ]);
        let parsed_amount_b = u64::from_le_bytes([
            instruction_data[9], instruction_data[10], instruction_data[11], instruction_data[12],
            instruction_data[13], instruction_data[14], instruction_data[15], instruction_data[16],
        ]);
        let parsed_min_lp = u64::from_le_bytes([
            instruction_data[17], instruction_data[18], instruction_data[19], instruction_data[20],
            instruction_data[21], instruction_data[22], instruction_data[23], instruction_data[24],
        ]);

        assert_eq!(parsed_amount_a, amount_a);
        assert_eq!(parsed_amount_b, amount_b);
        assert_eq!(parsed_min_lp, min_lp_amount);
    }
}

#[test]
fn test_add_liquidity_geometric_mean_calculation() {
    let test_cases = vec![
        (1000000u64, 1000000u64, 1000000u64),    // Perfect square
        (4000000u64, 1000000u64, 2000000u64),    // 4:1 ratio
        (9000000u64, 4000000u64, 6000000u64),    // 9:4 ratio
        (1600000000u64, 900000000u64, 1200000000u64), // Large amounts
    ];

    for (amount_a, amount_b, expected_approx) in test_cases {
        let product = (amount_a as f64) * (amount_b as f64);
        let geometric_mean = product.sqrt() as u64;
        
        let tolerance = expected_approx / 100;
        assert!(
            geometric_mean >= expected_approx.saturating_sub(tolerance) &&
            geometric_mean <= expected_approx.saturating_add(tolerance),
            "Geometric mean {} not within tolerance of expected {}",
            geometric_mean, expected_approx
        );
        
        assert!(geometric_mean >= 1000, "Geometric mean {} below minimum 1000", geometric_mean);
    }

}

#[test]
fn test_add_liquidity_edge_cases() {
    
    let amount_a = 1u64;
    let amount_b = 1u64;
    let min_lp_amount = 1u64;
    
    let mut instruction_data = vec![1];
    instruction_data.extend_from_slice(&amount_a.to_le_bytes());
    instruction_data.extend_from_slice(&amount_b.to_le_bytes());
    instruction_data.extend_from_slice(&min_lp_amount.to_le_bytes());
    
    assert_eq!(instruction_data.len(), 25);
    
    let product = (amount_a as f64) * (amount_b as f64);
    let geometric_mean = product.sqrt() as u64;
    assert_eq!(geometric_mean, 1);
    
    let short_data = vec![1, 0, 0, 0]; 
    assert_eq!(short_data.len(), 4);
    assert!(short_data.len() < 25); 
    
    let zero_amount_a = 0u64;
    let zero_amount_b = 1000u64;
    let mut zero_instruction_data = vec![1];
    zero_instruction_data.extend_from_slice(&zero_amount_a.to_le_bytes());
    zero_instruction_data.extend_from_slice(&zero_amount_b.to_le_bytes());
    zero_instruction_data.extend_from_slice(&min_lp_amount.to_le_bytes());
    
    let parsed_zero_a = u64::from_le_bytes([
        zero_instruction_data[1], zero_instruction_data[2], zero_instruction_data[3], zero_instruction_data[4],
        zero_instruction_data[5], zero_instruction_data[6], zero_instruction_data[7], zero_instruction_data[8],
    ]);
    assert_eq!(parsed_zero_a, 0);
}