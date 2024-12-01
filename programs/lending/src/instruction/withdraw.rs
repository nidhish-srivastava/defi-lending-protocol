use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint,TokenAccount,TokenInterface,TransferChecked};
use crate::state::*;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Withdraw<'info>{
    #[account(mut)]
    pub signer : Signer<'info>,
    pub mint : InterfaceAccount<'info,Mint>,
    #[account(
        mut,
        seeds = [mint.key().as_ref()],
        bump
    )]
    pub bank : Account<'info,Bank>,
    #[account(
        mut,
        seeds = [b"treasury",mint.key().as_ref()],
        bump
    )]
    pub bank_token_account : InterfaceAccount<'info,TokenAccount>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>, 
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// 1. CPI transfer from bank's token account to user's token account

pub fn process_withdraw(ctx : Context<Withdraw>,amount : u64) -> Result<()>{
    Ok(())
}