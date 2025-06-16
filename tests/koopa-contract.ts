// import * as anchor from "@coral-xyz/anchor";
// import { Program } from "@coral-xyz/anchor";
// import { Koopa } from "../target/types/koopa";
// import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
// import { expect } from "chai";

// describe("koopa", () => {
//   // Configure the client to use the local cluster
//   const provider = anchor.AnchorProvider.env();
//   anchor.setProvider(provider);

//   const program = anchor.workspace.Koopa as Program<Koopa>;

//   // Common variables
//   const admin = anchor.web3.Keypair.generate();
//   const creator = anchor.web3.Keypair.generate();
//   const participant1 = anchor.web3.Keypair.generate();
//   const participant2 = anchor.web3.Keypair.generate();
//   const participant3 = anchor.web3.Keypair.generate();

//   // Token variables
//   let tokenMint: anchor.web3.PublicKey;
//   let creatorTokenAccount: anchor.web3.PublicKey;
//   let participant1TokenAccount: anchor.web3.PublicKey;
//   let participant2TokenAccount: anchor.web3.PublicKey;
//   let participant3TokenAccount: anchor.web3.PublicKey;

//   // Group variables
//   const groupName = "TestGroup";
//   const securityDeposit = new anchor.BN(50_000_000); // 50 tokens
//   const contributionAmount = new anchor.BN(100_000_000); // 100 tokens
//   const contributionInterval = 7; // 7 days
//   const payoutInterval = 30; // 30 days
//   const numParticipants = 3;

//   // PDA variables
//   let globalStatePDA: anchor.web3.PublicKey;
//   let globalStateBump: number;
//   let ajoGroupPDA: anchor.web3.PublicKey;
//   let ajoGroupBump: number;
//   let groupVaultPDA: anchor.web3.PublicKey;
//   let groupVaultBump: number;

//   // Helper function to airdrop SOL to an account
//   async function airdropSol(to: anchor.web3.PublicKey, amount: number) {
//     const signature = await provider.connection.requestAirdrop(to, amount * anchor.web3.LAMPORTS_PER_SOL);
//     await provider.connection.confirmTransaction(signature);
//   }

//   // Helper function to advance blockchain time
//   async function advanceTime(secondsToAdd: number) {
//     await provider.connection.simulateTransaction(
//       new anchor.web3.Transaction().add(
//         anchor.web3.SystemProgram.transfer({
//           fromPubkey: provider.wallet.publicKey,
//           toPubkey: provider.wallet.publicKey,
//           lamports: 0,
//         })
//       ),
//       [provider.wallet.payer],
//       { minContextSlot: (await provider.connection.getSlot()) + Math.ceil(secondsToAdd / 0.4) }
//     );
//   }

//   before(async () => {
//     // Airdrop SOL to all accounts
//     await airdropSol(admin.publicKey, 10);
//     await airdropSol(creator.publicKey, 10);
//     await airdropSol(participant1.publicKey, 10);
//     await airdropSol(participant2.publicKey, 10);
//     await airdropSol(participant3.publicKey, 10);

//     // Create token mint
//     tokenMint = await createMint(
//       provider.connection,
//       admin,
//       admin.publicKey,
//       null,
//       6 // 6 decimals
//     );

//     // Create token accounts for all users
//     creatorTokenAccount = await createAccount(provider.connection, creator, tokenMint, creator.publicKey);

//     participant1TokenAccount = await createAccount(
//       provider.connection,
//       participant1,
//       tokenMint,
//       participant1.publicKey
//     );

//     participant2TokenAccount = await createAccount(
//       provider.connection,
//       participant2,
//       tokenMint,
//       participant2.publicKey
//     );

//     participant3TokenAccount = await createAccount(
//       provider.connection,
//       participant3,
//       tokenMint,
//       participant3.publicKey
//     );

//     // Mint 1000 tokens to each participant
//     await mintTo(provider.connection, admin, tokenMint, creatorTokenAccount, admin.publicKey, 1000_000_000);

//     await mintTo(provider.connection, admin, tokenMint, participant1TokenAccount, admin.publicKey, 1000_000_000);

//     await mintTo(provider.connection, admin, tokenMint, participant2TokenAccount, admin.publicKey, 1000_000_000);

//     await mintTo(provider.connection, admin, tokenMint, participant3TokenAccount, admin.publicKey, 1000_000_000);

