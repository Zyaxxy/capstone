use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{Auction, Bids};

#[derive(Accounts)]
pub struct Bid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    #[account(mut)]
    pub auction: Account<'info, Auction>,

    #[account(
        init_if_needed,
        payer = bidder,
        space = Bids::DISCRIMINATOR.len() + Bids::INIT_SPACE,
        seeds = [b"bids", auction.key().as_ref(), bidder.key().as_ref()],
        bump
    )]
    pub bid_record: Account<'info, Bids>,

    #[account(mut)]
    pub bidder_bid_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = bid_mint,
        associated_token::authority = auction,
    )]
    pub vault_bid: InterfaceAccount<'info, TokenAccount>,

    #[account(address = auction.bid_mint)]
    pub bid_mint: InterfaceAccount<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Bid<'info> {
    pub fn bid(&mut self, additional_amount: u64, bumps: &BidBumps) -> Result<()> {
        // 1. Enforce the time limit
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp < self.auction.end_time,
            AuctionError::AuctionEnded
        );

        // 2. Initialize baseline data if this is a brand new bid
        if self.bid_record.amount == 0 {
            self.bid_record.bidder = self.bidder.key();
            self.bid_record.bump = bumps.bid_record;
            self.bid_record.refunded = false;
        }

        // 3. Update the user's total deposited amount
        self.bid_record.amount = self
            .bid_record
            .amount
            .checked_add(additional_amount)
            .unwrap();

        // 4. Update the Auction leaderboard if they are the new highest bidder
        if self.bid_record.amount > self.auction.highest_bid_amount {
            self.auction.highest_bidder = self.bidder.key();
            self.auction.highest_bid_amount = self.bid_record.amount;
        }

        // 5. Transfer tokens from the Bidder to the shared Vault
        let transfer_accounts = TransferChecked {
            from: self.bidder_bid_ata.to_account_info(),
            to: self.vault_bid.to_account_info(),
            mint: self.bid_mint.to_account_info(),
            authority: self.bidder.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), transfer_accounts);
        transfer_checked(cpi_ctx, additional_amount, self.bid_mint.decimals)
    }
}

#[error_code]
pub enum AuctionError {
    #[msg("The auction has already ended.")]
    AuctionEnded,
}
