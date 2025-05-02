use anchor_lang::prelude::*;

#[account]
pub struct AjoGroup {
    // Basic group information
    pub name: String,                  // Unique name for the group
    pub contribution_amount: u64,      // Amount in USDC to contribute each round
    pub interval_in_days: u16,         // Time between rounds (in days)
    pub num_participants: u8,          // Total number of participants needed
    pub creator: Pubkey,               // Address of the group creator
    
    // Participants and round management
    pub participants: Vec<Pubkey>,     // List of all participants (ordered by join time)
    pub current_round: u8,             // Current round number (0-indexed)
    pub current_receiver_index: u8,    // Index of the participant receiving funds this round
    pub started: bool,                 // Whether the group has started
    pub completed: bool,               // Whether all rounds are completed
    
    // Tracking
    pub total_distributed: u64,        // Total amount distributed so far
    pub last_round_timestamp: i64,     // Timestamp of when the last round started
    pub bumps: u8,                     // PDA bump
}

impl AjoGroup {
    // Calculate space required for account
    pub fn calculate_size(name: &str) -> usize {
        // Space for fixed fields
        let fixed_size = 8 +  // account discriminator
                         (4 + name.len()) +  // name (string)
                         8 +  // contribution_amount (u64)
                         2 +  // interval_in_days (u16)
                         1 +  // num_participants (u8)
                         32 + // creator (Pubkey)
                         4 +  // participants vector length
                         1 +  // current_round (u8)
                         1 +  // current_receiver_index (u8)
                         1 +  // started (bool)
                         1 +  // completed (bool)
                         8 +  // total_distributed (u64)
                         8 +  // last_round_timestamp (i64)
                         1;   // bumps (u8)
        
        // Space for participants (max 20 participants)
        let participants_size = 20 * 32;  // Max 20 participants × 32 bytes per Pubkey
        
        fixed_size + participants_size
    }
}