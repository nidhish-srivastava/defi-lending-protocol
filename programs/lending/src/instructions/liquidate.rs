/*
What is Liquidation?
In the context of decentralized finance (DeFi) and traditional finance, liquidation is the process of converting assets into cash or cash equivalents by selling them on the open market. In DeFi, liquidation specifically refers to the act of selling off a borrower's collateral when their loan position becomes undercollateralized, meaning the value of their collateral falls below a required threshold compared to their borrowed amount.

How Liquidation Works in DeFi
Collateral and Borrowing: Users deposit collateral (e.g., ETH, BTC) into a lending protocol to borrow other assets (e.g., stablecoins like USDC).
Collateralization Ratio: The protocol requires that the value of the collateral remains above a certain percentage of the borrowed amount (e.g., 150%).
Monitoring: The protocol continuously monitors the value of the collateral against the borrowed amount.
Undercollateralization: If the collateral value falls below the required threshold due to market fluctuations, the loan becomes undercollateralized.
Liquidation: When undercollateralization occurs, the protocol initiates a liquidation process where part or all of the collateral is sold to repay the loan, thus protecting the protocol from bad debt.
Example of Liquidation
Let's walk through a simple example:

Alice Deposits Collateral:

Alice deposits 2 ETH (worth $2,000, assuming 1 ETH = $1,000) into a DeFi lending protocol.
She borrows $1,000 in USDC against her ETH.
Collateralization Ratio:

The protocol requires a collateralization ratio of 150%.
For a $1,000 loan, Alice needs at least $1,500 worth of ETH as collateral.
Market Fluctuations:

The price of ETH drops to $600.
Alice's 2 ETH is now worth $1,200.
Undercollateralization:

Alice's collateral is now only worth $1,200, which is below the required $1,500 (150% of $1,000).
Her collateralization ratio is now 120%.
Liquidation Trigger:

The protocol detects that Alice's collateralization ratio has fallen below the threshold.
It initiates liquidation to protect itself from potential loss.
Liquidation Process:

The protocol sells a portion of Alice's ETH collateral to cover her $1,000 debt.
Let's say the protocol sells 1.67 ETH (worth $1,000 at $600/ETH) to repay the loan.
The protocol might also charge a liquidation fee or give a bonus to the liquidator (the person or bot executing the liquidation).
Post-Liquidation:

Alice is left with 0.33 ETH (worth $200) after liquidation.
Her loan is repaid, and the protocol remains solvent.
Benefits of Liquidation
Protects the Protocol: Ensures the protocol remains solvent and can cover all outstanding loans.
Incentivizes Liquidators: Provides opportunities for liquidators to earn fees or bonuses by maintaining the health of the system.
Mitigates Risk: Prevents the protocol from holding bad debt by quickly addressing undercollateralized positions.
In summary, liquidation in DeFi is a crucial mechanism to maintain the health and solvency of lending protocols by ensuring that loans are adequately collateralized and by mitigating the risk of bad debt through the sale of collateral assets.

*/
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{ self, Mint, TokenAccount, TokenInterface, TransferChecked };
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use crate::constants::{MAXIMUM_AGE, SOL_USD_FEED_ID, USDC_USD_FEED_ID};
use crate::state::*;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,
    pub price_update: Account<'info, PriceUpdateV2>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    pub borrowed_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut, 
        seeds = [collateral_mint.key().as_ref()],
        bump,
    )]  
    pub collateral_bank: Account<'info, Bank>,
    #[account(
        mut, 
        seeds = [b"treasury", collateral_mint.key().as_ref()],
        bump, 
    )]  
    pub collateral_bank_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut, 
        seeds = [borrowed_mint.key().as_ref()],
        bump,
    )]  
    pub borrowed_bank: Account<'info, Bank>,
    #[account(
        mut, 
        seeds = [b"treasury", borrowed_mint.key().as_ref()],
        bump, 
    )]  
    pub borrowed_bank_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut, 
        seeds = [liquidator.key().as_ref()],
        bump,
    )]  
    pub user_account: Account<'info, User>,
    #[account( 
        init_if_needed, 
        payer = liquidator,
        associated_token::mint = collateral_mint, 
        associated_token::authority = liquidator,
        associated_token::token_program = token_program,
    )]
    pub liquidator_collateral_token_account: InterfaceAccount<'info, TokenAccount>, 
    #[account( 
        init_if_needed, 
        payer = liquidator,
        associated_token::mint = borrowed_mint, 
        associated_token::authority = liquidator,
        associated_token::token_program = token_program,
    )]
    pub liquidator_borrowed_token_account: InterfaceAccount<'info, TokenAccount>, 
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}


