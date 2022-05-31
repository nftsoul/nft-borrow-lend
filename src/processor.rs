use crate::{
    error::TokenError,
    instruction::{
        TokenInstruction,
        ProcessOffer,
        ProcessInterest,
        ProcessWhitelist,
        ProcessUpdate,
    },
    utils::{generate_pda_and_bump_seed,derive_whitelist_address,get_token_balance},
    NFTPREFIX,WHITELIST,
    state::{NftDetails,Whitelist}
};
use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::{
    account_info::{AccountInfo,next_account_info},
    program_error::{PrintProgramError,ProgramError},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    program::{invoke,invoke_signed},
    system_instruction,
    pubkey::Pubkey,
    sysvar::{rent::Rent,Sysvar,clock::Clock},
    msg,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token_metadata::state::{Metadata,Creator};
use num_traits::FromPrimitive;
use mokshyafeed::collection_price;


const METAPLEX_PROGRAM_ID: &'static str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
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
        let meta_data_account=next_account_info(account_info_iter)?;
        let whitelist_info =next_account_info(account_info_iter)?; //

        //metaplex program id
        let metaplex_pid=METAPLEX_PROGRAM_ID
        .parse::<Pubkey>()
        .expect("Failed to parse Metaplex Program Id");

        let seeds = &[
        "metadata".as_bytes(),
        metaplex_pid.as_ref(),
        nft_mint.key.as_ref(),];

        let (metadata_address, _) = Pubkey::find_program_address(seeds, &metaplex_pid);

        //verifying the collection
        if *meta_data_account.key!=metadata_address
        {
            msg!("The metadata account doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        let metadata=Metadata::from_account_info(meta_data_account)?;
        let mut creators: Vec<Creator> = Vec::new();
        if let Some(c) = metadata.data.creators 
        {
            creators = c
                .iter()
                .map(|c| Creator {
                                address: c.address,
                                verified: c.verified,
                                share: c.share,
                                }).collect::<Vec<Creator>>();
            }
        if whitelist_info.owner!=program_id
        {
            msg!("Whitelist Info is not owned by the program");
            return Err(ProgramError::MissingRequiredSignature);
        }
        let first_creator=creators[0].address;
        let (whitelist_address, _)=derive_whitelist_address(&first_creator, program_id);

        if whitelist_address!=*whitelist_info.key
        {
            msg!("Whitelist Info key doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        let wlist = Whitelist::from_account(whitelist_info)?;
        if wlist.state
        {
            msg!("The collection isn't initialized at all");
            return Err(ProgramError::MissingRequiredSignature);
        }
        for i in 0..creators.len()
        {
            if wlist.producer[i]!=creators[i].address
            {
                msg!("The creators doesn't match");
                return Err(ProgramError::MissingRequiredSignature);
            }
        }
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
        let _nft_vault_signer_seeds: &[&[_]] = &[
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
                &spl_associated_token_account::instruction::create_associated_token_account(
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
        //let now = Clock::get()?.unix_timestamp as u64; 
        let mut escrow = NftDetails::try_from_slice(&pda.data.borrow())?;
        escrow.nft_mint=*nft_mint.key;
        escrow.nft_owner=*nft_owner.key;
        escrow.canceled=false;
        escrow.serialize(&mut &mut pda.data.borrow_mut()[..])?;
        
        Ok(())
    }
    pub fn process_offer(program_id: &Pubkey,accounts: &[AccountInfo], amount:u64)->ProgramResult{   
        let account_info_iter = &mut accounts.iter();
        let lender =  next_account_info(account_info_iter)?; // sender or signer
        let nft_owner = next_account_info(account_info_iter)?; // auction creator
        let nft_vault = next_account_info(account_info_iter)?; // nft vault which saves the amount 
        let nft_mint =next_account_info(account_info_iter)?; // 
        let nft_associated_address = next_account_info(account_info_iter)?; // address generated from nft_vault_address and nft mint address token account address
        let system_program = next_account_info(account_info_iter)?;//system_program
        let mokshya_prm=next_account_info(account_info_iter)?; 
        let data_account = next_account_info(account_info_iter)?; 

        if !lender.is_signer
        {
            msg!("The lender isn't signer");
            return Err(ProgramError::MissingRequiredSignature);   
        }
        if data_account.owner!=program_id
        {
            msg!("The data_account isn't owned by the program");
            return Err(ProgramError::MissingRequiredSignature);
        }
        let mut data = NftDetails::try_from_slice(&data_account.data.borrow())?;
        if data.nft_owner!=*nft_owner.key
        {
            msg!("The NFT owner doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.nft_mint!=*nft_mint.key
        {
            msg!("The NFT owner doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.loan_offered 
        {
            msg!("The Loan is already offered");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.loan_taken
        {
            msg!("The Loan is already taken");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.canceled
        {
            msg!("The process is canceled");
            return Err(ProgramError::MissingRequiredSignature);
        }
         //nft_vault where nft is stored
         let (nft_vault_address, bump_seed) = generate_pda_and_bump_seed(
            NFTPREFIX,
            nft_owner.key,
            data_account.key,
            program_id
        );
         //nft_vault where nft is stored
        if nft_vault_address!=*nft_vault.key
        {
            return Err(ProgramError::MissingRequiredSignature);
        }
        //signer seeds for nft_vault
        let _nft_vault_signer_seeds: &[&[_]] = &[
            NFTPREFIX.as_bytes(),
            &nft_owner.key.to_bytes(),
            &data_account.key.to_bytes(),
            &[bump_seed],
        ];
          //finding nft token account
          let vault_nft_token_address=get_associated_token_address(nft_vault.key,nft_mint.key);

        //verifying nft token address
        if vault_nft_token_address!=*nft_associated_address.key
        {
            return Err(ProgramError::MissingRequiredSignature);
        }         
    
        let token_balance = get_token_balance(nft_associated_address)?;
        if token_balance!=1
        {
            msg!("The vault doesn't contain the specified NFT");
            return Err(ProgramError::MissingRequiredSignature);
        }

        msg!("The price of the nft collection is ");
        let price=collection_price(&mokshya_prm.key, data_account)?;
        msg!("{}",price);
        if amount < price/2
        {
            msg!("The amount offered is lower ");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if amount > price/2
        {
            msg!("The amount offered is higher only {} will be deducted ",price/2);   
        }
        invoke(
            &system_instruction::transfer(
                lender.key,
                nft_vault.key,
                price/2,
            ),
            &[
                lender.clone(),
                nft_vault.clone(),
                system_program.clone()
            ],
        )?;
        data.loan_amt=price/2;
        data.loan_offered=true;
        data.lender=*lender.key;
        data.serialize(&mut &mut data_account.data.borrow_mut()[..])?;
       
       Ok(())

    }
    pub fn process_select(program_id: &Pubkey,accounts: &[AccountInfo],)-> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let lender =  next_account_info(account_info_iter)?; // sender or signer
        let nft_owner = next_account_info(account_info_iter)?; //nft owner
        let nft_vault = next_account_info(account_info_iter)?; // nft vault which saves the amount 
        let nft_mint =next_account_info(account_info_iter)?; // 
        let nft_associated_address = next_account_info(account_info_iter)?; // address generated from nft_vault_address and nft mint address token account address
        let system_program = next_account_info(account_info_iter)?;//system_program
        let data_account = next_account_info(account_info_iter)?; 

        if !nft_owner.is_signer
        {
            msg!("The lender isn't signer");
            return Err(ProgramError::MissingRequiredSignature);   
        }
        if data_account.owner!=program_id
        {
            msg!("The data_account isn't owned by the program");
            return Err(ProgramError::MissingRequiredSignature);
        }
        let mut data = NftDetails::try_from_slice(&data_account.data.borrow())?;
        if data.nft_owner!=*nft_owner.key
        {
            msg!("The NFT owner doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.nft_mint!=*nft_mint.key
        {
            msg!("The NFT owner doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.lender!=*lender.key
        {
            msg!("The lender doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !data.loan_offered 
        {
            msg!("The Loan isn't offered at all");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.loan_taken
        {
            msg!("The Loan is already taken");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.canceled
        {
            msg!("The process is canceled");
            return Err(ProgramError::MissingRequiredSignature);
        }
         //nft_vault where nft is stored
         let (nft_vault_address, bump_seed) = generate_pda_and_bump_seed(
            NFTPREFIX,
            nft_owner.key,
            data_account.key,
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
            &data_account.key.to_bytes(),
            &[bump_seed],
        ];
          //finding nft token account
          let vault_nft_token_address=get_associated_token_address(nft_vault.key,nft_mint.key);

        //verifying nft token address
        if vault_nft_token_address!=*nft_associated_address.key
        {
            return Err(ProgramError::MissingRequiredSignature);
        }         
    
        let token_balance = get_token_balance(nft_associated_address)?;
        if token_balance!=1
        {
            msg!("The vault doesn't contain the specified NFT");
            return Err(ProgramError::MissingRequiredSignature);
        }
        invoke_signed(  
            &system_instruction::transfer(
            nft_vault.key,
            nft_owner.key,
            data.loan_amt,    
        ),
        &[
            nft_vault.clone(),
            nft_owner.clone(),
            system_program.clone(),
        ],
        &[&nft_vault_signer_seeds],
        )?;
        let now = Clock::get()?.unix_timestamp as u64; 
        data.loan_start = now;
        data.loan_taken=true;
        data.serialize(&mut &mut data_account.data.borrow_mut()[..])?;
        Ok(())
    }
    pub fn process_cancel(program_id: &Pubkey,accounts: &[AccountInfo],)-> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let nft_owner =  next_account_info(account_info_iter)?; // sender or signer
        let nft_mint = next_account_info(account_info_iter)?;  // mint address of nft
        let nft_owner_nft_associated = next_account_info(account_info_iter)?;  // nft owner nft id token account address
        let token_program_id = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let pda = next_account_info(account_info_iter)?; // pda data 
        let nft_vault = next_account_info(account_info_iter)?;  // nft vault address from NFTPREFIX, nft_owner, pda and program id
        let nft_associated_address = next_account_info(account_info_iter)?; // address generated from nft_vault_address and nft mint address token account address
        let system_program = next_account_info(account_info_iter)?; //system program
        
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
          if pda.owner!=program_id
          {
              msg!("The data_account isn't owned by the program");
              return Err(ProgramError::MissingRequiredSignature);
          }
          let mut data = NftDetails::try_from_slice(&pda.data.borrow())?;

          if data.nft_owner!=*nft_owner.key
          {
              msg!("The NFT owner doesn't match");
              return Err(ProgramError::MissingRequiredSignature);
          }
          if data.nft_mint!=*nft_mint.key
          {
              msg!("The NFT owner doesn't match");
              return Err(ProgramError::MissingRequiredSignature);
          }
          if data.loan_taken
           {
              msg!("The Loan is already taken, you need to pay interest to release the fund");
              return Err(ProgramError::MissingRequiredSignature);
           }
           let token_balance = get_token_balance(nft_associated_address)?;
           if token_balance!=1
           {
               msg!("The vault doesn't contain the specified NFT");
               return Err(ProgramError::MissingRequiredSignature);
            }
            //checking if someone has already offerred the loan or not
            if data.loan_offered
            {
                let lender = next_account_info(account_info_iter)?; //lender account
                if *lender.key!=data.lender
                {
                    msg!("The lender key doesn't match");
                    return Err(ProgramError::MissingRequiredSignature);
                }
                msg!("Releasing the fund of the lender");
                invoke_signed(  
                    &system_instruction::transfer(
                    nft_vault.key,
                    nft_owner.key,
                    data.loan_amt,    
                ),
                &[
                    nft_vault.clone(),
                    nft_owner.clone(),
                    system_program.clone(),
                ],
                &[&nft_vault_signer_seeds],
                )?;
            }
            //All conditions satisfied release the NFT
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_id.key,
                    nft_associated_address.key,
                    nft_owner_nft_associated.key,
                    nft_vault.key,
                    &[nft_vault.key],
                    1
                )?,
                &[
                    token_program_id.clone(),
                    nft_owner_nft_associated.clone(),
                    nft_associated_address.clone(),
                    nft_owner.clone(),
                    system_program.clone()
                ],
                &[&nft_vault_signer_seeds],
            )?;          
  
            data.canceled=true;
            data.serialize(&mut &mut pda.data.borrow_mut()[..])?;
        Ok(())
    }
    pub fn process_interest(program_id: &Pubkey,accounts: &[AccountInfo],amount:u64)-> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let nft_owner =  next_account_info(account_info_iter)?; // sender or signer
        let nft_mint = next_account_info(account_info_iter)?;  // mint address of nft
        let nft_owner_nft_associated = next_account_info(account_info_iter)?;  // nft owner nft id token account address
        let token_program_id = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let pda = next_account_info(account_info_iter)?; // pda data 
        let nft_vault = next_account_info(account_info_iter)?;  // nft vault address from NFTPREFIX, nft_owner, pda and program id
        let nft_associated_address = next_account_info(account_info_iter)?; // address generated from nft_vault_address and nft mint address token account address
        let system_program = next_account_info(account_info_iter)?; //system program
        let whitelist_info =next_account_info(account_info_iter)?; //
        let creator1=next_account_info(account_info_iter)?; //
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
          if pda.owner!=program_id
          {
              msg!("The data_account isn't owned by the program");
              return Err(ProgramError::MissingRequiredSignature);
          }
          let mut data = NftDetails::try_from_slice(&pda.data.borrow())?;

          if data.nft_owner!=*nft_owner.key
          {
              msg!("The NFT owner doesn't match");
              return Err(ProgramError::MissingRequiredSignature);
          }
          if data.nft_mint!=*nft_mint.key
          {
              msg!("The NFT owner doesn't match");
              return Err(ProgramError::MissingRequiredSignature);
          }
          if !data.loan_taken
           {
              msg!("The Loan isnot taken");
              return Err(ProgramError::MissingRequiredSignature);
           }
           let token_balance = get_token_balance(nft_associated_address)?;
           if token_balance!=1
            {
               msg!("The vault doesn't contain the specified NFT");
               return Err(ProgramError::MissingRequiredSignature);
            }
            let now = Clock::get()?.unix_timestamp as u64; 
            let days:u64 =(now-data.loan_start)/86400;
            if days >14
            {
                msg!("Your collateral NFT is taken by Platform You aren't allowed to withdraw");
                return Err(ProgramError::MissingRequiredSignature);
            }
            let first_creator=creator1.key;
            let (whitelist_address, _)=derive_whitelist_address(first_creator, program_id);

            if whitelist_address!=*whitelist_info.key
            {
                msg!("Whitelist Info key doesn't match");
                return Err(ProgramError::MissingRequiredSignature);
            }   

            if whitelist_info.owner!=program_id
            {
                msg!("Whitelist Info not owned by the program");
                return Err(ProgramError::MissingRequiredSignature);
            }
            let wlist = Whitelist::from_account(whitelist_info)?;  
            let total_payment=data.loan_amt*(1+wlist.daily_interest_rate*days);
            if amount<total_payment
            {
                msg!("You should pay {} this much amount to clear the debt",total_payment);
                return Err(ProgramError::MissingRequiredSignature);
            }            
            //checking if someone has already offerred the loan or not

            let lender = next_account_info(account_info_iter)?; //lender account
            if *lender.key!=data.lender
            {
                msg!("The lender key doesn't match");
                return Err(ProgramError::MissingRequiredSignature);
            }
            msg!("Releasing the fund of the lender");
            invoke(  
                &system_instruction::transfer(
                nft_owner.key,
                lender.key,
                total_payment,    
            ),
            &[
                nft_owner.clone(),
                lender.clone(),
                system_program.clone(),
            ],
            )?;

            //All conditions satisfied release the NFT
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_id.key,
                    nft_associated_address.key,
                    nft_owner_nft_associated.key,
                    nft_vault.key,
                    &[nft_vault.key],
                    1
                )?,
                &[
                    token_program_id.clone(),
                    nft_owner_nft_associated.clone(),
                    nft_associated_address.clone(),
                    nft_owner.clone(),
                    system_program.clone()
                ],
                &[&nft_vault_signer_seeds],
            )?;          
  
            data.canceled=true;
            data.serialize(&mut &mut pda.data.borrow_mut()[..])?;


         Ok(())
    }
    pub fn process_lending(program_id: &Pubkey,accounts: &[AccountInfo],)-> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let nft_owner =  next_account_info(account_info_iter)?; // sender or signer
        let nft_mint = next_account_info(account_info_iter)?;  // mint address of nft
        let token_program_id = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let pda = next_account_info(account_info_iter)?; // pda data 
        let nft_vault = next_account_info(account_info_iter)?;  // nft vault address from NFTPREFIX, nft_owner, pda and program id
        let nft_associated_address = next_account_info(account_info_iter)?; // address generated from nft_vault_address and nft mint address token account address
        let system_program = next_account_info(account_info_iter)?; //system program
        let lender = next_account_info(account_info_iter)?; //lender account
        let lender_token_account = next_account_info(account_info_iter)?; //lender token account from mint and lender key
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let rent_info  = next_account_info(account_info_iter)?; // rent 

          //checking if the owner is the signer or not
          if !lender.is_signer
          {
              return Err(ProgramError::MissingRequiredSignature);
          }

        if pda.owner!=program_id
        {
            msg!("The data_account isn't owned by the program");
            return Err(ProgramError::MissingRequiredSignature);
        }
        let mut data = NftDetails::try_from_slice(&pda.data.borrow())?;

        if data.nft_owner!=*nft_owner.key
        {
            msg!("The NFT owner doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if data.nft_mint!=*nft_mint.key
        {
            msg!("The NFT owner doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !data.loan_taken
         {
            msg!("The Loan isnot taken");
            return Err(ProgramError::MissingRequiredSignature);
         }
         
         //finding nft token account
         let nft_token_address=get_associated_token_address(lender.key,nft_mint.key);
 
         //verifying nft token address
         if nft_token_address!=*lender_token_account.key
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
         let token_balance = get_token_balance(nft_associated_address)?;
           if token_balance!=1
            {
               msg!("The vault doesn't contain the specified NFT");
               return Err(ProgramError::MissingRequiredSignature);
            }
            let now = Clock::get()?.unix_timestamp as u64; 
            let days:u64 =(now-data.loan_start)/86400;
            if days <14
            {
                msg!("14 day time is required");
                return Err(ProgramError::MissingRequiredSignature);
            }
            if *lender.key!=data.lender
            {
                msg!("The lender key doesn't match");
                return Err(ProgramError::MissingRequiredSignature);
            }
            msg!("Releasing the NFT to the lender");

            if lender_token_account.data_is_empty(){
                invoke(            
                    &spl_associated_token_account::instruction::create_associated_token_account(
                        lender.key,
                        lender.key,
                        nft_mint.key,
                    ),&[
                        lender.clone(),
                        lender_token_account.clone(),
                        nft_mint.clone(),
                        token_program_id.clone(),
                        rent_info.clone(),
                        associated_token_info.clone(),
                        system_program.clone()
                    ]
                )?;}

            //All conditions satisfied release the NFT
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_id.key,
                    nft_associated_address.key,
                    lender_token_account.key,
                    nft_vault.key,
                    &[nft_vault.key],
                    1
                )?,
                &[
                    token_program_id.clone(),
                    lender_token_account.clone(),
                    nft_associated_address.clone(),
                    nft_owner.clone(),
                    system_program.clone()
                ],
                &[&nft_vault_signer_seeds],
            )?;          
  
            data.canceled=true;
            data.serialize(&mut &mut pda.data.borrow_mut()[..])?;
            Ok(())
        }

   
        pub fn process_whitelist(program_id: &Pubkey,accounts: &[AccountInfo],number:u64)-> ProgramResult {
        //depositing the NFT
        let account_info_iter = &mut accounts.iter();
        let admin =  next_account_info(account_info_iter)?; // sender or signer
        let system_program = next_account_info(account_info_iter)?; //system program
        let whitelist_info =next_account_info(account_info_iter)?; //
        let creator1=next_account_info(account_info_iter)?; //
        
        if !admin.is_signer
        {
            msg!("Admin isn't the signer");
            return Err(ProgramError::MissingRequiredSignature); 
        }
         //verifying admin
         let admin_address="5j2V6qBBt7S6guRhP6Jg4nUeYUhYmySoZAyLS7uTdREt"
         .parse::<Pubkey>()
         .expect("Failed to parse admin address");
 
         if admin_address!=*admin.key
         {
             msg!("Admin Address doesn't match");
             return Err(ProgramError::MissingRequiredSignature);
         }

        let first_creator=creator1.key;
        let (whitelist_address, bump_seed)=derive_whitelist_address(first_creator, program_id);

        if whitelist_address!=*whitelist_info.key
        {
            msg!("Whitelist Info key doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }
         //signer seeds for nft_vault
         let whitelist_signer_seeds: &[&[_]] = &[
            WHITELIST.as_bytes(),
            &creator1.key.to_bytes(),
            &[bump_seed],
        ];
         //rent account
         let rent = Rent::get()?;
         let number=number as usize;
         let size = std::mem::size_of::<Whitelist>()+number*std::mem::size_of::<Pubkey>();
         let transfer_amount =  rent.minimum_balance(size);
         invoke_signed(
             &system_instruction::create_account(
                 admin.key,
                 whitelist_info.key,
                 transfer_amount,
                 size as u64,
                program_id,
             ),
             &[
                 admin.clone(),
                 whitelist_info.clone(),
                 system_program.clone(),
             ],
             &[&whitelist_signer_seeds],
         )?;    

        let mut  wlist = Whitelist::from_account(whitelist_info)?;
        wlist.state=true;
        wlist.producer.push(*first_creator);
        for _ in 1..number
        {
            let creator=next_account_info(account_info_iter)?; //
            wlist.producer.push(*creator.key);
        }
        wlist.serialize(&mut *whitelist_info.data.borrow_mut())?;        
        Ok(())
    }
    pub fn process_remove_whitelist(program_id: &Pubkey,accounts: &[AccountInfo])-> ProgramResult {
        //depositing the NFT
        let account_info_iter = &mut accounts.iter();
        let admin =  next_account_info(account_info_iter)?; // sender or signer
        let whitelist_info =next_account_info(account_info_iter)?; //
        let creator1=next_account_info(account_info_iter)?; //

        if !admin.is_signer
        {
            msg!("Admin isn't the signer");
            return Err(ProgramError::MissingRequiredSignature); 
        }
         //verifying admin
         let admin_address="5j2V6qBBt7S6guRhP6Jg4nUeYUhYmySoZAyLS7uTdREt"
         .parse::<Pubkey>()
         .expect("Failed to parse admin address");
 
         if admin_address!=*admin.key
         {
             msg!("Admin Address doesn't match");
             return Err(ProgramError::MissingRequiredSignature);
         }
        let first_creator=creator1.key;
        let (whitelist_address, _)=derive_whitelist_address(first_creator, program_id);

        if whitelist_address!=*whitelist_info.key
        {
            msg!("Whitelist Info key doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }   

        if whitelist_info.owner!=program_id
        {
            msg!("Whitelist Info not owned by the program");
            return Err(ProgramError::MissingRequiredSignature);
        }
        let mut  wlist = Whitelist::from_account(whitelist_info)?;
        msg!("The state is changed whitelisting off");
        wlist.state=false;
        wlist.serialize(&mut *whitelist_info.data.borrow_mut())?;        
        Ok(())
    }
    pub fn process_update_interest(program_id: &Pubkey,accounts: &[AccountInfo],interest:u64)-> ProgramResult {
        //depositing the NFT
        let account_info_iter = &mut accounts.iter();
        let admin =  next_account_info(account_info_iter)?; // sender or signer
        let whitelist_info =next_account_info(account_info_iter)?; //
        let creator1=next_account_info(account_info_iter)?; //

        if !admin.is_signer
        {
            msg!("Admin isn't the signer");
            return Err(ProgramError::MissingRequiredSignature); 
        }
         //verifying admin
         let admin_address="5j2V6qBBt7S6guRhP6Jg4nUeYUhYmySoZAyLS7uTdREt"
         .parse::<Pubkey>()
         .expect("Failed to parse admin address");
 
         if admin_address!=*admin.key
         {
             msg!("Admin Address doesn't match");
             return Err(ProgramError::MissingRequiredSignature);
         }
        let first_creator=creator1.key;
        let (whitelist_address, _)=derive_whitelist_address(first_creator, program_id);

        if whitelist_address!=*whitelist_info.key
        {
            msg!("Whitelist Info key doesn't match");
            return Err(ProgramError::MissingRequiredSignature);
        }   

        if whitelist_info.owner!=program_id
        {
            msg!("Whitelist Info not owned by the program");
            return Err(ProgramError::MissingRequiredSignature);
        }
        let mut wlist = Whitelist::from_account(whitelist_info)?;
        if !wlist.state
        {
            msg!("Whitelist Cancelled");
            return Err(ProgramError::MissingRequiredSignature);
        }
        wlist.daily_interest_rate=interest;
        wlist.serialize(&mut *whitelist_info.data.borrow_mut())?;        
        Ok(())
    }

    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult
    {
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
            TokenInstruction::ProcessWhitelist(ProcessWhitelist{number}) => {
                msg!("Instruction:  Whitelist Collection");
                Self::process_whitelist(program_id,accounts,number)
            }
            TokenInstruction::ProcessRemoveWhitelist => {
                msg!("Instruction:  Lender Action");
                Self::process_remove_whitelist(program_id,accounts)
            }
            TokenInstruction::ProcessUpdate(ProcessUpdate{interest}) => {
                msg!("Instruction:  Whitelist Collection");
                Self::process_update_interest(program_id,accounts,interest)
            }}
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