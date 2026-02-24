use anchor_lang::prelude::*;

#[error_code]
pub enum AuctionError {
    #[msg("The auction has already ended.")]
    AuctionEnded,
    #[msg("The auction has not ended yet.")]
    AuctionNotEnded,
    #[msg("The auction has already been resolved.")]
    AlreadyResolved,
    #[msg("The winner cannot claim a refund.")]
    CannotRefundWinner,
}
