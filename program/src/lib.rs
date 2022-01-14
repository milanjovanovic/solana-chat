use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

mod processor;

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
pub fn process_instruction(
    program_id: &Pubkey, // Public key of the account the hello world program was loaded into
    accounts: &[AccountInfo], // The account to say hello to
    instruction_data: &[u8], // Ignored, all helloworld instructions are hellos
) -> ProgramResult {
    msg!("Chat program working entrypoint! ");

    let acount_iterator = &mut accounts.iter();
    let from_acc = next_account_info(acount_iterator)?;

    if !from_acc.is_signer {
        return Err(ProgramError::IllegalOwner);
    }

    processor::process_instruction(program_id, accounts, instruction_data)
}
