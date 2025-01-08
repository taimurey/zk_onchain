use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Incorrect Parameter provided")]
    InvalidParameter,
    #[msg("Invalid authority")]
    InvalidAuthority,
    #[msg("Account Not Authorized")]
    InvalidSigner,
    #[msg("Signer doesn't exist")]
    InvalidSignerExist,
    #[msg("Too many service signers")]
    TooManyServiceSigners,
    #[msg("No authority to perform this action")]
    Unauthorized,
    #[msg("Service signer already exists")]
    DuplicateServiceSigner,
    #[msg("Invalid compression params provided")]
    InvalidCompressedParams,
    #[msg("Invalid Server Id")]
    InvalidServerId,
    #[msg("Invalid Server name")]
    InvalidServerName,
}
