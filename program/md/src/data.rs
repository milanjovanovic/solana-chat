use std::{fmt, mem};

use arrayref::array_ref;
use solana_program::{
    msg,
    pubkey::{Pubkey, PUBKEY_BYTES},
};

pub const MINIMUM_MESSAGE_DATA_SIZE: usize =
    mem::size_of::<u32>() + PUBKEY_BYTES + mem::size_of::<u32>() + 1;

pub const MINIMUM_OPEN_ACCOUNT_DATA_SIZE: usize =
    (mem::size_of::<u32>() * 3) + mem::size_of::<u8>() + 1 + 1;

const U32_SIZE: usize = mem::size_of::<u32>();
const U8_SIZE: usize = mem::size_of::<u8>();

#[derive(Debug)]
#[repr(u8)]
pub enum ChatCommand {
    SendMessages = 0,
    DeleteMessages = 1,
    OpenAccount = 2,
}

#[derive(Debug, Clone)]
pub struct ChatDeserializationError;
impl std::error::Error for ChatDeserializationError {}

impl fmt::Display for ChatDeserializationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "can't deserialize data")
    }
}

pub trait ChatData {
    fn size(&self) -> usize;
    fn serialize(&self, data: &mut [u8]) -> Result<(), ChatDeserializationError>;
    fn deserialize(&mut self, data: &[u8]) -> Result<(), ChatDeserializationError>;
}

#[derive(Debug, PartialEq, Default)]
pub struct Message {
    pub id: u32,
    pub from: Pubkey,
    pub msg_size: u32,
    pub msg: String,
}

impl Message {
    pub fn new(id: u32, from: Pubkey, msg: String) -> Self {
        let mut message = Message {
            id,
            from,
            msg_size: 0,
            msg,
        };
        message.msg_size = message.msg.len() as u32;
        message
    }
}

impl ChatData for Message {
    fn size(&self) -> usize {
        U32_SIZE + PUBKEY_BYTES + self.msg_size as usize + U32_SIZE
    }
    fn deserialize(&mut self, data: &[u8]) -> Result<(), ChatDeserializationError> {
        let id = u32::from_le_bytes(*array_ref!(data, 0, U32_SIZE));
        let from = Pubkey::new_from_array(*array_ref!(data, U32_SIZE, PUBKEY_BYTES));
        let msg_size = u32::from_le_bytes(*array_ref!(data, U32_SIZE + PUBKEY_BYTES, U32_SIZE));
        let msg_start = (U32_SIZE * 2) + PUBKEY_BYTES;
        let msg_end = msg_start + msg_size as usize;
        let msg = String::from_utf8_lossy(&data[msg_start..msg_end]).into_owned();

        self.id = id;
        self.from = from;
        self.msg_size = msg_size;
        self.msg = msg;

        Ok(())
    }

    fn serialize(&self, data: &mut [u8]) -> Result<(), ChatDeserializationError> {
        if self.size() != data.len() {
            return Err(ChatDeserializationError {});
        }

        let mut start: usize = 0;
        let mut end: usize = U32_SIZE;
        data[start..end].copy_from_slice(&u32::to_le_bytes(self.id));

        start = end;
        end += PUBKEY_BYTES;
        data[start..end].copy_from_slice(&Pubkey::to_bytes(self.from)[..]);

        start = end;
        end += U32_SIZE;
        data[start..end].copy_from_slice(&u32::to_le_bytes(self.msg_size));

        start = end;
        end += self.msg_size as usize;
        data[start..end].copy_from_slice(String::as_bytes(&self.msg));

        Ok(())
    }
}

pub fn deserialize_messages(data: &[u8]) -> Result<Vec<Message>, ChatDeserializationError> {
    let mut messages = Vec::new();
    let mut start = 0;
    if data.is_empty() {
        return Ok(messages);
    }
    loop {
        let mut msg = Message::default();
        msg.deserialize(&data[start..])?;
        let size = msg.size();
        messages.push(msg);
        start += size;
        if start >= data.len() {
            break;
        }
    }
    Ok(messages)
}

