use anchor_lang::prelude::*;
use anchor_spl::token::{ transfer, Mint, Token, TokenAccount, Transfer};

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
mod koopa_contract {
    use super::*;

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

        // Set group data
        group.name = name;
        group.contribution_amount = contribution_amount;
        group.interval_in_days = interval_in_days;
        group.num_participants = num_participants;
        group.creator = creator.key();
        group.participants = vec![creator.key()];
        group.current_round = 0;
        group.started = false;
        group.completed = false;
        group.current_receiver_index = 0;
        group.total_distributed = 0;
        group.last_round_timestamp = 0;
        group.bumps = ctx.bumps.ajo_group;

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
        require!(
            !group.participants.contains(&participant.key()),
            KooPaaError::AlreadyJoined
        );

        // Add participant to the group
        group.participants.push(participant.key());

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

        // Set the order - simple first-come-first-serve based on join order
        // (participants array order is the order of payout)

        Ok(())
    }


    // Make a contribution to the Ajo group
    pub fn contribute(ctx: Context<Contribute>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let contributor = &ctx.accounts.contributor;
        // let clock = Clock::get()?;

        // Check if the group has started
        require!(group.started, KooPaaError::GroupNotStarted);
        
        // Check if the group has completed
        require!(!group.completed, KooPaaError::GroupCompleted);

        // Check if the contributor is a participant
        require!(
            group.participants.contains(&contributor.key()),
            KooPaaError::NotAParticipant
        );

        // Check if the current round is valid
        let current_recipient = group.participants[group.current_receiver_index as usize];

        // If the contributor is the current recipient, they don't need to contribute
        if contributor.key() == current_recipient {
            return Ok(());
        }

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
            group.contribution_amount,
        )?;

        Ok(())
    }

     // Move to the next round (can only be called by creator after interval has passed)
    pub fn next_round(ctx: Context<NextRound>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
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
        let interval_seconds = (group.interval_in_days as i64) * 24 * 60 * 60;
        require!(
            current_time >= group.last_round_timestamp + interval_seconds,
            KooPaaError::IntervalNotPassed
        );

        // Update the total distributed amount
        group.total_distributed += group.contribution_amount * (group.num_participants as u64 - 1);
        
        // Update current round and timestamp
        group.current_round += 1;
        group.last_round_timestamp = current_time;
        
        // Move to next recipient
        group.current_receiver_index += 1;
        
        // Check if all rounds are completed
        if group.current_round >= group.num_participants {
            group.completed = true;
        }

        Ok(())
    }

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

   #[account(mut)]
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
        constraint = recipient_token_account.owner == ajo_group.participants[ajo_group.current_receiver_index as usize],
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct NextRound<'info> {
    #[account(mut)]
    pub ajo_group: Account<'info, AjoGroup>,

    #[account(constraint = ajo_group.creator == creator.key())]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}
