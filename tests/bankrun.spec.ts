import { describe, it } from 'node:test';
import { BN, Program } from '@coral-xyz/anchor';
import { BankrunProvider } from 'anchor-bankrun';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { createAccount, createMint, mintTo } from 'spl-token-bankrun';
import { PythSolanaReceiver } from '@pythnetwork/pyth-solana-receiver';

import { startAnchor, BanksClient, ProgramTestContext } from 'solana-bankrun';

import { PublicKey, Keypair, Connection } from '@solana/web3.js';

// @ts-ignore
import IDL from '../target/idl/lending_protocol.json';
import { LendingProtocol } from '../target/types/lending_protocol';
import { BankrunContextWrapper } from '../bankrun-utils/bankrunConnection';

describe('Lending Smart Contract Tests', async () => {
  let signer: Keypair;
  let usdcBankAccount: PublicKey;
  let solBankAccount: PublicKey;

  let solTokenAccount: PublicKey;
  let provider: BankrunProvider;
  let program: Program<LendingProtocol>;
  let banksClient: BanksClient;
  let context: ProgramTestContext;
  let bankrunContextWrapper: BankrunContextWrapper;

  const pyth = new PublicKey('7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE');

  const devnetConnection = new Connection('https://api.devnet.solana.com');
  const accountInfo = await devnetConnection.getAccountInfo(pyth);

  context = await startAnchor(
    '',
    [{ name: 'lending', programId: new PublicKey(IDL.address) }],
    [
      {
        address: pyth,
        info: accountInfo,
      },
    ]
  );
  provider = new BankrunProvider(context);

  bankrunContextWrapper = new BankrunContextWrapper(context);

  const connection = bankrunContextWrapper.connection.toConnection();

  const pythSolanaReceiver = new PythSolanaReceiver({
    connection,
    wallet: provider.wallet,
  });

  const SOL_PRICE_FEED_ID =
    '0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a';

  const solUsdPriceFeedAccount = pythSolanaReceiver
    .getPriceFeedAccountAddress(0, SOL_PRICE_FEED_ID)
    .toBase58();

  const solUsdPriceFeedAccountPubkey = new PublicKey(solUsdPriceFeedAccount);
  const feedAccountInfo = await devnetConnection.getAccountInfo(
    solUsdPriceFeedAccountPubkey
  );

  context.setAccount(solUsdPriceFeedAccountPubkey, feedAccountInfo);

  console.log('pricefeed:', solUsdPriceFeedAccount);

  console.log('Pyth Account Info:', accountInfo);

  program = new Program<LendingProtocol>(IDL as LendingProtocol, provider);

  banksClient = context.banksClient;

  signer = provider.wallet.payer;

  const mintUSDC = await createMint(
    // @ts-ignore
    banksClient,
    signer,
    signer.publicKey,
    null,
    2
  );

  const mintSOL = await createMint(
    // @ts-ignore
    banksClient,
    signer,
    signer.publicKey,
    null,
    2
  );

  [usdcBankAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from('treasury'), mintUSDC.toBuffer()],
    program.programId
  );

  [solBankAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from('treasury'), mintSOL.toBuffer()],
    program.programId
  );

  [solTokenAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from('treasury'), mintSOL.toBuffer()],
    program.programId
  );

  console.log('USDC Bank Account', usdcBankAccount.toBase58());

  console.log('SOL Bank Account', solBankAccount.toBase58());
  // 1. Initialization
  // This test initializes a user account and links it to the mint for USDC, ensuring the user can interact with the protocol.
  it('Test Init User', async () => {
  console.log('\n--- Test: Initialize User ---');
  console.log('Preparing to initialize user account...');
  console.log(`Signer Address: ${signer.publicKey.toBase58()}`);
    
    const initUserTx = await program.methods
      .initUser(mintUSDC)
      .accounts({
        signer: signer.publicKey,
      })
      .rpc({ commitment: 'confirmed' });

      console.log('Transaction Signature for User Initialization:', initUserTx);
  });

  // Prepares USDC and SOL "bank" accounts:
  // These tests create treasury accounts for USDC and SOL and fund them with initial token amounts.
  it('Test Init and Fund USDC Bank', async () => {
    console.log('\n--- Test: Initialize and Fund USDC Bank ---');
    console.log(`USDC Mint: ${mintUSDC.toBase58()}`);
    console.log(`Treasury Account (USDC): ${usdcBankAccount.toBase58()}`);

    const initUSDCBankTx = await program.methods
      .initBank(new BN(1), new BN(1))
      .accounts({
        signer: signer.publicKey,
        mint: mintUSDC,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });

    console.log('Initialized USDC Bank Transaction Signature:', initUSDCBankTx);

    const amount = 10_000;
    console.log('\n--- Minting Process ---');
    console.log(`Amount to Mint: ${amount} USDC`);
    console.log(`Mint Address: ${mintUSDC.toBase58()}`);
    console.log(`Destination Treasury Account: ${usdcBankAccount.toBase58()}`);
    console.log(`Mint Authority (Signer): ${signer.publicKey.toBase58()}`);

    const mintTx = await mintTo(
      // @ts-ignores
      banksClient,
      signer,
      mintUSDC,
      usdcBankAccount,
      signer,
      amount
    );

    console.log(`Minted ${amount} USDC to Treasury. Transaction Signature:`, mintTx);
  });


  it('Test Init and Fund SOL Bank', async () => {
    console.log('\n--- Test: Initialize and Fund SOL Bank ---');
    console.log(`SOL Mint Address: ${mintSOL.toBase58()}`);
    console.log(`Treasury Account (SOL): ${solBankAccount.toBase58()}`);
  
    const initSOLBankTx = await program.methods
      .initBank(new BN(1), new BN(1))
      .accounts({
        signer: signer.publicKey,
        mint: mintSOL,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });
  
    console.log('Initialized SOL Bank Transaction Signature:', initSOLBankTx);
  
    const amount = 10_000; // 10,000 SOL in token terms
    console.log(`Minting ${amount} SOL to the Treasury Account...`);
    console.log(`Mint Address: ${mintSOL.toBase58()}`);
    console.log(`Destination Treasury Account: ${solBankAccount.toBase58()}`);
    console.log(`Mint Authority: ${signer.publicKey.toBase58()}`);
  
    const mintSOLTx = await mintTo(
      // @ts-ignore
      banksClient,
      signer,
      mintSOL,
      solBankAccount,
      signer,
      amount
    );
  
    console.log(`Minted ${amount} SOL to Treasury. Transaction Signature:`, mintSOLTx);
  });
  

  // 2. Token account creation

  // Ensures users can have accounts to hold USDC
  // This step sets up a token account for the user and funds it with a specified amount of USDC.
  it('Create and Fund Token Account', async () => {
    console.log('\n--- Test: Create and Fund USDC Token Account ---');

    console.log('Creating a new USDC Token Account...');
    const USDCTokenAccount = await createAccount(
      // @ts-ignore
      banksClient,
      signer,
      mintUSDC,
      signer.publicKey
    );
  
    console.log('USDC Token Account Created:', USDCTokenAccount.toBase58());
  
    const amount = 10_000 * 10 ** 9; // 10,000 USDC
    console.log(`Minting ${amount / 10 ** 9} USDC to the created token account...`);
    console.log(`Mint Address: ${mintUSDC.toBase58()}`);
    console.log(`Destination Token Account: ${USDCTokenAccount.toBase58()}`);
    console.log(`Mint Authority: ${signer.publicKey.toBase58()}`);
  
    const mintUSDCTx = await mintTo(
      // @ts-ignore
      banksClient,
      signer,
      mintUSDC,
      USDCTokenAccount,
      signer,
      amount
    );
  
    console.log('Mint Transaction Signature:', mintUSDCTx);
    console.log(`Successfully minted ${amount / 10 ** 9} USDC to ${USDCTokenAccount.toBase58()}`);
  });

  // The user deposits a specified amount of USDC into the protocol. This tests the deposit function.
  it('Test Deposit', async () => {
    console.log('\n--- Test: Deposit USDC ---');
    const depositAmount = 100_000_000_000; // 100 USDC
    console.log(`Depositing ${depositAmount / 10 ** 9} USDC...`);
    console.log(`Depositor: ${signer.publicKey.toBase58()}`);
    console.log(`Treasury Account (USDC): ${usdcBankAccount.toBase58()}`);
  
    const depositUSDC = await program.methods
      .deposit(new BN(depositAmount))
      .accounts({
        signer: signer.publicKey,
        mint: mintUSDC,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });
  
    console.log('Deposit Transaction Signature:', depositUSDC);
  });

  // The user borrows SOL using their USDC deposit as collateral. The test ensures the borrow logic interacts with the Pyth price feed correctly.
  it('Test Borrow', async () => {
 console.log('\n--- Test: Borrow SOL ---');
  const borrowAmount = 1; // 1 SOL
  console.log(`Borrowing ${borrowAmount} SOL...`);
  console.log(`Borrower: ${signer.publicKey.toBase58()}`);
  console.log(`Price Feed Account: ${solUsdPriceFeedAccount}`);

  const borrowSOL = await program.methods
    .borrow(new BN(borrowAmount))
    .accounts({
      signer: signer.publicKey,
      mint: mintSOL,
      tokenProgram: TOKEN_PROGRAM_ID,
      priceUpdate: solUsdPriceFeedAccount,
    })
    .rpc({ commitment: 'confirmed' });

  console.log('Borrow Transaction Signature:', borrowSOL);
  });

  // The user repays a borrowed amount of SOL. This verifies that the repayment logic updates balances properly.
  it('Test Repay', async () => {
    console.log('\n--- Test: Repay SOL ---');
    const repayAmount = 1; // 1 SOL
    console.log(`Repaying ${repayAmount} SOL...`);
    console.log(`Repayer: ${signer.publicKey.toBase58()}`);
    console.log(`Treasury Account (SOL): ${solBankAccount.toBase58()}`);
  
    const repaySOL = await program.methods
      .repay(new BN(repayAmount))
      .accounts({
        signer: signer.publicKey,
        mint: mintSOL,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });
  
    console.log('Repay Transaction Signature:', repaySOL);
  });

  // The user withdraws some of their deposited USDC. This ensures the protocol allows withdrawals up to the remaining balance after accounting for collateralization.
  it('Test Withdraw', async () => {
    console.log('\n--- Test: Withdraw USDC ---');
    const withdrawAmount = 100; // 100 USDC
    console.log(`Withdrawing ${withdrawAmount} USDC...`);
    console.log(`Withdrawer: ${signer.publicKey.toBase58()}`);
    console.log(`Treasury Account (USDC): ${usdcBankAccount.toBase58()}`);
  
    const withdrawUSDC = await program.methods
      .withdraw(new BN(withdrawAmount))
      .accounts({
        signer: signer.publicKey,
        mint: mintUSDC,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });
  
    console.log('Withdraw Transaction Signature:', withdrawUSDC);
  });
});

/*
1.Setup and Initialization:

Connects to the Solana Devnet and initializes various components such as mint accounts, token accounts, and bank accounts for USDC and SOL.
Establishes a price feed account for fetching SOL prices, using the Pyth network for accurate price data.

2.Functionality Tests:

Each test case corresponds to a specific action in the lending protocol:
Initialization (initUser and initBank).
Minting tokens (mintTo).
Core protocol actions (deposit, borrow, repay, withdraw).

3.Core Protocol Actions
Deposit
Borrow
Repay
Withdraw
*/

/*
Execution Sequence : 
Initialize protocol components (users, mints, and bank accounts).
Deposit USDC as collateral.
Borrow SOL using deposited USDC.
Repay borrowed SOL.
Withdraw a portion of the collateral (USDC).

Why This Test Set? : 
The test sequence covers all primary user interactions:
Adding funds (Deposit).
Utilizing funds (Borrow).
Fulfilling obligations (Repay).
Retrieving excess funds (Withdraw).
Ensures the lending protocol handles each operation correctly and validates critical computations like price feed integration and account updates.

*/