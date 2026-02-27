use anchor_lang::prelude::*;
pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

declare_id!("C27TZ2WfzrWun9AgXky6Roqxpnasxrc69KwpHXAa3pm");

#[program]
pub mod capstone {
    use super::*;

    pub fn make_auction(
        ctx: Context<MakeAuction>,
        seed: u64,
        end_time: i64,
        deposit_amount: u64,
    ) -> Result<()> {
        ctx.accounts.init_auction(seed, end_time, &ctx.bumps)?;
        ctx.accounts.deposit_prize(deposit_amount)
    }

    pub fn bid(ctx: Context<Bid>, additional_amount: u64) -> Result<()> {
        ctx.accounts.bid(additional_amount, &ctx.bumps)
    }

    pub fn claim_refund(ctx: Context<ClaimRefund>) -> Result<()> {
        ctx.accounts.refund_loser()
    }

    pub fn resolve_auction(ctx: Context<ResolveAuction>) -> Result<()> {
        ctx.accounts.resolve()
    }

    pub fn cancel_auction(ctx: Context<CancelAuction>) -> Result<()> {
        ctx.accounts.cancel()
    }
}
