use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use super::error::AuctionError;
use crate::Auction;

#[derive(Accounts)]
pub struct ResolveAuction<'info> {
    #[account(mut)]
    pub resolver: Signer<'info>, // The person paying the transaction fee to crank the contract

    #[account(mut)]
    pub auction: Account<'info, Auction>,

    /// CHECK: We only need this to validate the winner_nft_ata ownership
    #[account(address = auction.highest_bidder)]
    pub winner: AccountInfo<'info>,

    /// CHECK: We only need this to validate the maker_bid_ata ownership
    #[account(address = auction.maker)]
    pub maker: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = resolver, // The crank pays the ~0.002 SOL rent to open the account
        associated_token::mint = bid_mint,
        associated_token::authority = maker,
    )]
    pub maker_bid_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = resolver, // The crank pays the rent for the winner's new ATA
        associated_token::mint = nft_mint,
        associated_token::authority = winner,
    )]
    pub winner_nft_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = auction,
    )]
    pub vault_nft: InterfaceAccount<'info, TokenAccount>,

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

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> ResolveAuction<'info> {
    pub fn resolve(&mut self) -> Result<()> {
        let clock = Clock::get()?;

        // 1. Ensure the auction is actually over
        require!(
            clock.unix_timestamp >= self.auction.end_time,
            AuctionError::AuctionNotEnded
        );

        // 2. Ensure it hasn't already been resolved to prevent double-spending
        require!(!self.auction.resolved, AuctionError::AlreadyResolved);

        // 3. Mark as resolved immediately (Checks-Effects-Interactions pattern)
        self.auction.resolved = true;

        // 4. Prepare the PDA signatures to authorize the vault transfers
        let seed_bytes = self.auction.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"auction",
            self.auction.maker.as_ref(),
            seed_bytes.as_ref(),
            &[self.auction.bump],
        ]];

        // 5. Transfer the Prize (NFT) to the Winner
        let transfer_nft_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.vault_nft.to_account_info(),
                to: self.winner_nft_ata.to_account_info(),
                mint: self.nft_mint.to_account_info(),
                authority: self.auction.to_account_info(),
            },
            signer_seeds,
        );
        transfer_checked(transfer_nft_ctx, 1, self.nft_mint.decimals)?;

        // 6. Transfer the Winning Bid (USDC/Tokens) to the Maker
        let transfer_bid_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.vault_bid.to_account_info(),
                to: self.maker_bid_ata.to_account_info(),
                mint: self.bid_mint.to_account_info(),
                authority: self.auction.to_account_info(),
            },
            signer_seeds,
        );
        transfer_checked(
            transfer_bid_ctx,
            self.auction.highest_bid_amount,
            self.bid_mint.decimals,
        )
    }
}
