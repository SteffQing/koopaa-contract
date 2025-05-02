use anchor_lang::prelude::*;

#[error_code]
pub enum KooPaaError {
    #[msg("Contribution amount must be greater than zero")]
    InvalidContributionAmount,
    
    #[msg("Interval must be between 1 and 90 days")]
    InvalidInterval,
    
    #[msg("Number of participants must be between 3 and 20")]
    InvalidParticipantCount,
    
    #[msg("Group name is too long (maximum 50 characters)")]
    NameTooLong,
    
    #[msg("Group has already started")]
    GroupAlreadyStarted,
    
    #[msg("Group is already full")]
    GroupAlreadyFull,
    
    #[msg("You have already joined this group")]
    AlreadyJoined,
    
    #[msg("Only the creator can start the group")]
    OnlyCreatorCanStart,
    
    #[msg("Not enough participants have joined to start the group")]
    NotEnoughParticipants,
    
    #[msg("Group has not started yet")]
    GroupNotStarted,
    
    #[msg("Group has completed all rounds")]
    GroupCompleted,
    
    #[msg("You are not a participant in this group")]
    NotAParticipant,
    
    #[msg("Interval has not passed yet")]
    IntervalNotPassed,
    
    #[msg("Insufficient funds in token account")]
    InsufficientFunds,
}