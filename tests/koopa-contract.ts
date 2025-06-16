import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { Koopa } from "../target/types/koopa";
import { expect } from "chai";

describe("koopaa", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Koopa as Program<Koopa>;

  let mint: anchor.web3.PublicKey;
  let creatorTokenAccount: anchor.web3.PublicKey;
  let participantTokenAccount: anchor.web3.PublicKey;
  let vaultAccount: anchor.web3.PublicKey;
  let groupPda: anchor.web3.PublicKey;
  let groupVaultPda: anchor.web3.PublicKey;
  let globalStatePda: anchor.web3.PublicKey;
  let groupName = "Alpha Group";

  const creator = provider.wallet;
  const participant = anchor.web3.Keypair.generate();

  before(async () => {
    // Mint setup
    mint = await createMint(
      provider.connection,
      creator.payer,
      creator.publicKey,
      null,
      6
    );

    creatorTokenAccount = await createAccount(
      provider.connection,
      creator.payer,
      mint,
      creator.publicKey
    );

    participantTokenAccount = await createAccount(
      provider.connection,
      creator.payer,
      mint,
      participant.publicKey
    );

    await mintTo(
      provider.connection,
      creator.payer,
      mint,
      creatorTokenAccount,
      creator.publicKey,
      1_000_000_000
    );

    await mintTo(
      provider.connection,
      creator.payer,
      mint,
      participantTokenAccount,
      creator.publicKey,
      1_000_000_000
    );

    // PDAs
    [globalStatePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("global-state")],
      program.programId
    );

    [groupPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("ajo-group"), Buffer.from(groupName)],
      program.programId
    );

    [groupVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("group-vault"), groupPda.toBuffer()],
      program.programId
    );

    // Airdrop participant
    await provider.connection.requestAirdrop(participant.publicKey, 2e9);
  });

  it("initializes the global state", async () => {
    await program.methods
      .initialize()
      .accountsStrict({
        globalState: globalStatePda,
        admin: creator.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const state = await program.account.globalState.fetch(globalStatePda);
    expect(state.totalGroups.toNumber()).to.equal(0);
  });

  it("creates an ajo group", async () => {
    await program.methods
      .createAjoGroup(
        groupName,
        new anchor.BN(1000000), // security_deposit
        new anchor.BN(100000), // contribution_amount
        1, // contribution_interval
        7, // payout_interval
        3 // num_participants
      )
      .accountsStrict({
        globalState: globalStatePda,
        ajoGroup: groupPda,
        creator: creator.publicKey,
        creatorTokenAccount,
        groupTokenVault: groupVaultPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const group = await program.account.ajoGroup.fetch(groupPda);
    expect(group.name).to.equal(groupName);
    expect(group.numParticipants).to.equal(3);
    expect(group.participants.length).to.equal(1);
  });

  it("joins ajo group", async () => {
    await program.methods
      .joinAjoGroup()
      .accountsStrict({
        globalState: globalStatePda,
        ajoGroup: groupPda,
        participant: participant.publicKey,
        participantTokenAccount,
        groupTokenVault: groupVaultPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([participant])
      .rpc();

    const group = await program.account.ajoGroup.fetch(groupPda);
    expect(group.participants.length).to.equal(2);
  });

  it("fails to contribute before group starts", async () => {
    try {
      await program.methods
        .contribute()
        .accountsStrict({
          ajoGroup: groupPda,
          contributor: participant.publicKey,
          contributorTokenAccount: participantTokenAccount,
          groupTokenVault: groupVaultPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([participant])
        .rpc();
    } catch (err) {
      expect(err.message).to.contain("GroupNotStarted");
    }
  });

  const thirdParticipant = anchor.web3.Keypair.generate();
  let thirdTokenAccount: anchor.web3.PublicKey;

  it("third participant joins and group starts", async () => {
    thirdTokenAccount = await createAccount(
      provider.connection,
      creator.payer,
      mint,
      thirdParticipant.publicKey
    );

    await mintTo(
      provider.connection,
      creator.payer,
      mint,
      thirdTokenAccount,
      creator.publicKey,
      1_000_000_000
    );

    await provider.connection.requestAirdrop(thirdParticipant.publicKey, 2e9);

    await program.methods
      .joinAjoGroup()
      .accountsStrict({
        globalState: globalStatePda,
        ajoGroup: groupPda,
        participant: thirdParticipant.publicKey,
        participantTokenAccount: thirdTokenAccount,
        groupTokenVault: groupVaultPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([thirdParticipant])
      .rpc();

    const group = await program.account.ajoGroup.fetch(groupPda);
    expect(group.isActive).to.be.true;
    expect(group.participants.length).to.equal(3);
    expect(group.payoutIndex).to.equal(0);
  });

  it("participants contribute", async () => {
    const contrib = async (
      signer: anchor.web3.Keypair,
      tokenAccount: anchor.web3.PublicKey
    ) => {
      await program.methods
        .contribute()
        .accountsStrict({
          ajoGroup: groupPda,
          contributor: signer.publicKey,
          contributorTokenAccount: tokenAccount,
          groupTokenVault: groupVaultPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([signer])
        .rpc();
    };

    // Creator
    await contrib(creator.payer, creatorTokenAccount);
    // Participant 1
    await contrib(participant, participantTokenAccount);
    // Participant 2
    await contrib(thirdParticipant, thirdTokenAccount);

    const group = await program.account.ajoGroup.fetch(groupPda);
    expect(group.roundContributions.toNumber()).to.equal(3);
  });

  it("triggers payout to the first recipient", async () => {
    const recipientIndex = 0;
    const recipientKey = groupPda; // Just for logic ref — actual recipient is first participant

    const preVaultBalance = await getAccount(
      provider.connection,
      groupVaultPda
    );
    const preRecipientBalance = await getAccount(
      provider.connection,
      creatorTokenAccount
    );

    await program.methods
      .payout()
      .accountsStrict({
        ajoGroup: groupPda,
        groupTokenVault: groupVaultPda,
        recipientTokenAccount: creatorTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const postVaultBalance = await getAccount(
      provider.connection,
      groupVaultPda
    );
    const postRecipientBalance = await getAccount(
      provider.connection,
      creatorTokenAccount
    );

    expect(postRecipientBalance.amount > preRecipientBalance.amount).to.be.true;

    const group = await program.account.ajoGroup.fetch(groupPda);
    expect(group.payoutIndex).to.equal(1); // moved to next recipient
  });

  it("participant votes to close the group", async () => {
    await program.methods
      .closeAjoGroup()
      .accountsStrict({
        ajoGroup: groupPda,
        participant: participant.publicKey,
      })
      .signers([participant])
      .rpc();

    const group = await program.account.ajoGroup.fetch(groupPda);
    expect(group.votedToClose.length).to.equal(1);
  });

  it("participant claims refund after closure", async () => {
    // Manually set group to closed for test shortcut
    await program.methods
      .closeAjoGroup()
      .accountsStrict({
        ajoGroup: groupPda,
        participant: creator.publicKey,
      })
      .rpc();

    await program.methods
      .closeAjoGroup()
      .accountsStrict({
        ajoGroup: groupPda,
        participant: thirdParticipant.publicKey,
      })
      .signers([thirdParticipant])
      .rpc();

    const groupAfterClose = await program.account.ajoGroup.fetch(groupPda);
    expect(groupAfterClose.isClosed).to.be.true;

    // Claim refund
    const preBalance = await getAccount(
      provider.connection,
      participantTokenAccount
    );

    await program.methods
      .claimRefund()
      .accountsStrict({
        ajoGroup: groupPda,
        participant: participant.publicKey,
        participantTokenAccount: participantTokenAccount,
        groupTokenVault: groupVaultPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([participant])
      .rpc();

    const postBalance = await getAccount(
      provider.connection,
      participantTokenAccount
    );
    expect(postBalance.amount > preBalance.amount).to.be.true;
  });
});
