use clap::Parser;
use core::fmt;
use core::str::FromStr;
use solana_client::rpc_client::{self, RpcClient};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::read_keypair_file;
use solana_sdk::signer::keypair::Keypair;
use std::error::Error;
use std::path::Path;

mod chat;

use chat::{open_account, receive_messages, send_message};

#[derive(Debug, Clone)]
struct CustomError<'a>(&'a str);

impl<'a> fmt::Display for CustomError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl<'a> Error for CustomError<'a> {}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    program_keypair: String,

    #[clap(short, long)]
    command: String,

    #[clap(short, long)]
    keypair: String,

    #[clap(short, long)]
    message: Option<String>,

    #[clap(short, long)]
    to_user: Option<String>,

    #[clap(short, long)]
    account_name: Option<String>,
}

fn load_key_pair(user_key_pair_file: &str) -> Result<Keypair, Box<dyn Error>> {
    let user_key_pair = read_keypair_file(Path::new(user_key_pair_file))?;
    Ok(user_key_pair)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let program_keypair: String = args.program_keypair;
    let command: String = args.command;
    let key_pair: String = args.keypair;
    let message: Option<String> = args.message;
    let to_user: Option<String> = args.to_user;
    let account_name: Option<String> = args.account_name;

    let user_kp = load_key_pair(&key_pair)?;
    let program_kp = load_key_pair(&program_keypair)?;
    let rpc_client: RpcClient = RpcClient::new("http://localhost:8899".to_string());

    match command.as_str() {
        "send" => {
            if let (Some(to), Some(msg)) = (to_user, message) {
                let to_pk = Pubkey::from_str(&to).unwrap();
                send_message(&rpc_client, &program_kp, &user_kp, &to_pk, msg)
            } else {
                panic!("Missing to_user or message !");
            }
        }
        "open_account" => {
            if let Some(name) = account_name {
                open_account(&rpc_client, &program_kp, &user_kp, &name)
            } else {
                panic!("Missing account_name");
            }
        }
        "receive" => receive_messages(&rpc_client, &program_kp, &user_kp, None),
        "delete" => {
            panic!("Not implemented");
        }
        _ => panic!("Unknown option !"),
    }
}

// fn main_old() {
//     let args: Vec<String> = std::env::args().collect();

//     let user_key_pair_file = &args[1];

//     println!("Using key pair file: {}", user_key_pair_file);

//     let user_key_pair = read_keypair_file(Path::new(user_key_pair_file)).unwrap();

//     let payer_key_pair: Keypair =
//         read_keypair_file(Path::new("/Users/milan/.config/solana/id.json")).unwrap();

//     let rpc_client: RpcClient = RpcClient::new("http://localhost:8899".to_string());

//     let rent = rpc_client
//         .get_minimum_balance_for_rent_exemption(ACCOUNT_SIZE as usize)
//         .unwrap();

//     let program_key_pair: Keypair = read_keypair_file(Path::new(
//         "/Users/milan/solana/project/testing/src/program-rust/target/deploy/testing-keypair.json",
//     ))
//     .unwrap();

//     let program_pub_key = Pubkey::from_str(PROGRAM_ADDRESS).unwrap();

//     // // open account
//     // if args.len() == 3 {
//     //     match open_account(
//     //         &rpc_client,
//     //         &user_key_pair.pubkey(),
//     //         rent,
//     //         &program_pub_key,
//     //         &payer_key_pair,
//     //         &user_key_pair,
//     //         &program_key_pair,
//     //         10,
//     //     ) {
//     //         Ok(pk) => {
//     //             println!("Created new account: {}", pk.to_string());
//     //         }

//     //         Err(err) => {
//     //             println!("Got error while creating new account: {}", err);
//     //             return;
//     //         }
//     //     }
//     // }

//     let msg_id: u32 = 1;
//     let msg_pk = Pubkey::from_str("DidmGHY2FMXTPzxMhiMjNSzwuqcHhJ679yP4NdCQsoqM").unwrap();
//     let msg_msg = String::from("this is msg");

//     let msg_id_bytes = &u32::to_le_bytes(msg_id)[..];
//     let msg_pk_bytes = &msg_pk.to_bytes()[..];
//     let msg_msg_bytes = String::as_bytes(&msg_msg);

//     // let d: Vec<u8> = msg_pk_bytes
//     //     .into_iter()
//     //     .chain(msg_msg_bytes.into_iter())
//     //     .collect();

//     let instruction_data: Vec<u8> = msg_id_bytes
//         .iter()
//         .copied()
//         .chain(
//             msg_pk_bytes
//                 .iter()
//                 .copied()
//                 .chain(msg_msg_bytes.iter().copied()),
//         )
//         .collect();

//     let ac_meta = AccountMeta::new(user_key_pair.pubkey(), true);
//     let accounts = vec![ac_meta];
//     let instruction = Instruction::new_with_bytes(program_pub_key, &instruction_data[..], accounts);

//     let hsh = rpc_client.get_latest_blockhash().unwrap();
//     let transaction = Transaction::new_signed_with_payer(
//         &[instruction],
//         Some(&payer_key_pair.pubkey()),
//         &[&user_key_pair, &payer_key_pair],
//         hsh,
//     );

//     let transaction_result = rpc_client.send_and_confirm_transaction_with_spinner(&transaction);
//     match transaction_result {
//         Ok(sig) => {
//             println!("Transaction successed !");
//             println!("Signature: {}", sig.to_string());
//         }
//         Err(err) => {
//             println!("Got Error: {:?}", err);
//         }
//     }

//     // let pk = key_pair.pubkey();
//     // let pda = Pubkey::find_program_address(&[b"foo"], &program_key_pair.pubkey());
// }

// fn open_account(
//     rpc_client: &RpcClient,
//     user: &Pubkey,
//     rent: u64,
//     owner: &Pubkey,
//     payer: &Keypair,
//     user_key_pair: &Keypair,
//     program_key_pair: &Keypair,
//     size: u8,
// ) -> Result<Pubkey, Box<dyn Error>> {
//     let account_pub_key = Pubkey::create_with_seed(user, "betting", owner)?;

//     if let Ok(_existing_account) = rpc_client.get_account(&account_pub_key) {
//         println!("Account already exist !");
//         Ok(account_pub_key)
//     } else {
//         println!("Creating new account ..");
//         let allocation_size = Game::new().try_to_vec().unwrap().len() as u64 * size as u64;

//         let instruction = system_instruction::create_account_with_seed(
//             &payer.pubkey(),
//             &account_pub_key,
//             user,
//             "betting",
//             rent,
//             allocation_size,
//             owner,
//         );

//         let hash = rpc_client.get_latest_blockhash()?;

//         let transaction = Transaction::new_signed_with_payer(
//             &[instruction],
//             Some(&payer.pubkey()),
//             &[payer, user_key_pair],
//             hash,
//         );

//         match rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
//             Ok(sig) => {
//                 println!("Transaction successed !");
//                 println!("Signature: {}", sig.to_string());
//             }
//             Err(err) => {
//                 println!("Got Error: {:?}", err);
//             }
//         }

//         Ok(account_pub_key)
//     }
// }
