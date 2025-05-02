use anchor_lang::prelude::*;
use crate::state::*;

// Helper function to find the PDA for an Ajo group
pub fn find_group_pda(name: &str, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"ajo-group", name.as_bytes()],
        program_id,
    )
}

// Convert days to seconds
pub fn days_to_seconds(days: u16) -> i64 {
    (days as i64) * 24 * 60 * 60
}

// Check if a participant is due to contribute in the current round
pub fn should_contribute(group: &AjoGroup, participant: &Pubkey) -> bool {
    // If the participant is the current recipient, they don't need to contribute
    if *participant == group.participants[group.current_receiver_index as usize] {
        return false;
    }
    
    // Otherwise, they should contribute
    true
}

// Calculate the total group contribution per round
pub fn calculate_round_total(group: &AjoGroup) -> u64 {
    // Total contribution is the contribution amount times the number of contributors
    // (which is all participants except the current recipient)
    group.contribution_amount * ((group.num_participants - 1) as u64)
}