//! Instruction types
use solana_program::{
    program_error::ProgramError,
    msg
};


use crate::{
    error::TokenError,
};
use std::convert::TryInto;

pub struct ProcessOffer{
    pub amount: u64,
}
pub struct ProcessInterest{
    pub amount: u64,
}

pub enum TokenInstruction {
    ProcessDeposit, ///0
    ProcessOffer(ProcessOffer), ///1
    ProcessSelection, ///2
    ProcessCancel,///3
    ProcessInterest(ProcessInterest),///4
    ProcessLender,
    ProcessWhitelist,
}
impl TokenInstruction {
    /// Unpacks a byte buffer into a [TokenInstruction](enum.TokenInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        use TokenError::InvalidInstruction;
        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        msg!("{:?}",input);
        Ok(match tag {
            // Initialize deposit NFT instruction 
            0 => {
                Self::ProcessDeposit
            }
            1 => {
                let (amount, rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessOffer(ProcessOffer{amount})
            }
            2 => {
                Self::ProcessSelection
            }
            3 => {
                msg!("{:?}",rest);
                Self::ProcessCancel
            }
            4 => {
                let (amount, rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessInterest(ProcessInterest{amount})
            }
            5 => {
                Self::ProcessLender
            }
            6 =>{
                Self::ProcessWhitelist
            }

            _ => return Err(TokenError::InvalidInstruction.into()),
        })
    }
}