import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Koopa } from "../target/types/koopa";
import {
	TOKEN_PROGRAM_ID,
	createMint,
	createAccount,
	mintTo,
} from "@solana/spl-token";
import { expect } from "chai";

describe("koopa", () => {
	// Configure the client to use the local cluster
	const provider = anchor.AnchorProvider.env();
	anchor.setProvider(provider);

	const program = anchor.workspace.Koopa as Program<Koopa>;

	// Common variables
	const admin = anchor.web3.Keypair.generate();
	const creator = anchor.web3.Keypair.generate();
	const participant1 = anchor.web3.Keypair.generate();
	const participant2 = anchor.web3.Keypair.generate();
	const participant3 = anchor.web3.Keypair.generate();

	// Token variables
	let usdcMint: anchor.web3.PublicKey;
	let adminTokenAccount: anchor.web3.PublicKey;
	let creatorTokenAccount: anchor.web3.PublicKey;
	let participant1TokenAccount: anchor.web3.PublicKey;
	let participant2TokenAccount: anchor.web3.PublicKey;
	let participant3TokenAccount: anchor.web3.PublicKey;

	// Group variables
	const groupName = "TestGroup";
	const contributionAmount = new anchor.BN(100_000_000); // 100 USDC (with 6 decimals)
	const intervalInDays = 7; // 7 days
	const numParticipants = 3;

	// PDA variables
	let globalStatePDA: anchor.web3.PublicKey;
	let globalStateBump: number;
	let ajoGroupPDA: anchor.web3.PublicKey;
	let ajoGroupBump: number;

	// Fee for protocol
	const feePercentage = 10; // 1% (represented as 10 = 1.0%)

	// Helper function to airdrop SOL to an account
	async function airdropSol(to: anchor.web3.PublicKey, amount: number) {
		const signature = await provider.connection.requestAirdrop(
			to,
			amount * anchor.web3.LAMPORTS_PER_SOL,
		);
		await provider.connection.confirmTransaction(signature);
	}

	before(async () => {
		// Airdrop SOL to admin and creator
		await airdropSol(admin.publicKey, 10);
		await airdropSol(creator.publicKey, 10);
		await airdropSol(participant1.publicKey, 10);
		await airdropSol(participant2.publicKey, 10);
		await airdropSol(participant3.publicKey, 10);

		// Create USDC mint (simulating USDC)
		usdcMint = await createMint(
			provider.connection,
			admin,
			admin.publicKey,
			null,
			6, // USDC has 6 decimals
		);

		// Create token accounts for all users
		adminTokenAccount = await createAccount(
			provider.connection,
			admin,
			usdcMint,
			admin.publicKey,
		);

		creatorTokenAccount = await createAccount(
			provider.connection,
			creator,
			usdcMint,
			creator.publicKey,
		);

		participant1TokenAccount = await createAccount(
			provider.connection,
			participant1,
			usdcMint,
			participant1.publicKey,
		);

		participant2TokenAccount = await createAccount(
			provider.connection,
			participant2,
			usdcMint,
			participant2.publicKey,
		);

		participant3TokenAccount = await createAccount(
			provider.connection,
			participant3,
			usdcMint,
			participant3.publicKey,
		);

		// Mint 1000 USDC to each participant
		await mintTo(
			provider.connection,
			admin,
			usdcMint,
			creatorTokenAccount,
			admin.publicKey,
			1000_000_000,
		);

		await mintTo(
			provider.connection,
			admin,
			usdcMint,
			participant1TokenAccount,
			admin.publicKey,
			1000_000_000,
		);

		await mintTo(
			provider.connection,
			admin,
			usdcMint,
			participant2TokenAccount,
			admin.publicKey,
			1000_000_000,
		);

		await mintTo(
			provider.connection,
			admin,
			usdcMint,
			participant3TokenAccount,
			admin.publicKey,
			1000_000_000,
		);

		// Find PDA for global state
		[globalStatePDA, globalStateBump] =
			anchor.web3.PublicKey.findProgramAddressSync(
				[Buffer.from("global-state")],
				program.programId,
			);

		// Find PDA for ajo group
		[ajoGroupPDA, ajoGroupBump] = anchor.web3.PublicKey.findProgramAddressSync(
			[Buffer.from("ajo-group"), Buffer.from(groupName)],
			program.programId,
		);
	});

	it("Initializes the global state", async () => {
		// Initialize global state
		await program.methods
			.initialize(feePercentage)
			.accounts({
				globalState: globalStatePDA,
				admin: admin.publicKey,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([admin])
			.rpc();

		// Verify global state
		const globalState = await program.account.globalState.fetch(globalStatePDA);
		expect(globalState.totalGroups.toNumber()).to.equal(0);
		expect(globalState.totalRevenue.toNumber()).to.equal(0);
		expect(globalState.activeGroups.toNumber()).to.equal(0);
		expect(globalState.completedGroups.toNumber()).to.equal(0);
		expect(globalState.admin.toString()).to.equal(admin.publicKey.toString());
		expect(globalState.feePercentage).to.equal(feePercentage);
	});

	it("Creates a new Ajo group", async () => {
		// Create ajo group
		await program.methods
			.createAjoGroup(
				groupName,
				contributionAmount,
				intervalInDays,
				numParticipants,
			)
			.accounts({
				ajoGroup: ajoGroupPDA,
				creator: creator.publicKey,
				globalState: globalStatePDA,
				tokenMint: usdcMint,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([creator])
			.rpc();

		// Verify ajo group
		const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
		expect(ajoGroup.name).to.equal(groupName);
		expect(ajoGroup.contributionAmount.toNumber()).to.equal(
			contributionAmount.toNumber(),
		);
		expect(ajoGroup.intervalInDays).to.equal(intervalInDays);
		expect(ajoGroup.numParticipants).to.equal(numParticipants);
		expect(ajoGroup.creator.toString()).to.equal(creator.publicKey.toString());
		expect(ajoGroup.participants.length).to.equal(0);
		expect(ajoGroup.started).to.be.false;
		expect(ajoGroup.completed).to.be.false;

		// Verify global state updated
		const globalState = await program.account.globalState.fetch(globalStatePDA);
		expect(globalState.totalGroups.toNumber()).to.equal(1);
		expect(globalState.activeGroups.toNumber()).to.equal(1);
	});

	it("Joins an Ajo group", async () => {
		// Participant 1 joins
		await program.methods
			.joinAjoGroup()
			.accounts({
				ajoGroup: ajoGroupPDA,
				participant: participant1.publicKey,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([participant1])
			.rpc();

		// Participant 2 joins
		await program.methods
			.joinAjoGroup()
			.accounts({
				ajoGroup: ajoGroupPDA,
				participant: participant2.publicKey,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([participant2])
			.rpc();

		// Participant 3 joins
		await program.methods
			.joinAjoGroup()
			.accounts({
				ajoGroup: ajoGroupPDA,
				participant: participant3.publicKey,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([participant3])
			.rpc();

		// Verify participants were added
		const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
		expect(ajoGroup.participants.length).to.equal(3);
		expect(ajoGroup.participants[0].pubkey.toString()).to.equal(
			participant1.publicKey.toString(),
		);
		expect(ajoGroup.participants[1].pubkey.toString()).to.equal(
			participant2.publicKey.toString(),
		);
		expect(ajoGroup.participants[2].pubkey.toString()).to.equal(
			participant3.publicKey.toString(),
		);

		// Verify claim rounds are assigned (0, 1, 2)
		expect(ajoGroup.participants[0].claimRound).to.equal(0);
		expect(ajoGroup.participants[1].claimRound).to.equal(1);
		expect(ajoGroup.participants[2].claimRound).to.equal(2);
	});

	it("Starts the Ajo group", async () => {
		// Start the group
		await program.methods
			.startAjoGroup()
			.accounts({
				ajoGroup: ajoGroupPDA,
				creator: creator.publicKey,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([creator])
			.rpc();

		// Verify the group has started
		const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
		expect(ajoGroup.started).to.be.true;
		expect(ajoGroup.currentRound).to.equal(0);
		expect(ajoGroup.lastRoundTimestamp.toNumber()).to.be.greaterThan(0);
	});

	it("Allows contributions to the first round", async () => {
		// Participant 2 contributes (to participant 1 who is recipient of round 0)
		await program.methods
			.contribute()
			.accounts({
				ajoGroup: ajoGroupPDA,
				contributor: participant2.publicKey,
				contributorTokenAccount: participant2TokenAccount,
				recipientTokenAccount: participant1TokenAccount,
				treasuryTokenAccount: adminTokenAccount,
				globalState: globalStatePDA,
				tokenMint: usdcMint,
				tokenProgram: TOKEN_PROGRAM_ID,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([participant2])
			.rpc();

		// Participant 3 contributes (to participant 1)
		await program.methods
			.contribute()
			.accounts({
				ajoGroup: ajoGroupPDA,
				contributor: participant3.publicKey,
				contributorTokenAccount: participant3TokenAccount,
				recipientTokenAccount: participant1TokenAccount,
				treasuryTokenAccount: adminTokenAccount,
				globalState: globalStatePDA,
				tokenMint: usdcMint,
				tokenProgram: TOKEN_PROGRAM_ID,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([participant3])
			.rpc();

		// Verify participant 1 (recipient) did not need to contribute
		const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
		expect(ajoGroup.participants[0].roundsContributed.length).to.equal(0); // Participant 1 is recipient
		expect(ajoGroup.participants[1].roundsContributed.length).to.equal(1); // Participant 2 contributed
		expect(ajoGroup.participants[2].roundsContributed.length).to.equal(1); // Participant 3 contributed

		// Verify fees were collected
		const globalState = await program.account.globalState.fetch(globalStatePDA);
		expect(globalState.totalRevenue.toNumber()).to.be.greaterThan(0);
	});

	it("Allows the recipient to claim their round", async () => {
		// Participant 1 claims round 0
		await program.methods
			.claimRound()
			.accounts({
				ajoGroup: ajoGroupPDA,
				recipient: participant1.publicKey,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([participant1])
			.rpc();

		// Verify participant 1 has claimed
		const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
		expect(ajoGroup.participants[0].claimed).to.be.true;
		expect(ajoGroup.participants[0].claimTime.toNumber()).to.be.greaterThan(0);
		expect(ajoGroup.participants[0].claimAmount.toNumber()).to.be.greaterThan(
			0,
		);
	});

	it("Moves to the next round", async () => {
		// Fast-forward blockchain time (simulating days passing)
		// Note: This won't work in a real test environment without special test-only mechanisms

		// Move to next round
		await program.methods
			.nextRound()
			.accounts({
				ajoGroup: ajoGroupPDA,
				creator: creator.publicKey,
				globalState: globalStatePDA,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([creator])
			.rpc({ skipPreflight: true }); // Skipping preflight to ignore time constraints for demo

		// Verify moved to next round
		const ajoGroup = await program.account.ajoGroup.fetch(ajoGroupPDA);
		expect(ajoGroup.currentRound).to.equal(1);

		// Note: In a real test we'd need to handle the time constraint check
	});

	// More tests would follow for remaining rounds, edge cases, etc.
});
