use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use super::error::AuctionError;
use crate::Auction;

/// Lets the maker reclaim their NFT if the auction ended with zero bids.
/// Without this, a no-bid auction would permanently lock the NFT in the vault
/// because resolve_auction requires a winner_bid_record PDA that was never created.
#[derive(Accounts)]
pub struct CancelAuction<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        has_one = maker,
        seeds = [b"auction", maker.key().as_ref(), auction.seed.to_le_bytes().as_ref()],
        bump = auction.bump,
    )]
    pub auction: Account<'info, Auction>,

    /// The maker's ATA to receive the NFT back. We use init_if_needed in case
    /// they closed it after depositing.
    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = nft_mint,
        associated_token::authority = maker,
    )]
    pub maker_nft_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = auction,
    )]
    pub vault_nft: InterfaceAccount<'info, TokenAccount>,

    /// Even though vault_bid should be empty (no bids), we close it here
    /// so the maker gets their ATA rent back.
    #[account(
        mut,
        associated_token::mint = bid_mint,
        associated_token::authority = auction,
    )]
    pub vault_bid: InterfaceAccount<'info, TokenAccount>,

    #[account(address = auction.nft_mint)]
    pub nft_mint: InterfaceAccount<'info, Mint>,

    #[account(address = auction.bid_mint)]
    pub bid_mint: InterfaceAccount<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CancelAuction<'info> {
    pub fn cancel(&mut self) -> Result<()> {
        let clock = Clock::get()?;

        // Can only cancel after the auction period is over
        require!(
            clock.unix_timestamp >= self.auction.end_time,
            AuctionError::AuctionNotEnded
        );

        // Only allow cancellation if nobody bid — if there are bids,
        // the normal resolve + refund flow should be used instead
        require!(
            self.auction.highest_bid_amount == 0,
            AuctionError::AuctionHasBids
        );

        // Build PDA signer seeds for the auction authority
        let seed_bytes = self.auction.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"auction",
            self.auction.maker.as_ref(),
            seed_bytes.as_ref(),
            &[self.auction.bump],
        ]];

        // Send the NFT back to the maker
        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.vault_nft.to_account_info(),
                    to: self.maker_nft_ata.to_account_info(),
                    mint: self.nft_mint.to_account_info(),
                    authority: self.auction.to_account_info(),
                },
                signer_seeds,
            ),
            1,
            self.nft_mint.decimals,
        )?;

        // Close both vault ATAs — their rent goes back to the maker
        close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault_nft.to_account_info(),
                destination: self.maker.to_account_info(),
                authority: self.auction.to_account_info(),
            },
            signer_seeds,
        ))?;

        close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.vault_bid.to_account_info(),
                destination: self.maker.to_account_info(),
                authority: self.auction.to_account_info(),
            },
            signer_seeds,
        ))?;

        // Manually close the Auction PDA — Anchor's `close` macro can't help
        // here because we're inside the impl, not in the accounts struct.
        let auction_info = self.auction.to_account_info();
        let maker_info = self.maker.to_account_info();

        let rent = auction_info.lamports();
        **auction_info.lamports.borrow_mut() = 0;
        **maker_info.lamports.borrow_mut() = maker_info.lamports().checked_add(rent).unwrap();
        auction_info.data.borrow_mut().fill(0);

        Ok(())
    }
}
