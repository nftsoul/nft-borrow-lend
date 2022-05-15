use crate::{
    error::TokenError,
    instruction::{
        TokenInstruction,
        ProcessOffer,
        ProcessInterest,
    },
    utils::{generate_pda_and_bump_seed,},
    NFTPREFIX,
    state::{NftDetails,}
};
use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::{
    account_info::{AccountInfo,next_account_info},
    program_error::{PrintProgramError,ProgramError},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    program::{invoke,},
    system_instruction,
    pubkey::Pubkey,
    sysvar::{rent::Rent,Sysvar,clock::Clock},
    msg,
};
use spl_associated_token_account::get_associated_token_address;
use num_traits::FromPrimitive;
/// Program state handler.
pub struct Processor {}
impl Processor {
    pub fn process_deposit_nft(program_id: &Pubkey,accounts: &[AccountInfo],)-> ProgramResult {
        //depositing the NFT
        let account_info_iter = &mut accounts.iter();
        let nft_owner =  next_account_info(account_info_iter)?; // sender or signer
        let nft_mint = next_account_info(account_info_iter)?;  // mint address of nft
        let nft_owner_nft_associated = next_account_info(account_info_iter)?;  // nft owner nft id token account address
        let token_program_id = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let pda = next_account_info(account_info_iter)?; // pda data 
        let nft_vault = next_account_info(account_info_iter)?;  // nft vault address from NFTPREFIX, nft_owner, pda and program id
        let nft_associated_address = next_account_info(account_info_iter)?; // address generated from nft_vault_address and nft mint address token account address
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let rent_info  = next_account_info(account_info_iter)?; // rent 
        let system_program = next_account_info(account_info_iter)?; //system program
        let whitelist_info =next_account_info(account_info_iter)?;

        //verifying the collection
       //checking if the owner is the signer or not
        if !nft_owner.is_signer
        {
            return Err(ProgramError::MissingRequiredSignature);
        }
        //finding nft token account
        let nft_token_address=get_associated_token_address(nft_owner.key,nft_mint.key);

        //verifying nft token address
        if nft_token_address!=*nft_owner_nft_associated.key
        {
            return Err(ProgramError::MissingRequiredSignature);
        }
        
        //nft_vault where nft is stored
        let (nft_vault_address, bump_seed) = generate_pda_and_bump_seed(
            NFTPREFIX,
            nft_owner.key,
            pda.key,
            program_id
        );
         //nft_vault where nft is stored
        if nft_vault_address!=*nft_vault.key
        {
            return Err(ProgramError::MissingRequiredSignature);
        }
        //signer seeds for nft_vault
        let nft_vault_signer_seeds: &[&[_]] = &[
            NFTPREFIX.as_bytes(),
            &nft_owner.key.to_bytes(),
            &pda.key.to_bytes(),
            &[bump_seed],
        ];
         //finding nft token account
         let vault_nft_token_address=get_associated_token_address(nft_vault.key,nft_mint.key);

         //verifying nft token address
         if vault_nft_token_address!=*nft_associated_address.key
         {
             return Err(ProgramError::MissingRequiredSignature);
         }
         
         //rent account
        let rent = Rent::get()?;
        let transfer_amount =  rent.minimum_balance(std::mem::size_of::<NftDetails>());
        invoke(
            &system_instruction::create_account(
                nft_owner.key,
                pda.key,
                transfer_amount,
                std::mem::size_of::<NftDetails>() as u64,
               program_id,
            ),
            &[
                nft_owner.clone(),
                pda.clone(),
                system_program.clone(),
            ],
        )?;    
        // nft owner associated token using spl token mint
      
        if nft_associated_address.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    nft_owner.key,
                    nft_vault.key,
                    nft_mint.key,
                ),&[
                    nft_owner.clone(),
                    nft_associated_address.clone(),
                    nft_vault.clone(),
                    nft_mint.clone(),
                    token_program_id.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?;

        }
        msg!("transfer");
        invoke(
            &spl_token::instruction::transfer(
                token_program_id.key,
                nft_owner_nft_associated.key,
                nft_associated_address.key,
                nft_owner.key,
                &[nft_owner.key],
                1
            )?,
            &[
                token_program_id.clone(),
                nft_owner_nft_associated.clone(),
                nft_associated_address.clone(),
                nft_owner.clone(),
                system_program.clone()
            ],
        )?;
        let now = Clock::get()?.unix_timestamp as u64; 
        let mut escrow = NftDetails::try_from_slice(&pda.data.borrow())?;
        escrow.nft_mint=*nft_mint.key;
        escrow.nft_owner=*nft_owner.key;
        escrow.serialize(&mut &mut pda.data.borrow_mut()[..])?;
        
