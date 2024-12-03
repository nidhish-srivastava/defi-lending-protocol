use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{self,Mint,TokenAccount,TokenInterface,TransferChecked};
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info>{
    #[account(mut)]
    pub signer : Signer<'info>,
    pub mint : InterfaceAccount<'info,Mint>,
    // bank account and its token account
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
    // user account and its token account
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
    pub user_token_account : InterfaceAccount<'info,TokenAccount>,
    pub token_program : Interface<'info,TokenInterface>,
    pub token_program : Interface<'info,TokenInterface>,
    pub associated_token_program : Program<'info,AssociatedToken>,
    pub system_program : Program<'info,System>
}

// 1. CPI transfer from user's token account to bank's token account

pub fn process_deposit(ctx : Context<Deposit>,amount : u64)-> Result<()>{
    // check if user has enough collateral to borrow
    let bank = &mut ctx.accounts.bank;
    let user = &mut ctx.accounts.user_account;
    
    Ok(())
}