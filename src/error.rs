use pinocchio::program_error::ProgramError;

impl From<PinocchioError> for ProgramError{
    fn from(e: PinocchioError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub enum PinocchioError{
    InvalidAmount = 0x0,
    SlippageExceeded = 0x1,
    InsufficientLiquidity = 0x2,
    PoolAlreadyInitialized = 0x3,

    InvalidFeeRate = 0x4,
    MathOverflow = 0x5,
    InvalidTokenMint = 0x6,
    Unauthorized = 0x7,
    InvalidPoolState = 0x8,
    IdenticalMints = 0x9,
}

impl PinocchioError {
    pub fn discription(&self) -> &'static str {
        match self {
            PinocchioError::InvalidAmount => "Invalid amount provided",
            PinocchioError::SlippageExceeded => "Slippage exceeded the allowed limit",
            PinocchioError::InsufficientLiquidity => "Insufficient liquidity in the pool",
            PinocchioError::PoolAlreadyInitialized => "The pool is already initialized",
            PinocchioError::InvalidFeeRate => "Invalid fee rate provided",
            PinocchioError::MathOverflow => "Math operation resulted in overflow",
            PinocchioError::InvalidTokenMint => "Invalid token mint provided",
            PinocchioError::Unauthorized => "Unauthorized access",
            PinocchioError::InvalidPoolState => "The pool is in an invalid state",
            PinocchioError::IdenticalMints => "Cannot swap between identical mints",
        }
    }
}