//     // Find PDA for global state
//     [globalStatePDA, globalStateBump] = anchor.web3.PublicKey.findProgramAddressSync(
//       [Buffer.from("global-state")],
//       program.programId
//     );

//     // Find PDA for ajo group
//     [ajoGroupPDA, ajoGroupBump] = anchor.web3.PublicKey.findProgramAddressSync(
//       [Buffer.from("ajo-group"), Buffer.from(groupName)],
//       program.programId
//     );
//   });

//   it("Initializes the global state", async () => {
//     // Initialize global state
//     await program.methods
//       .initialize()
//       .accounts({
//         globalState: globalStatePDA,
//         admin: admin.publicKey,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([admin])
//       .rpc();

//     // Verify global state
//     const globalState = await program.account.globalState.fetch(globalStatePDA);
//     expect(globalState.totalGroups.toNumber()).to.equal(0);
//     expect(globalState.activeGroups.toNumber()).to.equal(0);
//   });

//   it("Creates a new Ajo group", async () => {
//     // Find PDA for group vault before creating group
//     [groupVaultPDA, groupVaultBump] = anchor.web3.PublicKey.findProgramAddressSync(
//       [Buffer.from("group-vault"), ajoGroupPDA.toBuffer()],
//       program.programId
//     );

//     // Create ajo group
//     await program.methods
//       .createAjoGroup(
//         groupName,
//         securityDeposit,
//         contributionAmount,
//         contributionInterval,
//         payoutInterval,
//         numParticipants
//       )
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         creator: creator.publicKey,
//         globalState: globalStatePDA,
//         tokenMint: tokenMint,
//         creatorTokenAccount: creatorTokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//         rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//       })
//       .signers([creator])
//       .rpc();

//     // Verify ajo group
//     const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.name).to.equal(groupName);
//     expect(ajoGroup.contributionAmount.toNumber()).to.equal(contributionAmount.toNumber());
//     expect(ajoGroup.contributionInterval).to.equal(contributionInterval);
//     expect(ajoGroup.payoutInterval).to.equal(payoutInterval);
//     expect(ajoGroup.numParticipants).to.equal(numParticipants);
//     expect(ajoGroup.participants.length).to.equal(1); // Creator is automatically added
//     expect(ajoGroup.participants[0].pubkey.toString()).to.equal(creator.publicKey.toString());
//     expect(ajoGroup.startTimestamp).to.be.null;
//     expect(ajoGroup.isClosed).to.be.false;

//     // Verify global state updated
//     const globalState = await program.account.globalState.fetch(globalStatePDA);
//     expect(globalState.totalGroups.toNumber()).to.equal(1);

//     // Verify security deposit was transferred
//     const vaultAccount = await getAccount(provider.connection, groupVaultPDA);
//     expect(Number(vaultAccount.amount)).to.equal(securityDeposit.toNumber());
//   });

//   it("Allows participants to join the group", async () => {
//     // Participant 1 joins
//     await program.methods
//       .joinAjoGroup()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         participant: participant1.publicKey,
//         globalState: globalStatePDA,
//         tokenMint: tokenMint,
//         participantTokenAccount: participant1TokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([participant1])
//       .rpc();

//     // Verify participant was added
//     let ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.participants.length).to.equal(2);
//     expect(ajoGroup.participants[1].pubkey.toString()).to.equal(participant1.publicKey.toString());
//     expect(ajoGroup.startTimestamp).to.be.null; // Group shouldn't start yet

//     // Participant 2 joins
//     await program.methods
//       .joinAjoGroup()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         participant: participant2.publicKey,
//         globalState: globalStatePDA,
//         tokenMint: tokenMint,
//         participantTokenAccount: participant2TokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([participant2])
//       .rpc();

//     // Verify participant was added
//     ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.participants.length).to.equal(3);
//     expect(ajoGroup.participants[2].pubkey.toString()).to.equal(participant2.publicKey.toString());

//     // Group should be started since all participants have joined
//     expect(ajoGroup.startTimestamp).to.not.be.null;

//     // Verify global state
//     const globalState = await program.account.globalState.fetch(globalStatePDA);
//     expect(globalState.activeGroups.toNumber()).to.equal(1);

//     // Verify vault has all security deposits
//     const vaultAccount = await getAccount(provider.connection, groupVaultPDA);
//     expect(Number(vaultAccount.amount)).to.equal(securityDeposit.toNumber() * 3);
//   });