// Core logic of liquidation
pub fn process_liquidate(ctx: Context<Liquidate>) -> Result<()> {
    let collateral_bank = &mut ctx.accounts.collateral_bank;
    let user = &mut ctx.accounts.user_account;

    /*
    Oracles are services that provide external data to a blockchain network. Blockchains are siloed environments that do not inherently know the outside world. Oracles solve this limitation by offering a decentralized way to get various types of data onchain, such as: Results of sporting events.
    1.Retrieve Prices from Oracles
    */
    let price_update = &mut ctx.accounts.price_update;
    // Retrieve price feed ids from sol and isdc
    let sol_feed_id = get_feed_id_from_hex(SOL_USD_FEED_ID)?;
    let usdc_feed_id = get_feed_id_from_hex(USDC_USD_FEED_ID)?;
    // fetching latest prices from sol and usdc using price_update account
    let sol_price =
        price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &sol_feed_id)?;
    let usdc_price =
        price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &usdc_feed_id)?;

    // 2. Calculate total collateral and total borrowed
    let total_collateral = (sol_price.price as u64 * user.deposited_sol)
        + (usdc_price.price as u64 * user.deposited_usdc);
    let total_borrowed = (sol_price.price as u64 * user.borrowed_sol)
        + (usdc_price.price as u64 * user.borrowed_usdc);

    // 3. Calculate Health Factor of user's account. If it is >=1, the user is not undercollaterised,liquidation cant proceed
    let health_factor = (total_collateral * collateral_bank.liquidation_threshold) / total_borrowed;
    if health_factor >= 1 {
        return Err(ErrorCode::NotUndercollateralized.into());
    }

    //4. Determine liquidation amount
    let liquidation_amount = total_borrowed * collateral_bank.liquidation_close_factor;
    /*
    Calculate the amount to be liquidated based on the total borrowed and the bank's liquidation close factor.
    */

    // 5. Transfer borrowed Tokens to Bank
    let transfer_to_bank = TransferChecked {
        from: ctx
            .accounts
            .liquidator_borrowed_token_account
            .to_account_info(),
        mint: ctx.accounts.borrowed_mint.to_account_info(),
        to: ctx.accounts.borrowed_bank_token_account.to_account_info(),
        authority: ctx.accounts.liquidator.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx_to_bank = CpiContext::new(cpi_program.clone(), transfer_to_bank);
    let decimals = ctx.accounts.borrowed_mint.decimals;
    token_interface::transfer_checked(cpi_ctx_to_bank, liquidation_amount, decimals)?;

    // 6. Transfer collateral  and bonus to liquidator
    let liquidation_bonus =
        (liquidation_amount * collateral_bank.liquidation_bonus) + liquidation_amount; // Calculating the bonus for the liquidator

    let transfer_to_liquidator = TransferChecked {
        from: ctx.accounts.collateral_bank_token_account.to_account_info(),
        mint: ctx.accounts.collateral_mint.to_account_info(),
        to: ctx
            .accounts
            .liquidator_collateral_token_account
            .to_account_info(),
        authority: ctx.accounts.collateral_bank_token_account.to_account_info(),
    };
    let mint_key = ctx.accounts.collateral_mint.key();
    let signer_seeds: &[&[&[u8]]] = &[&[ // Defining seeds for the program  for the PDA signer
        b"treasury",
        mint_key.as_ref(),
        &[ctx.bumps.collateral_bank_token_account],
    ]];
    let cpi_ctx_to_liquidator = // Creating cpi context for transfer with signer
        CpiContext::new(cpi_program.clone(), transfer_to_liquidator).with_signer(signer_seeds);
    let collateral_decimals = ctx.accounts.collateral_mint.decimals;
    token_interface::transfer_checked( // Perform the token transfer
        cpi_ctx_to_liquidator,
        liquidation_bonus,
        collateral_decimals,
    )?;
    
    Ok(())
}
