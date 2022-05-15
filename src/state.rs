//! State transition types
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
};


/// Initializeing solana stream states
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct NftDetails{
    pub nft_mint: Pubkey,
    pub nft_owner:Pubkey,
    pub loan_start: u64,
    pub loan_amt: u64,
}
