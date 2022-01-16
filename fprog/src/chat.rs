use md::data::{
    deserialize_account_data, AccountMetadata, ChatCommand, ChatData, ChatInstruction, Message,
};
use solana_client::rpc_client::{self, RpcClient};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::{Pubkey, PubkeyError};
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;
use std::error::Error;

static ACCOUNT_SIZE: u64 = 5 * 1024;

static SEED: &str = "chat";

fn create_chat_instruction(
    program: Pubkey,
    from_account: Pubkey,
    to_account: Pubkey,
    chat_instruction: ChatInstruction,
) -> Result<Instruction, Box<dyn Error>> {
    let data_size = chat_instruction.size();
    let mut instruction_data = vec![0; data_size];
    chat_instruction.serialize(&mut instruction_data[..])?;
    let ac_meta = AccountMeta::new(from_account, true);
    let new_account_meta = AccountMeta::new(to_account, false);
    Ok(Instruction::new_with_bytes(
        program,
        &instruction_data[..],
        vec![ac_meta, new_account_meta],
    ))
}

fn infer_chat_account_pubkey(user_pk: &Pubkey, program_pk: &Pubkey) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_with_seed(user_pk, SEED, program_pk)
}

pub fn open_account(
    rpc_client: &RpcClient,
    program_keypair: &Keypair,
    from_user: &Keypair,
    account_name: &str,
) -> Result<(), Box<dyn Error>> {
    let account_pub_key =
        infer_chat_account_pubkey(&from_user.pubkey(), &program_keypair.pubkey())?;

    let rent = rpc_client.get_minimum_balance_for_rent_exemption(ACCOUNT_SIZE as usize)?;

    let existing_account = rpc_client.get_account(&account_pub_key);

    if existing_account.is_err() {
        println!("Creating new  account {}", &account_pub_key.to_string());
        let allocation_size = ACCOUNT_SIZE;

        let account = rpc_client.get_account(&from_user.pubkey())?;
        let lamports = account.lamports;
        println!("User: {} has {} lamports", from_user.pubkey(), lamports);

        let open_account_inst = system_instruction::create_account_with_seed(
            &from_user.pubkey(),
            &account_pub_key,
            &from_user.pubkey(),
            SEED,
            rent,
            allocation_size,
            &program_keypair.pubkey(),
        );

        let chat_instruction = ChatInstruction::OpenAccount {
            account_metadata: AccountMetadata::new(account_name),
        };

        let initialize_acc_inst = create_chat_instruction(
            program_keypair.pubkey(),
            from_user.pubkey(),
            account_pub_key,
            chat_instruction,
        )?;

        let hash = rpc_client.get_latest_blockhash()?;

        let transaction = Transaction::new_signed_with_payer(
            &[open_account_inst, initialize_acc_inst],
            Some(&from_user.pubkey()),
            &[from_user],
            hash,
        );

        match rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
            Ok(sig) => {
                println!("Transaction successed !");
                println!("Signature: {}", sig);
            }
            Err(err) => {
                println!("Got Error: {:?}", err);
                return Err(Box::new(err));
            }
        }
    } else {
        println!("Account {} already exist", account_pub_key);
    }

    Ok(())
}

pub fn receive_messages(
    rpc_client: &RpcClient,
    program_keypair: &Keypair,
    from_user: &Keypair,
    _last_message_id: Option<u32>,
) -> Result<(), Box<dyn Error>> {
    let user_char_account =
        infer_chat_account_pubkey(&from_user.pubkey(), &program_keypair.pubkey())?;

    let data = rpc_client.get_account_data(&user_char_account)?;

    if let Ok((account_metadata, messages)) = deserialize_account_data(&data[..]) {
        println!("{:?}", account_metadata);
        println!("{:?}", messages);
    } else {
        println!("account is empty");
    }

    println!("size of data: {}", data.len());

    Ok(())
}

pub fn infer_chat_address(
    rpc_client: &RpcClient,
    program_keypair: &Keypair,
    from_user: &Keypair,
) -> Result<(), Box<dyn Error>> {
    let from_user_chat_pk =
        infer_chat_account_pubkey(&from_user.pubkey(), &program_keypair.pubkey())?;
    println!("Address: {}", from_user_chat_pk);
    Ok(())
}

pub fn send_message(
    rpc_client: &RpcClient,
    program_keypair: &Keypair,
    from_user: &Keypair,
    to_user: &Pubkey,
    msg: String,
) -> Result<(), Box<dyn Error>> {
    // FIXME, from_user should be generated with seed
    // this from_user is system account that pays for transaction
    let from_user_chat_pk =
        infer_chat_account_pubkey(&from_user.pubkey(), &program_keypair.pubkey())?;
    let _to_account = rpc_client.get_account(to_user)?;
    let message = Message::new(0, from_user.pubkey(), msg);

    let chat_instruction = ChatInstruction::SendMessages {
        messages: vec![message],
    };

    let instruction = create_chat_instruction(
        program_keypair.pubkey(),
        from_user.pubkey(),
        *to_user,
        chat_instruction,
    )?;

    let hash = rpc_client.get_latest_blockhash()?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&from_user.pubkey()),
        &[from_user],
        hash,
    );

    match rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
        Ok(sig) => {
            println!("Transaction successed !");
            println!("Signature: {}", sig);
        }
        Err(err) => {
            println!("Got Error: {:?}", err);
            return Err(Box::new(err));
        }
    }

    Ok(())
}
