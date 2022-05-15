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

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct CoinFlip{
    pub won: u64,
    pub address: Pubkey,
    pub amount: u64,
}
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Auction{
    pub max_price: u64,
    pub max_payer: Pubkey,
    pub num_tokens: u64,
    pub day:u64,
}