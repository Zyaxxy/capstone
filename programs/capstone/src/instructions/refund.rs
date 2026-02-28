use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use super::error::AuctionError;
use crate::{Auction, Bids};

#[derive(Accounts)]
pub struct ClaimRefund<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>, // The losing bidder signs and pays the network fee

    /// CHECK: We only need this to send vault_bid ATA rent back to the maker
    #[account(mut, address = auction.maker)]
    pub maker: AccountInfo<'info>,

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
        payer = bidder, // If they somehow closed their ATA
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

        // Ensuring the auction is over
        require!(
            clock.unix_timestamp >= self.auction.end_time,
            AuctionError::AuctionNotEnded
        );

        // Ensuring the winner cannot withdraw their locked bid
        require!(
            self.bid_record.bidder != self.auction.highest_bidder,
            AuctionError::CannotRefundWinner
        );

        // Ensuring the bid has not been refunded already
        require!(!self.bid_record.refunded, AuctionError::AlreadyRefunded);

        // Preparing the PDA signatures to authorize the vault transfer
        let seed_bytes = self.auction.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"auction",
            self.auction.maker.as_ref(),
            seed_bytes.as_ref(),
            &[self.auction.bump],
        ]];

        // Transfering the losing amount back to the bidder
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
        transfer_checked(transfer_ctx, self.bid_record.amount, self.bid_mint.decimals)?;

        // Last one out turns off the lights — if all tokens have been withdrawn,
        // we close the vault ATA and the Auction PDA so the maker gets their rent back.
        self.vault_bid.reload()?;
        if self.vault_bid.amount == 0 {
            // Close the now-empty token vault
            close_account(CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                CloseAccount {
                    account: self.vault_bid.to_account_info(),
                    destination: self.maker.to_account_info(),
                    authority: self.auction.to_account_info(),
                },
                signer_seeds,
            ))?;

            // Manually close the Auction PDA — Anchor can't do this for us
            // because the last refund caller isn't the maker, so we zero the
            // account's lamports and data ourselves.
            let auction_info = self.auction.to_account_info();
            let maker_info = self.maker.to_account_info();

            let rent = auction_info.lamports();
            **auction_info.lamports.borrow_mut() = 0;
            **maker_info.lamports.borrow_mut() = maker_info.lamports().checked_add(rent).unwrap();
            auction_info.data.borrow_mut().fill(0);
        }

        Ok(())
    }
}
