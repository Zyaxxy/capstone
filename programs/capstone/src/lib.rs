use anchor_lang::prelude::*;

declare_id!("C27TZ2WfzrWun9AgXky6Roqxpnasxrc69KwpHXAa3pm");

#[program]
pub mod capstone {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
