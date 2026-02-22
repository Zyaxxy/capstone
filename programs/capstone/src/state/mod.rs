use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Auction {
    pub seed: u64,
    pub maker: Pubkey,
    pub nft_mint: Pubkey,
    pub bid_mint: Pubkey,
    pub end_time: i64,
    pub bump: u8,
    pub resolved: bool,
    pub highest_bidder: Pubkey,
    pub highest_bid_amount: u64,
}

#[account]
#[derive(InitSpace)]
pub struct Bids {
    pub bidder: Pubkey,
    pub amount: u64,
    pub bump: u8,
    pub refunded: bool,
}