        Ok(())
    }

    pub fn process_offer(program_id: &Pubkey,accounts: &[AccountInfo], price:u64)->ProgramResult{   
        //Program to auction
        let account_info_iter = &mut accounts.iter();
        let bidder =  next_account_info(account_info_iter)?; // sender or signer
        let nft_owner = next_account_info(account_info_iter)?; // auction creator
        let pda_data = next_account_info(account_info_iter)?; // pda data that consists number of tokens , auction created
        let nft_vault = next_account_info(account_info_iter)?; // nft vault which saves the amount 
        let auction_data = next_account_info(account_info_iter)?; //account made using Auction Prefix, Nft owner and Day
        let system_program = next_account_info(account_info_iter)?;//system_program
        let rent_info  = next_account_info(account_info_iter)?; // rent 

       
       Ok(())

    }
    pub fn process_select(program_id: &Pubkey,accounts: &[AccountInfo],)-> ProgramResult {
        //program to buy nft at the price set by the program initiator
        let account_info_iter = &mut accounts.iter();
        let buyer =  next_account_info(account_info_iter)?; // sender or signer
        let nft_owner = next_account_info(account_info_iter)?; // auction creator
        let pda_data = next_account_info(account_info_iter)?; // pda data that consists number of tokens 
        let token_program_id = next_account_info(account_info_iter)?; //TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let nft_vault = next_account_info(account_info_iter)?; // nft vault
        let spl_vault_associated_address = next_account_info(account_info_iter)?;  // find associated address from nft vault and spl token mint
        let buyer_spl_associated =  next_account_info(account_info_iter)?; // sender or signer
        let spl_token_mint = next_account_info(account_info_iter)?; 
        let rent_info = next_account_info(account_info_iter)?; 
        let associated_token_info = next_account_info(account_info_iter)?; 
        let system_program = next_account_info(account_info_iter)?;
        Ok(())
    }
    pub fn process_cancel(program_id: &Pubkey,accounts: &[AccountInfo],)-> ProgramResult {
        //The winner of the auction can claim the tokens
        let account_info_iter = &mut accounts.iter();
        let buyer =  next_account_info(account_info_iter)?; // sender or signer
        let nft_owner = next_account_info(account_info_iter)?; // auction creator
        let pda_data = next_account_info(account_info_iter)?; // pda data that consists number of tokens , auction created
        let token_program_id = next_account_info(account_info_iter)?; //TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let nft_vault = next_account_info(account_info_iter)?; // nft vault
        let spl_vault_associated_address = next_account_info(account_info_iter)?;  // find associated address from nft vault and spl token mint
        let buyer_spl_associated =  next_account_info(account_info_iter)?; // sender or signer
        let spl_token_mint = next_account_info(account_info_iter)?; // spl token mint
        let system_program = next_account_info(account_info_iter)?; 
        let rent_info =next_account_info(account_info_iter)?; 
        let auction_data=next_account_info(account_info_iter)?;
        let associated_token_info= next_account_info(account_info_iter)?;

         //verifying pda_data
         if pda_data.owner!=program_id
         {
             return Err(ProgramError::MissingRequiredSignature);
         }
  
        Ok(())
    }
    pub fn process_interest(program_id: &Pubkey,accounts: &[AccountInfo],amount:u64)-> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let player =  next_account_info(account_info_iter)?; // sender or signer
        let coinflip_pda = next_account_info(account_info_iter)?; // pda data that consists number of tokens , auction created
        let token_program_id = next_account_info(account_info_iter)?; //TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let spl_vault_associated_address = next_account_info(account_info_iter)?;  // find associated address from nft vault and spl token mint
        let player_associated_token = next_account_info(account_info_iter)?; // spl token mint associate account
        let spl_token_mint = next_account_info(account_info_iter)?; // spl token mint
        let nft_owner = next_account_info(account_info_iter)?; // auction creator
        let system_program = next_account_info(account_info_iter)?; 
        let pda =next_account_info(account_info_iter)?;  // main data account
        let nft_vault = next_account_info(account_info_iter)?; // nft vault

         Ok(())
    }
    pub fn process_lending(program_id: &Pubkey,accounts: &[AccountInfo],)-> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let player =  next_account_info(account_info_iter)?; // sender or signer
        let nft_owner = next_account_info(account_info_iter)?; // auction creator
        let pda = next_account_info(account_info_iter)?; // pda data that consists number of tokens , auction created
        let coinflip_pda = next_account_info(account_info_iter)?; // pda data for coinflip
        let token_program_id = next_account_info(account_info_iter)?; //TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let nft_vault = next_account_info(account_info_iter)?; // nft vault
        let spl_vault_associated_address = next_account_info(account_info_iter)?;  // find associated address from nft vault and spl token mint
        let buyer_spl_associated =  next_account_info(account_info_iter)?; // sender or signer
        let spl_token_mint=next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?; 
        let rent_info=next_account_info(account_info_iter)?; 

 
        Ok(())
    }
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = TokenInstruction::unpack(input)?;
        match instruction {
            TokenInstruction::ProcessDeposit => {
                msg!("Instruction: Deposit NFT");
                Self::process_deposit_nft(program_id,accounts,)
            }
            TokenInstruction::ProcessOffer(ProcessOffer{amount}) => {
                msg!("Instruction:  Offer");
                Self::process_offer(program_id, accounts, amount)
            }
            TokenInstruction::ProcessSelection => {
                msg!("Instruction:  Selection");
                Self::process_select(program_id,accounts)
            }
            TokenInstruction::ProcessCancel => {
                msg!("Instruction:  Cancel");
                Self::process_cancel(program_id,accounts)
            }
            TokenInstruction::ProcessInterest(ProcessInterest{amount}) => {
                msg!("Instruction:  Pay Interest");
                Self::process_interest(program_id, accounts, amount)
            }
            TokenInstruction::ProcessLender => {
                msg!("Instruction:  Lender Action");
                Self::process_lending(program_id,accounts)
            }
            TokenInstruction::ProcessWhitelist => {
                msg!("Instruction:  Whitelist Collection");
                Self::process_lending(program_id,accounts)
            }
           
    }
}
}
impl PrintProgramError for TokenError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            TokenError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            TokenError::InvalidInstruction => msg!("Error: Invalid instruction"),
            TokenError::Overflow => msg!("Error: Token Overflow"),
            TokenError::Notstarted =>msg!("Error: Not started"),
            TokenError::TokenFinished =>msg!("Error: Token Finished"),

        }
    }
}