//! State transition types
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
    program_error::ProgramError,
    account_info::AccountInfo,
    borsh::try_from_slice_unchecked,
};


/// Initializeing solana stream states
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct NftDetails{
    pub nft_mint: Pubkey,
    pub nft_owner:Pubkey,
    pub loan_start: u64,
    pub loan_amt: u64,
    pub lender: Pubkey,
    pub loan_taken: bool,
    pub loan_offered:bool,
    pub canceled: bool,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Whitelist {
    pub producer: Vec<Pubkey>,
    pub state: bool,
    pub daily_interest_rate: u64,
}
impl Whitelist {
    pub fn from_account(account:&AccountInfo)-> Result<Whitelist, ProgramError> {
            let md: Whitelist =try_from_slice_unchecked(&account.data.borrow_mut())?;
            Ok(md)
    }
}
