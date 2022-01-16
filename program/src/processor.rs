use md::data::{
    serialize_messages, AccountMetadata, ChatData, ChatDeserializationError, ChatInstruction,
    Message,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

fn receive_messages(
    account_data: &mut [u8],
    account_metadata: &mut AccountMetadata,
    messages: &mut Vec<Message>,
) -> Result<(), ChatDeserializationError> {
    if messages.is_empty() {
        return Ok(());
    }

    let mut last_message_id = account_metadata.last_message_id;
    for msg in messages.iter_mut() {
        msg.id = last_message_id;
        last_message_id += 1;
    }

    let messages_size: usize = messages.iter().map(|c| c.size()).sum();
    let start_index = account_metadata.next_free_index as usize;

    serialize_messages(
        messages,
        &mut account_data[start_index..start_index + messages_size],
    )?;

    account_metadata.next_free_index = (start_index + messages_size) as u32;
    account_metadata.last_message_id = messages.last().unwrap().id;
    account_metadata.serialize(&mut account_data[0..account_metadata.size()])
}

fn delete_messages(_id: u32) {}

fn open_account(
    account_data: &mut [u8],
    account_metadata: &AccountMetadata,
) -> Result<(), ChatDeserializationError> {
    account_metadata.serialize(&mut account_data[0..account_metadata.size()])
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let acount_iterator = &mut accounts.iter();
    let _from_user = next_account_info(acount_iterator)?;
    let to_acc = next_account_info(acount_iterator)?;

    let to_acc_data = &mut *to_acc.try_borrow_mut_data()?;
    let mut acc_metadata = AccountMetadata::default();
    if acc_metadata.deserialize(to_acc_data).is_err() {
        return ProgramResult::Err(ProgramError::InvalidInstructionData);
    }

    let chat_instruction = &mut ChatInstruction::deserialize(instruction_data)
        .map_err(|_e| -> ProgramError { ProgramError::InvalidInstructionData })?;

    match chat_instruction {
        ChatInstruction::SendMessages { messages } => {
            msg!("SendMessages");
            if receive_messages(to_acc_data, &mut acc_metadata, messages).is_err() {
                return ProgramResult::Err(ProgramError::InvalidInstructionData);
            }
            ProgramResult::Ok(())
        }
        ChatInstruction::DeleteMessages { id } => {
            msg!("DeleteMessages");
            delete_messages(*id);
            ProgramResult::Ok(())
        }
        ChatInstruction::OpenAccount { account_metadata } => {
            msg!("OpenAccount");
            if acc_metadata.initialized > 0 {
                msg!("Account: {} already exist", account_metadata.account_name);
                return ProgramResult::Err(ProgramError::InvalidInstructionData);
            }
            msg!("Opening account: {}", account_metadata.account_name);
            if let Err(_e) = open_account(to_acc_data, account_metadata) {
                return ProgramResult::Err(ProgramError::InvalidInstructionData);
            }
            Ok(())
        }
    }
}