//   it("Prevents joining an already started group", async () => {
//     try {
//       await program.methods
//         .joinAjoGroup()
//         .accounts({
//           ajoGroup: ajoGroupPDA,
//           participant: participant3.publicKey,
//           globalState: globalStatePDA,
//           tokenMint: tokenMint,
//           participantTokenAccount: participant3TokenAccount,
//           groupTokenVault: groupVaultPDA,
//           tokenProgram: TOKEN_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//         })
//         .signers([participant3])
//         .rpc();

//       // Should not reach here
//       expect.fail("Should not be able to join an already started group");
//     } catch (error) {
//       // Expected error
//       expect(error.toString()).to.include("GroupAlreadyStarted");
//     }
//   });

//   it("Allows contribution after interval has passed", async () => {
//     // Advance time by contribution interval
//     await advanceTime(contributionInterval * 86400 + 10);

//     // Make contribution
//     await program.methods
//       .contribute()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         contributor: creator.publicKey,
//         contributorTokenAccount: creatorTokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([creator])
//       .rpc();

//     // Verify contribution was recorded
//     const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.participants[0].contributionRound.toNumber()).to.equal(1);

//     // Verify tokens were transferred
//     const vaultAccount = await getAccount(provider.connection, groupVaultPDA);
//     const expectedBalance = securityDeposit.toNumber() * 3 + contributionAmount.toNumber();
//     expect(Number(vaultAccount.amount)).to.equal(expectedBalance);
//   });

//   it("Prevents double contribution in same round", async () => {
//     try {
//       await program.methods
//         .contribute()
//         .accounts({
//           ajoGroup: ajoGroupPDA,
//           contributor: creator.publicKey,
//           contributorTokenAccount: creatorTokenAccount,
//           groupTokenVault: groupVaultPDA,
//           tokenMint: tokenMint,
//           tokenProgram: TOKEN_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//         })
//         .signers([creator])
//         .rpc();

//       // Should not reach here
//       expect.fail("Should not be able to contribute twice in same round");
//     } catch (error) {
//       // Expected error
//       expect(error.toString()).to.include("AlreadyContributed");
//     }
//   });

//   it("Allows other participants to contribute", async () => {
//     // Participant 1 contributes
//     await program.methods
//       .contribute()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         contributor: participant1.publicKey,
//         contributorTokenAccount: participant1TokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([participant1])
//       .rpc();

//     // Participant 2 contributes
//     await program.methods
//       .contribute()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         contributor: participant2.publicKey,
//         contributorTokenAccount: participant2TokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([participant2])
//       .rpc();

//     // Verify all contributions were recorded
//     const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.participants[0].contributionRound.toNumber()).to.equal(1);
//     expect(ajoGroup.participants[1].contributionRound.toNumber()).to.equal(1);
//     expect(ajoGroup.participants[2].contributionRound.toNumber()).to.equal(1);

//     // Verify tokens were transferred
//     const vaultAccount = await getAccount(provider.connection, groupVaultPDA);
//     const expectedBalance = securityDeposit.toNumber() * 3 + contributionAmount.toNumber() * 3;
//     expect(Number(vaultAccount.amount)).to.equal(expectedBalance);
//   });

//   it("Prevents payout before payout interval", async () => {
//     try {
//       await program.methods
//         .payout()
//         .accounts({
//           ajoGroup: ajoGroupPDA,
//           groupSigner: ajoGroupPDA,
//           groupTokenVault: groupVaultPDA,
//           recipient: creatorTokenAccount, // First payout goes to creator
//           caller: creator.publicKey,
//           tokenMint: tokenMint,
//           tokenProgram: TOKEN_PROGRAM_ID,
//         })
//         .signers([creator])
//         .rpc();

//       // Should not reach here
//       expect.fail("Should not be able to payout before interval");
//     } catch (error) {
//       // Expected error
//       expect(error.toString()).to.include("PayoutNotYetDue");
//     }
//   });

//   it("Allows payout after payout interval", async () => {
//     // Advance time to payout interval
//     await advanceTime((payoutInterval - contributionInterval) * 86400 + 10);

//     // Handle second round of contributions first
//     // Advance time by contribution interval
//     await advanceTime(contributionInterval * 86400 + 10);

//     // All participants contribute for round 2
//     await program.methods
//       .contribute()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         contributor: creator.publicKey,
//         contributorTokenAccount: creatorTokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([creator])
//       .rpc();

//     await program.methods
//       .contribute()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         contributor: participant1.publicKey,
//         contributorTokenAccount: participant1TokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([participant1])
//       .rpc();

