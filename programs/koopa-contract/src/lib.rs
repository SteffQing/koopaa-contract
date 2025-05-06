//lib.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

pub mod errors;
pub mod state;
pub mod utils;

use errors::*;
use state::*;
use utils::*;

// This is your program's public key and it will update
// automatically when you build the project.

declare_id!("5upMRrwYFpvhkfmyUfb9Eun2EPWWu4XyBpkBLfUK2Tgm");

#[program]
mod koopa {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_percentage: u8) -> Result<()> {
        require!(fee_percentage <= 100, KooPaaError::InvalidFeePercentage);

        let global_state = &mut ctx.accounts.global_state;

        global_state.total_groups = 0;
        global_state.total_revenue = 0;
        global_state.active_groups = 0;
        global_state.completed_groups = 0;
        global_state.admin = ctx.accounts.admin.key();
        global_state.fee_percentage = fee_percentage;
        global_state.bumps = ctx.bumps.global_state;

        Ok(())
    }

    // Create a new Ajo group
    pub fn create_ajo_group(
        ctx: Context<CreateAjoGroup>,
        name: String,
        contribution_amount: u64,
        interval_in_days: u16,
        num_participants: u8,
    ) -> Result<()> {
        // Validate inputs
        require!(
            contribution_amount > 0,
            KooPaaError::InvalidContributionAmount
        );
        require!(
            interval_in_days > 0 && interval_in_days <= 90,
            KooPaaError::InvalidInterval
        );
        require!(
            num_participants >= 3 && num_participants <= 20,
            KooPaaError::InvalidParticipantCount
        );
        require!(name.len() <= 50, KooPaaError::NameTooLong);

        let group = &mut ctx.accounts.ajo_group;
        let creator = &ctx.accounts.creator;
        let global_state = &mut ctx.accounts.global_state;

        // Set group data
        group.name = name;
        group.contribution_amount = contribution_amount;
        group.interval_in_days = interval_in_days;
        group.num_participants = num_participants;
        group.creator = creator.key();
        group.participants = vec![];
        group.current_round = 0;
        group.started = false;
        group.completed = false;
        group.current_receiver_index = 0;
        group.total_distributed = 0;
        group.last_round_timestamp = 0;
        group.bumps = ctx.bumps.ajo_group;

        // Update global state
        global_state.total_groups += 1;
        global_state.active_groups += 1;

        Ok(())
    }

    // Join an existing group
    pub fn join_ajo_group(ctx: Context<JoinAjoGroup>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let participant = &ctx.accounts.participant;

        // Check if the group has already started
        require!(!group.started, KooPaaError::GroupAlreadyStarted);

        // Check if the group is already full
        require!(
            group.participants.len() < group.num_participants as usize,
            KooPaaError::GroupAlreadyFull
        );

        // Check if the participant is already in the group
        let already_joined = group
            .participants
            .iter()
            .any(|p| p.pubkey == participant.key());

        require!(!already_joined, KooPaaError::AlreadyJoined);

        // Store the current length before pushing to avoid borrowing issues
        let current_position = group.participants.len() as u8;

        // Add participant to the group with their data
        group.participants.push(AjoParticipant {
            pubkey: participant.key(),
            turn_number: current_position,
            claim_round: current_position, // Claim order based on join position
            claimed: false,
            claim_time: 0,
            claim_amount: 0,
            rounds_contributed: vec![],
            bump: 0,
        });

        Ok(())
    }

    // Start the Ajo group (can only be called by creator when required participants have joined)
    pub fn start_ajo_group(ctx: Context<StartAjoGroup>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let clock = Clock::get()?;

        // Check if it's the creator calling
        require!(
            group.creator == ctx.accounts.creator.key(),
            KooPaaError::OnlyCreatorCanStart
        );

        // Check if the group has required number of participants
        require!(
            group.participants.len() == group.num_participants as usize,
            KooPaaError::NotEnoughParticipants
        );

        // Check if the group has already started
        require!(!group.started, KooPaaError::GroupAlreadyStarted);

        // Mark group as started and set first round timestamp
        group.started = true;
        group.last_round_timestamp = clock.unix_timestamp;
        group.current_round = 0;

        // The first recipient is the person with claim_round == 0
        group.current_receiver_index = 0;

        Ok(())
    }

    // Make a contribution to the Ajo group
    pub fn contribute(ctx: Context<Contribute>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let contributor = &ctx.accounts.contributor;
        let global_state = &mut ctx.accounts.global_state;
        let clock = Clock::get()?;

        // Check if the group has started
        require!(group.started, KooPaaError::GroupNotStarted);

        // Check if the group has completed
        require!(!group.completed, KooPaaError::GroupCompleted);

        // Find the participant in the group
        let participant_index = group
            .participants
            .iter()
            .position(|p| p.pubkey == contributor.key())
            .ok_or(KooPaaError::NotParticipant)?;

        // Find the current recipient (the one whose claim_round matches current_round)
        let recipient_index = group
            .participants
            .iter()
            .position(|p| p.claim_round == group.current_round)
            .ok_or(KooPaaError::NotCurrentRecipient)?;

        let recipient_pubkey = group.participants[recipient_index].pubkey;

        // If contributor is the current recipient, they don't need to contribute
        if contributor.key() == recipient_pubkey {
            return Ok(());
        }

        // Check if already contributed to this round
        let already_contributed = group.participants[participant_index]
            .rounds_contributed
            .contains(&group.current_round);
        require!(!already_contributed, KooPaaError::AlreadyContributed);

        // Calculate fee (if any)
        let fee_amount = calculate_fee(group.contribution_amount, global_state.fee_percentage);
        let transfer_amount = group.contribution_amount - fee_amount;

        // Transfer tokens from contributor to the current recipient
        let transfer_accounts = Transfer {
            from: ctx.accounts.contributor_token_account.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: contributor.to_account_info(),
        };

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
            ),
            transfer_amount,
        )?;

        // If fee is configured, transfer fee to treasury
        if fee_amount > 0 {
            let fee_transfer = Transfer {
                from: ctx.accounts.contributor_token_account.to_account_info(),
                to: ctx.accounts.treasury_token_account.to_account_info(),
                authority: contributor.to_account_info(),
            };

            transfer(
                CpiContext::new(ctx.accounts.token_program.to_account_info(), fee_transfer),
                fee_amount,
            )?;

            // Update the global state with the fee
            global_state.total_revenue += fee_amount;
        }

        // Store the current round to avoid borrowing conflict
        let current_round = group.current_round;

        // Update participant's contribution record
        group.participants[participant_index]
            .rounds_contributed
            .push(current_round);

        Ok(())
    }

    // Claim payouts for the current round
    pub fn claim_round(ctx: Context<ClaimRound>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let recipient = &ctx.accounts.recipient;
        let clock = Clock::get()?;

        // Check if the group has started
        require!(group.started, KooPaaError::GroupNotStarted);

        // Check if the group has completed
        require!(!group.completed, KooPaaError::GroupCompleted);

        // Find the recipient in the group
        let recipient_index = group
            .participants
            .iter()
            .position(|p| p.pubkey == recipient.key())
            .ok_or(KooPaaError::NotParticipant)?;

        // Check if this is the recipient's turn
        require!(
            group.participants[recipient_index].claim_round == group.current_round,
            KooPaaError::NotCurrentRecipient
        );

        // Check if they've already claimed
        require!(
            !group.participants[recipient_index].claimed,
            KooPaaError::AlreadyClaimed
        );

        // Check if all participants have contributed
        let all_contributed = all_contributed(group);
        require!(all_contributed, KooPaaError::NotAllContributed);

        // Calculate the total amount to be claimed
        let claim_amount = calculate_round_total(group);

        // Update the recipient's claim data
        group.participants[recipient_index].claimed = true;
        group.participants[recipient_index].claim_time = clock.unix_timestamp;
        group.participants[recipient_index].claim_amount = claim_amount;

        // Update group stats
        group.total_distributed += claim_amount;

        Ok(())
    }

    // Move to the next round (can only be called by creator after interval has passed)
    pub fn next_round(ctx: Context<NextRound>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let global_state = &mut ctx.accounts.global_state;
        let clock = Clock::get()?;

        // Check if it's the creator calling
        require!(
            group.creator == ctx.accounts.creator.key(),
            KooPaaError::OnlyCreatorCanStart
        );

        // Check if the group has started
        require!(group.started, KooPaaError::GroupNotStarted);

        // Check if the group has completed
        require!(!group.completed, KooPaaError::GroupCompleted);

        // Check if interval has passed
        let current_time = clock.unix_timestamp;
        let interval_seconds = days_to_seconds(group.interval_in_days);
        require!(
            current_time >= group.last_round_timestamp + interval_seconds,
            KooPaaError::IntervalNotPassed
        );

        // Update round information
        group.current_round += 1;
        group.last_round_timestamp = current_time;

        // Check if all rounds are completed
        if group.current_round >= group.num_participants {
            group.completed = true;
            global_state.active_groups -= 1;
            global_state.completed_groups += 1;
        }

        Ok(())
    }

    // Update global state settings (admin only)
    pub fn update_global_settings(
        ctx: Context<UpdateGlobalSettings>,
        fee_percentage: u8,
    ) -> Result<()> {
        require!(fee_percentage <= 100, KooPaaError::InvalidFeePercentage);

        let global_state = &mut ctx.accounts.global_state;
        global_state.fee_percentage = fee_percentage;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = GlobalState::SIZE,
        seeds = [b"global-state"],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateAjoGroup<'info> {
    #[account(
        init,
        payer = creator,
        space = AjoGroup::calculate_size(&name),
        seeds = [b"ajo-group", name.as_bytes()],
        bump
    )]
    pub ajo_group: Account<'info, AjoGroup>,

    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"global-state"],
        bump = global_state.bumps
    )]
    pub global_state: Account<'info, GlobalState>,

    pub token_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinAjoGroup<'info> {
    #[account(mut)]
    pub ajo_group: Account<'info, AjoGroup>,

    pub participant: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartAjoGroup<'info> {
    #[account(mut)]
    pub ajo_group: Account<'info, AjoGroup>,

    #[account(constraint = ajo_group.creator == creator.key())]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Contribute<'info> {
    #[account(mut)]
    pub ajo_group: Account<'info, AjoGroup>,

    pub contributor: Signer<'info>,

    #[account(
        mut,
        constraint = contributor_token_account.owner == contributor.key(),
        constraint = contributor_token_account.mint == token_mint.key(),
    )]
    pub contributor_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.mint == token_mint.key(),
        constraint = recipient_token_account.owner == ajo_group.participants[ajo_group.current_receiver_index as usize].pubkey,

    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = treasury_token_account.mint == token_mint.key(),
        constraint = treasury_token_account.owner == global_state.admin,
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"global-state"],
        bump = global_state.bumps
    )]
    pub global_state: Account<'info, GlobalState>,

    pub token_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRound<'info> {
    #[account(mut)]
    pub ajo_group: Account<'info, AjoGroup>,

    pub recipient: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct NextRound<'info> {
    #[account(mut)]
    pub ajo_group: Account<'info, AjoGroup>,

    #[account(constraint = ajo_group.creator == creator.key())]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"global-state"],
        bump = global_state.bumps
    )]
    pub global_state: Account<'info, GlobalState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateGlobalSettings<'info> {
    #[account(
        mut,
        seeds = [b"global-state"],
        bump = global_state.bumps,
        constraint = global_state.admin == admin.key() @ KooPaaError::OnlyAdminCanUpdate
    )]
    pub global_state: Account<'info, GlobalState>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}
