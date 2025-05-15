use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AjoParticipant {
    pub pubkey: Pubkey,
    pub contribution_round: u16,
    pub refund_amount: u64,
}

#[account]
pub struct AjoGroup {
    // Basic group information
    pub name: String,              // Unique name for the group
    pub security_deposit: u64,     // Amount in USDC to join this group
    pub contribution_amount: u64,  // Amount in USDC to contribute each round
    pub contribution_interval: u8, // Time between rounds when a user should pay (in days)
    pub payout_interval: u8,       // Time between payouts (in days)
    pub num_participants: u8,      // Total number of participants needed

    // Participants and round management
    pub participants: Vec<AjoParticipant>, // List of all participants (ordered by join time)
    pub start_timestamp: Option<i64>,
    pub payout_round: u16, // state for payouts made, useful in calc current round, index of recipient

    pub close_votes: Vec<Pubkey>, // Track who has voted to close
    pub is_closed: bool,

    pub vault_bump: u8,
    pub bumps: u8, // PDA bump
}

impl AjoGroup {
    // Calculate space required for account
    pub fn calculate_size(name: &str, num_participants: u8) -> usize {
        // Space for fixed fields
        let fixed_size = 8 +  // account discriminator
                         (4 + name.len()) +  // name (string)
                         8 +  // security_deposit (u64)
                         8 +  // contribution_amount (u64)
                         1 +  // contribution_interval (u8)
                         1 +  // payout_interval (u8)
                         1 +  // num_participants (u8)
                         4 +  // participants vector length
                         8 + 1 + // start_timestamp -> FIX if Optional has its bumps (i64)| Yes it does: 1
                         2 +  // payout_round (u16)
                         4 + (num_participants as usize * 32) + // close_votes vector + max pubkeys
                         1 +  // is_closed (bool)
                         1 + // vault_bump (u8)
                         1; // bumps (u8)

        // Space for participants (with all their data)
        // Each participant has: pubkey (32) + contribution_round (2) and refund_amount (8)
        let participant_size = 32 + 2 + 8; // ~42 bytes per participant
        let participants_size = num_participants as usize * participant_size; // Max 20 participants

        fixed_size + participants_size
    }
}

#[account]
pub struct GlobalState {
    pub total_groups: u64,  // Total number of groups created
    pub active_groups: u64, // Number of currently active groups
    pub bumps: u8,          // PDA bump
}

impl GlobalState {
    pub const SIZE: usize = 8 +    // discriminator
                            8 +    // total_groups
                            8 +    // active_groups
                            1; // bumps
}