//     await program.methods
//       .contribute()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         contributor: participant2.publicKey,
//         contributorTokenAccount: participant2TokenAccount,
//         groupTokenVault: groupVaultPDA,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([participant2])
//       .rpc();

//     // Now execute payout to first recipient (creator)
//     await program.methods
//       .payout()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         groupSigner: await anchor.web3.PublicKey.createProgramAddressSync(
//           [Buffer.from("group-vault"), ajoGroupPDA.toBuffer(), [groupVaultBump]],
//           program.programId
//         ),
//         groupTokenVault: groupVaultPDA,
//         recipient: creatorTokenAccount,
//         caller: creator.publicKey,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//       })
//       .signers([creator])
//       .rpc();

//     // Verify payout was recorded
//     const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.payoutRound.toNumber()).to.equal(1);

//     // Verify tokens were transferred
//     const vaultAccount = await getAccount(provider.connection, groupVaultPDA);
//     // Original: 3 security deposits + 6 contributions
//     // After payout: 3 security deposits + 3 contributions
//     const expectedBalance = securityDeposit.toNumber() * 3 + contributionAmount.toNumber() * 3;
//     expect(Number(vaultAccount.amount)).to.equal(expectedBalance);
//   });

//   it("Allows group to be closed with majority vote", async () => {
//     // Creator votes to close
//     await program.methods
//       .closeAjoGroup()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         participant: creator.publicKey,
//         globalState: globalStatePDA,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([creator])
//       .rpc();

//     // Check not closed yet
//     let ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.isClosed).to.be.false;
//     expect(ajoGroup.closeVotes.length).to.equal(1);

//     // Participant 1 votes to close (should make it pass with 2/3 votes)
//     await program.methods
//       .closeAjoGroup()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         participant: participant1.publicKey,
//         globalState: globalStatePDA,
//         systemProgram: anchor.web3.SystemProgram.programId,
//       })
//       .signers([participant1])
//       .rpc();

//     // Check group is now closed
//     ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.isClosed).to.be.true;
//     expect(ajoGroup.closeVotes.length).to.equal(2);

//     // Check global state
//     const globalState = await program.account.globalState.fetch(globalStatePDA);
//     expect(globalState.activeGroups.toNumber()).to.equal(0);
//   });

//   it("Allows participants to claim refunds after group is closed", async () => {
//     // Creator claims refund
//     await program.methods
//       .claimRefund()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         groupSigner: await anchor.web3.PublicKey.createProgramAddressSync(
//           [Buffer.from("group-vault"), ajoGroupPDA.toBuffer(), [groupVaultBump]],
//           program.programId
//         ),
//         groupTokenVault: groupVaultPDA,
//         participant: creator.publicKey,
//         participantTokenAccount: creatorTokenAccount,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//       })
//       .signers([creator])
//       .rpc();

//     // Verify refund was processed
//     const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
//     expect(ajoGroup.participants[0].refundAmount.toNumber()).to.equal(0);

//     // Participant 1 claims refund
//     await program.methods
//       .claimRefund()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         groupSigner: await anchor.web3.PublicKey.createProgramAddressSync(
//           [Buffer.from("group-vault"), ajoGroupPDA.toBuffer(), [groupVaultBump]],
//           program.programId
//         ),
//         groupTokenVault: groupVaultPDA,
//         participant: participant1.publicKey,
//         participantTokenAccount: participant1TokenAccount,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//       })
//       .signers([participant1])
//       .rpc();

//     // Participant 2 claims refund
//     await program.methods
//       .claimRefund()
//       .accounts({
//         ajoGroup: ajoGroupPDA,
//         groupSigner: await anchor.web3.PublicKey.createProgramAddressSync(
//           [Buffer.from("group-vault"), ajoGroupPDA.toBuffer(), [groupVaultBump]],
//           program.programId
//         ),
//         groupTokenVault: groupVaultPDA,
//         participant: participant2.publicKey,
//         participantTokenAccount: participant2TokenAccount,
//         tokenMint: tokenMint,
//         tokenProgram: TOKEN_PROGRAM_ID,
//       })
//       .signers([participant2])
//       .rpc();

//     // Verify vault is empty or close to empty (might have dust)
//     const vaultAccount = await getAccount(provider.connection, groupVaultPDA);
//     expect(Number(vaultAccount.amount)).to.be.lessThan(100);
//   });
// });
