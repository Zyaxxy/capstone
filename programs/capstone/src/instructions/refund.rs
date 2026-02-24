use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use super::error::AuctionError;
use crate::{Auction, Bids};

#[derive(Accounts)]
pub struct ClaimRefund<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>, // The losing bidder signs and pays the network fee

    #[account(mut)]
    pub auction: Account<'info, Auction>,

    #[account(
        mut,
        close = bidder, // Destroys the PDA and sends the ~0.0015 SOL rent back to the bidder
        seeds = [b"bids", auction.key().as_ref(), bidder.key().as_ref()],
        bump = bid_record.bump,
        has_one = bidder,
    )]
    pub bid_record: Account<'info, Bids>,

    #[account(
        init_if_needed,
        payer = bidder, // If they somehow closed their ATA, they pay to re-open it
        associated_token::mint = bid_mint,
        associated_token::authority = bidder,
    )]
    pub bidder_bid_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = bid_mint,
        associated_token::authority = auction,
    )]
    pub vault_bid: InterfaceAccount<'info, TokenAccount>,

    #[account(address = auction.bid_mint)]
    pub bid_mint: InterfaceAccount<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimRefund<'info> {
    pub fn refund_loser(&mut self) -> Result<()> {
        let clock = Clock::get()?;

        // 1. Ensuring the auction is over
        require!(
            clock.unix_timestamp >= self.auction.end_time,
            AuctionError::AuctionNotEnded
        );

        // 2. Ensuring the winner cannot withdraw their locked bid
        require!(
            self.bid_record.bidder != self.auction.highest_bidder,
            AuctionError::CannotRefundWinner
        );

        // 3. Preparing the PDA signatures to authorize the vault transfer
        let seed_bytes = self.auction.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"auction",
            self.auction.maker.as_ref(),
            seed_bytes.as_ref(),
            &[self.auction.bump],
        ]];

        // 4. Transfering the losing amount back to the bidder
        let transfer_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.vault_bid.to_account_info(),
                to: self.bidder_bid_ata.to_account_info(),
                mint: self.bid_mint.to_account_info(),
                authority: self.auction.to_account_info(),
            },
            signer_seeds,
        );

        transfer_checked(transfer_ctx, self.bid_record.amount, self.bid_mint.decimals)
    }
}
