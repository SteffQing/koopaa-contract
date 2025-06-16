//lib.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

pub mod errors;
pub mod events;
pub mod state;
pub mod utils;

use errors::*;
use events::*;
use state::*;
use utils::*;

// This is your program's public key and it will update
// automatically when you build the project.

declare_id!("33NAzyKNuayyqKNW6QMXbNT69CikAhCUhPbgwZn1LR3o");

#[program]
mod koopa {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;

        global_state.total_groups = 0;
        global_state.active_groups = 0;

        global_state.bumps = ctx.bumps.global_state;

        Ok(())
    }

    pub fn create_ajo_group(
        ctx: Context<CreateAjoGroup>,
        name: String,
        security_deposit: u64,
        contribution_amount: u64,
        contribution_interval: u8,
        payout_interval: u8,
        num_participants: u8,
    ) -> Result<()> {
        require!(
            contribution_amount > 0,
            KooPaaError::InvalidContributionAmount
        );
        require!(
            contribution_interval > 0 && contribution_interval <= 90,
            KooPaaError::InvalidInterval
        );
        require!(
            payout_interval >= 7 && payout_interval <= 90,
            KooPaaError::InvalidInterval
        );
        require!(
            payout_interval >= contribution_interval,
            KooPaaError::InvalidInterval
        );
        require!(
            num_participants >= 3 && num_participants <= 20,
            KooPaaError::InvalidParticipantCount
        );
        require!(name.len() <= 50, KooPaaError::NameTooLong);
        require!(security_deposit > 0, KooPaaError::InvalidSecurityDeposit);

        let transfer_accounts = Transfer {
            from: ctx.accounts.creator_token_account.to_account_info(),
            to: ctx.accounts.group_token_vault.to_account_info(),
            authority: ctx.accounts.creator.to_account_info(),
        };

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
            ),
            security_deposit,
        )?;

        let group = &mut ctx.accounts.ajo_group;
        let creator = &ctx.accounts.creator;
        let global_state = &mut ctx.accounts.global_state;
        let clock = Clock::get()?;

        let interval = payout_interval as f64 / contribution_interval as f64;
        let round_payout_interval = interval.ceil() as u8 * contribution_interval;

        group.name = name.clone();
        group.contribution_amount = contribution_amount;
        group.contribution_interval = contribution_interval;
        group.security_deposit = security_deposit;
        group.payout_interval = round_payout_interval;
        group.num_participants = num_participants;

        group.participants = vec![AjoParticipant {
            pubkey: creator.key(),
            contribution_round: 0,
            refund_amount: 0,
        }];
        group.payout_round = 0;
        group.start_timestamp = None;
        group.close_votes = vec![];
        group.is_closed = false;

        let (_group_pda, group_bump) =
            Pubkey::find_program_address(&[b"ajo-group", group.name.as_bytes()], ctx.program_id);
        let (_vault_pda, vault_bump) =
            Pubkey::find_program_address(&[b"group-vault", group.key().as_ref()], ctx.program_id);

        group.bumps = group_bump;
        group.vault_bump = vault_bump;

        global_state.total_groups += 1;

        emit!(AjoGroupCreatedEvent {
            group_name: name.clone(),
            security_deposit,
            contribution_amount,
            num_participants,
            contribution_interval,
            payout_interval: round_payout_interval,
        });

        emit!(ParticipantJoinedEvent {
            group_name: name.clone(),
            participant: creator.key(),
            join_timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    pub fn join_ajo_group(ctx: Context<JoinAjoGroup>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let global_state = &mut ctx.accounts.global_state;
        let participant = &ctx.accounts.participant;
        let clock = Clock::get()?;

        require!(
            group.start_timestamp.is_none(),
            KooPaaError::GroupAlreadyStarted
        );

        let already_joined = group
            .participants
            .iter()
            .any(|p| p.pubkey == participant.key());

        require!(!already_joined, KooPaaError::AlreadyJoined);

        let security_deposit = group.security_deposit;
        let transfer_accounts = Transfer {
            from: ctx.accounts.participant_token_account.to_account_info(),
            to: ctx.accounts.group_token_vault.to_account_info(),
            authority: participant.to_account_info(),
        };

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
            ),
            security_deposit,
        )?;

        group.participants.push(AjoParticipant {
            pubkey: participant.key(),
            contribution_round: 0,
            refund_amount: 0,
        });

        let group_name = group.name.clone();

        if group.participants.len() == group.num_participants as usize {
            group.start_timestamp = Some(clock.unix_timestamp);
            global_state.active_groups += 1;
            emit!(AjoGroupStartedEvent {
                group_name,
                start_timestamp: clock.unix_timestamp
            });
        }

        emit!(ParticipantJoinedEvent {
            group_name: group.name.clone(),
            participant: participant.key(),
            join_timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    pub fn contribute(ctx: Context<Contribute>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let contributor = &ctx.accounts.contributor;
        let clock = Clock::get()?;

        require!(
            group.start_timestamp.is_some(),
            KooPaaError::GroupNotStarted
        );

        let start_timestamp = group.start_timestamp.unwrap();
        let contribution_interval = group.contribution_interval;
        let contribution_amount = group.contribution_amount;

        let participant = group
            .participants
            .iter_mut()
            .find(|p| p.pubkey == contributor.key())
            .ok_or(KooPaaError::NotParticipant)?;

        let time_since_start = clock.unix_timestamp - start_timestamp;
        let contribution_interval_seconds = contribution_interval as i64 * 86400;
        let current_round = (time_since_start / contribution_interval_seconds) as u16;

        let last_paid_round = participant.contribution_round;
        require!(
            last_paid_round < current_round,
            KooPaaError::AlreadyContributed
        );

        let rounds_missed = current_round - last_paid_round;
        let transfer_amount = contribution_amount * rounds_missed as u64;

        let transfer_accounts = Transfer {
            from: ctx.accounts.contributor_token_account.to_account_info(),
            to: ctx.accounts.group_token_vault.to_account_info(),
            authority: contributor.to_account_info(),
        };

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
            ),
            transfer_amount,
        )?;

        participant.contribution_round = current_round;

        emit!(ContributionMadeEvent {
            group_name: group.name.clone(),
            contributor: contributor.key(),
            contribution_amount: transfer_amount,
            current_round,
        });

        Ok(())
    }

    pub fn payout(ctx: Context<Payout>) -> Result<()> {
        let authority_info = ctx.accounts.ajo_group.to_account_info();
        let group = &mut ctx.accounts.ajo_group;
        let clock = Clock::get()?;

        require!(
            ctx.accounts.recipient.mint == ctx.accounts.token_mint.key(),
            KooPaaError::InvalidTokenAccountMint
        );

        let start_timestamp = group.start_timestamp.ok_or(KooPaaError::GroupNotStarted)?;
        let time_since_start = clock.unix_timestamp - start_timestamp;

        let required_contributions_per_payout = group.payout_interval / group.contribution_interval;
        let min_required_contribution_rounds =
            (group.payout_round + 1) * required_contributions_per_payout as u16;

        for p in &group.participants {
            require!(
                p.contribution_round >= min_required_contribution_rounds,
                KooPaaError::NotAllContributed
            );
        }

        let payout_interval_secs = group.payout_interval as i64 * 86400;
        let expected_payout_round = (time_since_start / payout_interval_secs) as u16;

        require!(
            group.payout_round < expected_payout_round,
            KooPaaError::PayoutNotYetDue
        );

        let num_participants = group.participants.len() as u8;
        let recipient_index = (group.payout_round as usize) % (num_participants as usize);
        let recipient_pubkey = group.participants[recipient_index].pubkey;

        require!(
            recipient_pubkey == ctx.accounts.recipient.owner,
            KooPaaError::NotCurrentRecipient
        );

        let group_name = group.name.clone();
        let signer_seeds = &[b"ajo-group", group_name.as_bytes(), &[group.bumps]];

        let payout_amount = group.contribution_amount
            * (num_participants as u64)
            * (required_contributions_per_payout as u64);
        let transfer_accounts = Transfer {
            from: ctx.accounts.group_token_vault.to_account_info(),
            to: ctx.accounts.recipient.to_account_info(),
            authority: authority_info,
        };

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
                &[signer_seeds],
            ),
            payout_amount,
        )?;

        group.payout_round += 1;

        emit!(PayoutMadeEvent {
            group_name,
            recipient: recipient_pubkey,
            payout_amount,
            payout_round: group.payout_round,
        });

        Ok(())
    }

    pub fn close_ajo_group(ctx: Context<CloseAjoGroup>) -> Result<()> {
        let group = &mut ctx.accounts.ajo_group;
        let participant = &ctx.accounts.participant;
        let global_state = &mut ctx.accounts.global_state;

        if group.is_closed {
            return err!(KooPaaError::GroupAlreadyClosed);
        }

        let is_participant = group
            .participants
            .iter()
            .any(|p| p.pubkey == participant.key());

        require!(is_participant, KooPaaError::NotParticipant);

        let already_voted = group.close_votes.contains(&participant.key());
        require!(!already_voted, KooPaaError::AlreadyVotedToClose);

        group.close_votes.push(participant.key());

        let total_participants = group.participants.len();
        let total_votes = group.close_votes.len();

        let group_started = group.start_timestamp.is_some();
        let group_contribution_amount = group.contribution_amount;
        let group_security_deposit = group.security_deposit;

        if total_votes * 2 > total_participants {
            let minimum_common_contribution_round = group
                .participants
                .iter()
                .map(|p| p.contribution_round)
                .min()
                .unwrap();

            for participant in group.participants.iter_mut() {
                let contribution_refund = if group_started {
                    let refundable_rounds =
                        participant.contribution_round - minimum_common_contribution_round;
                    group_contribution_amount * refundable_rounds as u64
                } else {
                    0
                };

                participant.refund_amount = group_security_deposit + contribution_refund;
            }

            if group_started && global_state.active_groups > 0 {
                global_state.active_groups -= 1;
            }
            group.is_closed = true;

            emit!(AjoGroupClosedEvent {
                group_name: group.name.clone(),
                total_votes: total_votes as u8,
                group_size: total_participants as u8,
            });
        }

        Ok(())
    }

    pub fn claim_refund(ctx: Context<ClaimRefund>) -> Result<()> {
        let authority_info = ctx.accounts.ajo_group.to_account_info();
        let group = &mut ctx.accounts.ajo_group;
        let participant_key = ctx.accounts.participant.key();

        require!(group.is_closed, KooPaaError::GroupNotClosed);

        let participant_index = group
            .participants
            .iter()
            .position(|p| p.pubkey == participant_key)
            .ok_or(KooPaaError::NotParticipant)?;

        let refund_amount = group.participants[participant_index].refund_amount;
        require!(refund_amount > 0, KooPaaError::NoRefundToClaim);

        let transfer_accounts = Transfer {
            from: ctx.accounts.group_token_vault.to_account_info(),
            to: ctx.accounts.participant_token_account.to_account_info(),
            authority: authority_info,
        };

        let group_name = group.name.clone();
        let signer_seeds = &[b"ajo-group", group_name.as_bytes(), &[group.bumps]];

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
                &[signer_seeds],
            ),
            refund_amount,
        )?;

        // Mark refund claimed
        group.participants[participant_index].refund_amount = 0;

        emit!(RefundClaimedEvent {
            group_name: group.name.clone(),
            participant: participant_key,
            amount: refund_amount,
        });

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
#[instruction(
    name: String,
    security_deposit: u64,
    contribution_amount: u64,
    contribution_interval: u8,
    payout_interval: u8,
    num_participants: u8
)]
pub struct CreateAjoGroup<'info> {
    #[account(
        init,
        payer = creator,
        space = AjoGroup::calculate_size(&name, num_participants),
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

    #[account(
        mut,
        constraint = creator_token_account.owner == creator.key(),
        constraint = creator_token_account.mint == token_mint.key()
    )]
    pub creator_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        seeds = [b"group-vault", ajo_group.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = ajo_group
    )]
    pub group_token_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct JoinAjoGroup<'info> {
    #[account(
        mut,
        seeds = [b"ajo-group", ajo_group.name.as_bytes()],
        bump = ajo_group.bumps
    )]
    pub ajo_group: Account<'info, AjoGroup>,

    pub participant: Signer<'info>,

    #[account(
        mut,
        seeds = [b"global-state"],
        bump = global_state.bumps
    )]
    pub global_state: Account<'info, GlobalState>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = participant_token_account.owner == participant.key(),
        constraint = participant_token_account.mint == token_mint.key()
    )]
    pub participant_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"group-vault", ajo_group.key().as_ref()],
        bump = ajo_group.vault_bump,
    )]
    pub group_token_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Contribute<'info> {
    #[account(
        mut,
        seeds = [b"ajo-group", ajo_group.name.as_bytes()],
        bump = ajo_group.bumps
    )]
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
        seeds = [b"group-vault", ajo_group.key().as_ref()],
        bump = ajo_group.vault_bump,
    )]
    pub group_token_vault: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Payout<'info> {
    #[account(
        mut,
        seeds = [b"ajo-group", ajo_group.name.as_bytes()],
        bump = ajo_group.bumps
    )]
    pub ajo_group: Account<'info, AjoGroup>,

    #[account(
        mut,
        seeds = [b"group-vault", ajo_group.key().as_ref()],
        bump = ajo_group.vault_bump,
    )]
    pub group_token_vault: Account<'info, TokenAccount>,

    /// The recipient who will receive tokens (does NOT have to sign)
    #[account(mut)]
    pub recipient: Account<'info, TokenAccount>,

    pub caller: Signer<'info>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CloseAjoGroup<'info> {
    #[account(
        mut,
        seeds = [b"ajo-group", ajo_group.name.as_bytes()],
        bump = ajo_group.bumps
    )]
    pub ajo_group: Account<'info, AjoGroup>,

    pub participant: Signer<'info>,

    #[account(
        mut,
        seeds = [b"global-state"],
        bump = global_state.bumps
    )]
    pub global_state: Account<'info, GlobalState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRefund<'info> {
    #[account(
        mut,
        seeds = [b"ajo-group", ajo_group.name.as_bytes()],
        bump = ajo_group.bumps
    )]
    pub ajo_group: Account<'info, AjoGroup>,

    #[account(
        mut,
        seeds = [b"group-vault", ajo_group.key().as_ref()],
        bump = ajo_group.vault_bump,
    )]
    pub group_token_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub participant: Signer<'info>,

    #[account(
        mut,
        constraint = participant_token_account.owner == participant.key(),
        constraint = participant_token_account.mint == token_mint.key(),
    )]
    pub participant_token_account: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}
