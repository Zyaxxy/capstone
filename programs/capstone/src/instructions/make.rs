use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::Auction;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct MakeAuction<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub nft_mint: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub bid_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_nft_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = maker,
        seeds = [b"auction", maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        space = Auction::DISCRIMINATOR.len() + Auction::INIT_SPACE,
        bump,
    )]
    pub auction: Account<'info, Auction>,

    #[account(
        init,
        payer = maker,
        associated_token::mint = nft_mint,
        associated_token::authority = auction,
        associated_token::token_program = token_program,
    )]
    pub vault_nft: InterfaceAccount<'info, TokenAccount>,

    // Vault B: The shared pool that will collect the USDC/Bid tokens from everyone
    #[account(
        init,
        payer = maker,
        associated_token::mint = bid_mint,
        associated_token::authority = auction,
        associated_token::token_program = token_program,
    )]
    pub vault_bid: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> MakeAuction<'info> {
    pub fn init_auction(
        &mut self,
        seed: u64,
        end_time: i64,
        bumps: &MakeAuctionBumps,
    ) -> Result<()> {
        self.auction.set_inner(Auction {
            seed,
            maker: self.maker.key(),
            nft_mint: self.nft_mint.key(),
            bid_mint: self.bid_mint.key(),
            end_time,
            bump: bumps.auction,
            resolved: false,
            highest_bidder: Pubkey::default(),
            highest_bid_amount: 0,
        });

        Ok(())
    }

    pub fn deposit_prize(&mut self, deposit_amount: u64) -> Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.maker_nft_ata.to_account_info(),
            to: self.vault_nft.to_account_info(),
            mint: self.nft_mint.to_account_info(),
            authority: self.maker.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), transfer_accounts);

        transfer_checked(cpi_ctx, deposit_amount, self.nft_mint.decimals)
    }
}
