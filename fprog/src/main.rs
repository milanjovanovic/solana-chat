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