pub fn serialize_messages(
    messages: &[Message],
    data: &mut [u8],
) -> Result<(), ChatDeserializationError> {
    let mut current_index = 0;

    for message in messages {
        message.serialize(&mut data[current_index..current_index + message.size()])?;
        current_index += message.size();
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum ChatInstruction {
    SendMessages { messages: Vec<Message> },
    DeleteMessages { id: u32 },
    OpenAccount { account_metadata: AccountMetadata },
}

impl ChatInstruction {
    pub fn size(&self) -> usize {
        mem::size_of::<u8>()
            + match self {
                ChatInstruction::SendMessages { messages } => {
                    messages.iter().map(|c| c.size()).sum()
                }
                ChatInstruction::DeleteMessages { id: _ } => mem::size_of::<u32>(),
                ChatInstruction::OpenAccount { account_metadata } => account_metadata.size(),
            }
    }

    pub fn serialize(&self, data: &mut [u8]) -> Result<(), ChatDeserializationError> {
        if self.size() != data.len() {
            return Err(ChatDeserializationError {});
        }

        match self {
            ChatInstruction::SendMessages { messages } => {
                data[0] = 0;
                serialize_messages(messages, &mut data[mem::size_of::<u8>()..])?;
                Ok(())
            }

            ChatInstruction::DeleteMessages { id } => {
                data[0] = 1;
                data[mem::size_of::<u8>()..].copy_from_slice(&u32::to_le_bytes(*id));
                Ok(())
            }
            ChatInstruction::OpenAccount { account_metadata } => {
                data[0] = 2;
                account_metadata.serialize(&mut data[mem::size_of::<u8>()..])?;
                Ok(())
            }
        }
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ChatDeserializationError> {
        let (tag, rest) = data.split_first().ok_or(ChatDeserializationError)?;
        match tag {
            0 => Ok(ChatInstruction::SendMessages {
                messages: deserialize_messages(rest)?,
            }),
            1 => Ok(ChatInstruction::DeleteMessages {
                id: u32::from_le_bytes(*array_ref![rest, 0, mem::size_of::<u32>()]),
            }),
            2 => {
                let mut account_metadata = AccountMetadata::default();
                account_metadata.deserialize(rest)?;
                Ok(ChatInstruction::OpenAccount { account_metadata })
            }
            _ => Err(ChatDeserializationError),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct AccountMetadata {
    pub initialized: u8,
    pub next_free_index: u32,
    pub last_message_id: u32,
    pub account_name_len: u32,
    pub account_name: String,
}

impl AccountMetadata {
    const ACCOUNT_METADATA_BASE_SIZE: usize = (mem::size_of::<u32>() * 3) + mem::size_of::<u8>();
    // FIXME, set next_free_index to account_metadata.size()
    pub fn new(account_name: &str) -> Self {
        let name = account_name.to_string();
        let mut account_metadata = AccountMetadata {
            initialized: 1,
            next_free_index: 0,
            last_message_id: 0,
            account_name_len: name.len() as u32,
            account_name: name,
        };
        account_metadata.next_free_index = account_metadata.size() as u32;
        account_metadata
    }

    pub fn calculate_size_from_buffer(data: &[u8]) -> usize {
        let account_name_len_offset = U8_SIZE + (2 * U32_SIZE);
        let account_name_len =
            u32::from_le_bytes(*array_ref![data, account_name_len_offset, U32_SIZE]);
        AccountMetadata::ACCOUNT_METADATA_BASE_SIZE + account_name_len as usize
    }
}

impl ChatData for AccountMetadata {
    fn size(&self) -> usize {
        AccountMetadata::ACCOUNT_METADATA_BASE_SIZE as usize + self.account_name_len as usize
    }

    fn serialize(&self, data: &mut [u8]) -> Result<(), ChatDeserializationError> {
        if self.size() != data.len() {
            return Err(ChatDeserializationError {});
        }

        let mut start: usize = 0;
        let mut end = start + mem::size_of::<u8>();
        data[start..end].copy_from_slice(&u8::to_le_bytes(self.initialized));

        start = end;
        end += U32_SIZE;
        data[start..end].copy_from_slice(&u32::to_le_bytes(self.next_free_index));

        start = end;
        end += U32_SIZE;
        data[start..end].copy_from_slice(&u32::to_le_bytes(self.last_message_id));

        start = end;
        end += U32_SIZE;
        data[start..end].copy_from_slice(&u32::to_le_bytes(self.account_name_len));

        start = end;
        end += self.account_name_len as usize;
        data[start..end].copy_from_slice(String::as_bytes(&self.account_name));

        Ok(())
    }

    fn deserialize(&mut self, data: &[u8]) -> Result<(), ChatDeserializationError> {
        const U8_SIZE: usize = mem::size_of::<u8>();
        let initialized = u8::from_le_bytes(*array_ref!(data, 0, U8_SIZE));
        let next_free_index = u32::from_le_bytes(*array_ref!(data, U8_SIZE, U32_SIZE));
        let last_message_id = u32::from_le_bytes(*array_ref!(data, U32_SIZE + U8_SIZE, U32_SIZE));
        let account_name_len =
            u32::from_le_bytes(*array_ref!(data, (U32_SIZE * 2) + U8_SIZE, U32_SIZE));

        let account_name = String::from_utf8_lossy(
            &data[(U32_SIZE * 3) + U8_SIZE..(U32_SIZE * 3) + U8_SIZE + account_name_len as usize],
        )
        .into_owned();

        self.initialized = initialized;
        self.next_free_index = next_free_index;
        self.last_message_id = last_message_id;
        self.account_name_len = account_name_len;
        self.account_name = account_name;

        Ok(())
    }
}

pub fn deserialize_account_data(
    data: &[u8],
) -> Result<(AccountMetadata, Option<Vec<Message>>), ChatDeserializationError> {
    let account_metadata_size = AccountMetadata::calculate_size_from_buffer(data);
    let mut account_metadata = AccountMetadata::default();
    account_metadata.deserialize(&data[..account_metadata_size])?;
    let next_free_index = account_metadata.next_free_index as usize;
    if next_free_index > account_metadata_size {
        let messages = deserialize_messages(&data[account_metadata_size..next_free_index])?;
        Ok((account_metadata, Some(messages)))
    } else {
        Ok((account_metadata, None))
    }
}

#[cfg(test)]
mod tests {
    use crate::data::{deserialize_messages, serialize_messages, ChatData};

    use super::{AccountMetadata, ChatDeserializationError, ChatInstruction};

    static PROGRAM_ADDRESS: &str = "DidmGHY2FMXTPzxMhiMjNSzwuqcHhJ679yP4NdCQsoqM";

    #[test]
    fn message_serialization() -> Result<(), ChatDeserializationError> {
        use std::str::FromStr;

        use solana_program::pubkey::Pubkey;

        use crate::data::Message;

        let message = Message::new(
            1,
            Pubkey::from_str(&PROGRAM_ADDRESS.to_string()).unwrap(),
            "12345".to_string(),
        );

        let size = message.size();
        let mut data = vec![0; size];

        message.serialize(&mut data[..])?;
        let mut message_new = Message::default();
        message_new.deserialize(&data[..])?;
        assert_eq!(&message, &message_new);
        Ok(())
    }
    #[test]
    fn messages_serialization() -> Result<(), ChatDeserializationError> {
        use std::str::FromStr;

        use solana_program::pubkey::Pubkey;

        use crate::data::Message;

        let msg1 = Message {
            id: 1,
            from: Pubkey::from_str(&PROGRAM_ADDRESS.to_string()).unwrap(),
            msg_size: 5,
            msg: "12345".to_string(),
        };

        let msg2 = Message {
            id: 2,

            from: Pubkey::from_str(&PROGRAM_ADDRESS.to_string()).unwrap(),
            msg_size: 3,
            msg: "abc".to_string(),
        };

        let size = msg1.size() + msg2.size();
        let mut data = vec![0; size];
        let messages = vec![msg1, msg2];

        serialize_messages(&messages, &mut data[..])?;

        let messages_new = deserialize_messages(&data[..])?;
        assert_eq!(messages, messages_new);
        Ok(())
    }

    #[test]
    fn acount_metadata_serialization() -> Result<(), ChatDeserializationError> {
        let account_metadata = AccountMetadata {
            initialized: 1,
            next_free_index: 2,
            last_message_id: 3,
            account_name_len: 3,
            account_name: "abc".to_string(),
        };

        let size = account_metadata.size();
        let mut data = vec![0; size];

        account_metadata.serialize(&mut data[..])?;

        let mut s_account_metadata = AccountMetadata::default();
        s_account_metadata.deserialize(&data[..])?;

        assert_eq!(account_metadata, s_account_metadata);

        Ok(())
    }

    #[test]
    fn chat_instruction_serializtion_oa() -> Result<(), ChatDeserializationError> {
        let chat_inst = ChatInstruction::OpenAccount {
            account_metadata: AccountMetadata {
                initialized: 0,
                next_free_index: 20,
                last_message_id: 3,
                account_name_len: 3,
                account_name: "abc".to_string(),
            },
        };

        let mut data = vec![0; chat_inst.size()];
        chat_inst.serialize(&mut data[..])?;

        let chat_inst_new = ChatInstruction::deserialize(&data[..])?;

        assert_eq!(chat_inst, chat_inst_new);

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use std::str::FromStr;

        use solana_program::pubkey::Pubkey;

        use crate::data::{deserialize_messages, serialize_messages, ChatData, Message};

        use super::{AccountMetadata, ChatDeserializationError, ChatInstruction};

        static PROGRAM_ADDRESS: &str = "DidmGHY2FMXTPzxMhiMjNSzwuqcHhJ679yP4NdCQsoqM";

        #[test]
        fn message_serialization() -> Result<(), ChatDeserializationError> {
            use std::str::FromStr;

            use solana_program::pubkey::Pubkey;

            use crate::data::Message;

            let message = Message::new(
                1,
                Pubkey::from_str(&PROGRAM_ADDRESS.to_string()).unwrap(),
                "12345".to_string(),
            );

            let size = message.size();
            let mut data = vec![0; size];

            message.serialize(&mut data[..])?;
            let mut message_new = Message::default();
            message_new.deserialize(&data[..])?;
            assert_eq!(&message, &message_new);
            Ok(())
        }
        #[test]
        fn messages_serialization() -> Result<(), ChatDeserializationError> {
            use std::str::FromStr;

            use solana_program::pubkey::Pubkey;

            use crate::data::Message;

            let msg1 = Message {
                id: 1,
                from: Pubkey::from_str(&PROGRAM_ADDRESS.to_string()).unwrap(),
                msg_size: 5,
                msg: "12345".to_string(),
            };

            let msg2 = Message {
                id: 2,

                from: Pubkey::from_str(&PROGRAM_ADDRESS.to_string()).unwrap(),
                msg_size: 3,
                msg: "abc".to_string(),
            };

            let size = msg1.size() + msg2.size();
            let mut data = vec![0; size];
            let messages = vec![msg1, msg2];

            serialize_messages(&messages, &mut data[..])?;

            let messages_new = deserialize_messages(&data[..])?;
            assert_eq!(messages, messages_new);
            Ok(())
        }

        #[test]
        fn acount_metadata_serialization() -> Result<(), ChatDeserializationError> {
            let account_metadata = AccountMetadata {
                initialized: 1,
                next_free_index: 2,
                last_message_id: 3,
                account_name_len: 3,
                account_name: "abc".to_string(),
            };

            let size = account_metadata.size();
            let mut data = vec![0; size];

            account_metadata.serialize(&mut data[..])?;

            let mut s_account_metadata = AccountMetadata::default();
            s_account_metadata.deserialize(&data[..])?;

            assert_eq!(account_metadata, s_account_metadata);

            Ok(())
        }

        #[test]
        fn chat_instruction_serializtion_sm() -> Result<(), ChatDeserializationError> {
            let chat_inst = ChatInstruction::SendMessages { messages: vec![] };

            let mut data = vec![0; chat_inst.size()];
            chat_inst.serialize(&mut data[..])?;

            let chat_inst_new = ChatInstruction::deserialize(&data[..])?;

            assert_eq!(chat_inst, chat_inst_new);

            Ok(())
        }

        #[test]
        fn chat_instruction_serializtion_sm2() -> Result<(), ChatDeserializationError> {
            let message1 = Message::new(
                0,
                Pubkey::from_str(&PROGRAM_ADDRESS.to_string()).unwrap(),
                "message message".to_string(),
            );
            let message2 = Message::new(
                1,
                Pubkey::from_str(&PROGRAM_ADDRESS.to_string()).unwrap(),
                "message message 2".to_string(),
            );

            let chat_inst = ChatInstruction::SendMessages {
                messages: vec![message1, message2],
            };

            let mut data = vec![0; chat_inst.size()];
            chat_inst.serialize(&mut data[..])?;

            let chat_inst_new = ChatInstruction::deserialize(&data[..])?;

            assert_eq!(chat_inst, chat_inst_new);

            Ok(())
        }

        #[test]
        fn chat_instruction_serializtion_dm() -> Result<(), ChatDeserializationError> {
            let chat_inst = ChatInstruction::DeleteMessages { id: 100 };

            let mut data = vec![0; chat_inst.size()];
            chat_inst.serialize(&mut data[..])?;

            let chat_inst_new = ChatInstruction::deserialize(&data[..])?;

            println!("{:?}", chat_inst_new);

            assert_eq!(chat_inst, chat_inst_new);

            Ok(())
        }
    }
}
