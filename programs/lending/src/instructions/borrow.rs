use std::f32::consts::E;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use crate::constants::{MAXIMUM_AGE, SOL_USD_FEED_ID, USDC_USD_FEED_ID};
use crate::state::*;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut, 
        seeds = [mint.key().as_ref()],
        bump,
    )]  
    pub bank: Account<'info, Bank>,
    #[account(
        mut, 
        seeds = [b"treasury", mint.key().as_ref()],
        bump, 
    )]  
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut, 
        seeds = [signer.key().as_ref()],
        bump,
    )]  
    pub user_account: Account<'info, User>,
    #[account( 
        init_if_needed, 
        payer = signer,
        associated_token::mint = mint, 
        associated_token::authority = signer,
        associated_token::token_program = token_program,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>, 
    pub price_update: Account<'info, PriceUpdateV2>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn process_borrow(ctx : Context<Borrow>,amount : u64) -> Result<()>{

    // Extract Accounts
    let bank = &mut ctx.accounts.bank;
    let user = &mut ctx.accounts.user_account;
    let price_update = &mut ctx.accounts.price_update;

    // Determine Total Collateral
    let total_collateral : u64;
    match ctx.accounts.mint.to_account_info().key(){
        key if key == user.usdc_address => {
            let sol_feed_id = get_feed_id_from_hex(SOL_USD_FEED_ID)?; 
            let sol_price = price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &sol_feed_id)?;
            let accrued_interest = calculate_accrued_interest(user.deposited_sol, bank.interest_rate, user.last_updated)?;
            total_collateral = sol_price.price as u64 * (user.deposited_sol + accrued_interest);
        },
        _ => {
            let usdc_feed_id = get_feed_id_from_hex(USDC_USD_FEED_ID)?;
            let usdc_price = price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &usdc_feed_id)?;
            total_collateral = usdc_price.price as u64 * user.deposited_usdc;
        }
    }
    /*
    This block calculates the total collateral value that the user has deposited.
    If the user has deposited SOL (Solana), it fetches the current price of SOL, calculates accrued interest on the deposited SOL, and computes the total collateral value.
    If the user has deposited USDC, it fetches the current price of USDC and computes the total collateral value directly.
    The calculate_accrued_interest function calculates interest accrued on the deposited collateral.
    */
    let borrowable_amount = total_collateral as u64 * bank.liquidation_threshold;

    if borrowable_amount < amount {
    return Err(ErrorCode::OverBorrowableAmount.into());
    }
    /*
    The borrowable_amount is calculated by multiplying the total collateral value by the liquidation_threshold (a protocol-defined parameter determining how much can be borrowed against the collateral).
If the requested borrow amount exceeds the borrowable_amount, the function returns an error, indicating the user is attempting to borrow more than allowed.
     */

     // Perform Transfer 
     let transfer_cpi_accounts = TransferChecked {
        from: ctx.accounts.bank_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.bank_token_account.to_account_info(),
    };
    
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let mint_key = ctx.accounts.mint.key();
    let signer_seeds: &[&[&[u8]]] = &[
        &[
            b"treasury",
            mint_key.as_ref(),
            &[ctx.bumps.bank_token_account],
        ],
    ];
    let cpi_ctx = CpiContext::new(cpi_program, transfer_cpi_accounts).with_signer(signer_seeds);
    let decimals = ctx.accounts.mint.decimals;
    
    token_interface::transfer_checked(cpi_ctx, amount, decimals)?;

    // Update Protocol and User state
    if bank.total_borrowed == 0 {
        bank.total_borrowed = amount;
        bank.total_borrowed_shares = amount;
    } 
    
    let borrow_ratio = amount.checked_div(bank.total_borrowed).unwrap();
    let users_shares = bank.total_borrowed_shares.checked_mul(borrow_ratio).unwrap();
    
    bank.total_borrowed += amount;
    bank.total_borrowed_shares += users_shares; 
    
    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {
            user.borrowed_usdc += amount;
            user.deposited_usdc_shares += users_shares;
        },
        _ => {
            user.borrowed_sol += amount;
            user.deposited_sol_shares += users_shares;
        }
    }

    /*
    If this is the first borrow, it initializes the total_borrowed and total_borrowed_shares values in the bank account.
It then calculates the ratio of the new borrow amount to the total borrowed amount and uses this ratio to determine the user's share of the total borrowed shares.
The protocol's total borrowed amount and shares are updated.
The user's borrowed amounts and shares are updated based on whether they are borrowing USDC or SOL.
     */
    Ok(())
}

/*
The calculate_accrued_interest function calculates the interest accrued on the collateral that a user has deposited over time, based on an interest rate and the time elapsed since the last update.
*/
fn calculate_accrued_interest(deposited: u64, interest_rate: u64, last_update: i64) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp;
    let time_elapsed = current_time - last_update;
    // Apply exponential growth formula
    let new_value = (deposited as f64 * E.powf(interest_rate as f32 * time_elapsed as f32) as f64) as u64;
    Ok(new_value)
}

/*
Summary
The process_borrow function allows a user to borrow tokens from a DeFi protocol by:

Verifying that the user has sufficient collateral.
Calculating the borrowable amount based on the collateral value and protocol parameters.
Performing a token transfer from the protocol to the user.
Updating the protocol and user state to reflect the new borrowed amount and shares.
This process ensures that the protocol remains secure and that users can only borrow within
*/