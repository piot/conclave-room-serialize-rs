/*----------------------------------------------------------------------------------------------------------
 *  Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/piot/conclave-room-serialize-rs
 *  Licensed under the MIT License. See LICENSE in the project root for license information.
 *--------------------------------------------------------------------------------------------------------*/
//! The Conclave Room Protocol Serialization

use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use conclave_room::{Knowledge, Term};

use crate::ClientReceiveCommand::RoomInfoType;
use crate::ServerReceiveCommand::PingCommandType;

/// Sent from Client to Server
#[derive(Debug, PartialEq)]
pub struct PingCommand {
    pub term: Term,
    pub knowledge: Knowledge,
    pub has_connection_to_leader: bool,
}

impl PingCommand {
    pub fn to_octets(&self) -> Vec<u8> {
        let mut writer = vec![];

        writer.write_u16::<BigEndian>(self.term).unwrap();
        writer.write_u64::<BigEndian>(self.knowledge).unwrap();
        writer
            .write_u8(if self.has_connection_to_leader {
                0x01
            } else {
                0x00
            })
            .unwrap();

        writer
    }

    pub fn from_cursor(reader: &mut Cursor<&[u8]>) -> Self {
        Self {
            term: reader.read_u16::<BigEndian>().unwrap(),
            knowledge: reader.read_u64::<BigEndian>().unwrap(),
            has_connection_to_leader: reader.read_u8().unwrap() != 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ClientInfo {
    pub custom_user_id: u64,
    pub connection_index: u8,
}

/// Sent from Server to Client
#[derive(Debug, PartialEq)]
pub struct RoomInfoCommand {
    pub term: Term,
    pub leader_index: u8,
    pub client_infos: Vec<ClientInfo>,
}

impl RoomInfoCommand {
    pub fn to_octets(&self) -> Vec<u8> {
        let mut writer = vec![];

        writer.write_u16::<BigEndian>(self.term).unwrap();
        writer.write_u8(self.client_infos.len() as u8).unwrap();
        for client_info in self.client_infos.iter() {
            writer.write_u8(client_info.connection_index).unwrap();
            writer
                .write_u64::<BigEndian>(client_info.custom_user_id)
                .unwrap();
        }
        writer.write_u8(self.leader_index).unwrap();

        writer
    }

    pub fn from_cursor(reader: &mut Cursor<&[u8]>) -> Self {
        let term = reader.read_u16::<BigEndian>().unwrap();
        let length = reader.read_u8().unwrap() as usize;
        let slice = &mut vec![ClientInfo {
            custom_user_id: 0,
            connection_index: 0,
        }][..length];
        for client_info in slice.iter_mut().take(length) {
            *client_info = ClientInfo {
                connection_index: reader.read_u8().unwrap(),
                custom_user_id: reader.read_u64::<BigEndian>().unwrap(),
            }
        }
        Self {
            term,
            leader_index: reader.read_u8().unwrap(),
            client_infos: slice.to_vec(),
        }
    }
}

#[derive(Debug)]
pub enum ServerReceiveCommand {
    PingCommandType(PingCommand),
}

impl ServerReceiveCommand {
    pub fn to_octets(&self) -> Result<Vec<u8>, String> {
        let command_type_id = match self {
            PingCommandType(_) => PING_COMMAND_TYPE_ID,
            // _ => return Err(format!("unsupported command {:?}", self)),
        };

        let mut writer = vec![];

        writer
            .write_u8(command_type_id)
            .expect("could not write command type id");

        match self {
            PingCommandType(ping_command) => {
                writer.extend_from_slice(ping_command.to_octets().as_slice())
            }
            // _ => return Err(format!("unknown command enum {:?}", self)),
        }

        Ok(writer)
    }

    pub fn from_octets(input: &[u8]) -> Result<ServerReceiveCommand, String> {
        let reader = Cursor::new(input);
        ServerReceiveCommand::from_cursor(reader)
    }

    pub fn from_cursor(mut reader: Cursor<&[u8]>) -> Result<ServerReceiveCommand, String> {
        let command_type_id = reader.read_u8().unwrap();
        match command_type_id {
            PING_COMMAND_TYPE_ID => Ok(PingCommandType(PingCommand::from_cursor(&mut reader))),
            _ => Err(format!("unknown command 0x{:x}", command_type_id)),
        }
    }
}

pub const PING_COMMAND_TYPE_ID: u8 = 0x01;
pub const ROOM_INFO_COMMAND_TYPE_ID: u8 = 0x02;

#[derive(Debug)]
pub enum ClientReceiveCommand {
    RoomInfoType(RoomInfoCommand),
}

impl ClientReceiveCommand {
    pub fn to_octets(&self) -> Result<Vec<u8>, String> {
        let command_type_id = match self {
            RoomInfoType(_) => ROOM_INFO_COMMAND_TYPE_ID,
            // _ => return Err(format!("unsupported command {:?}", self)),
        };

        let mut writer = vec![];

        writer
            .write_u8(command_type_id)
            .expect("could not write command type id");

        match self {
            RoomInfoType(room_info_command) => {
                writer.extend_from_slice(room_info_command.to_octets().as_slice())
            }
            // _ => return Err(format!("unknown command enum {:?}", self)),
        }

        Ok(writer)
    }

    pub fn from_octets(input: &[u8]) -> Result<ClientReceiveCommand, String> {
        let mut rdr = Cursor::new(input);
        let command_type_id = rdr.read_u8().unwrap();
        match command_type_id {
            ROOM_INFO_COMMAND_TYPE_ID => Ok(RoomInfoType(RoomInfoCommand::from_cursor(&mut rdr))),
            _ => Err(format!("unknown command 0x{:x}", command_type_id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::ServerReceiveCommand::PingCommandType;
    use crate::ClientReceiveCommand::RoomInfoType;
    use crate::{PingCommand, ServerReceiveCommand, PING_COMMAND_TYPE_ID, ClientReceiveCommand,  ROOM_INFO_COMMAND_TYPE_ID};

    #[test]
    fn check_serializer() {
        let ping_command = PingCommand {
            term: 32,
            knowledge: 444441,
            has_connection_to_leader: false,
        };

        let encoded = ping_command.to_octets();
        let mut receive_cursor = Cursor::new(encoded.as_slice());
        let deserialized_ping_command = PingCommand::from_cursor(&mut receive_cursor);

        println!("before {:?}", &ping_command);
        println!("after {:?}", &deserialized_ping_command);
        assert_eq!(ping_command, deserialized_ping_command);
    }

    #[test]
    fn check_server_receive_message() {
        const EXPECTED_KNOWLEDGE_VALUE: u64 = 17718865395771014920;

        let octets = [
            PING_COMMAND_TYPE_ID,
            0x00,
            0x20, // Term
            0xF5,
            0xE6,
            0x0E,
            0x32,
            0xE9,
            0xE4,
            0x7F,
            0x08, // Knowledge
            0x01, // Has Connection
        ];

        let message = &ServerReceiveCommand::from_octets(&octets).unwrap();

        match message {
            PingCommandType(ping_command) => {
                println!("received {:?}", &ping_command);
                assert_eq!(ping_command.term, 0x20);
                assert_eq!(ping_command.knowledge, EXPECTED_KNOWLEDGE_VALUE);
                assert_eq!(ping_command.has_connection_to_leader, true);
                let octets_after = message.to_octets().unwrap();
                assert_eq!(octets, octets_after.as_slice());
            }
            // _ => assert!(false, "should be ping command"),
        }
    }


    #[test]
    fn check_client_receive_message() {
        const EXPECTED_LEADER_INDEX: u8 = 1;

        let octets = [
            ROOM_INFO_COMMAND_TYPE_ID,
            0x00, // Term
            0x4A, // Term (lower)
            0x00, // Number of client infos that follows
            EXPECTED_LEADER_INDEX, // Leader index
        ];


        let message = &ClientReceiveCommand::from_octets(&octets).unwrap();

        match message {
            RoomInfoType(room_info) => {
                println!("received {:?}", &room_info);
                assert_eq!(room_info.term, 0x4A);
                assert_eq!(room_info.leader_index, EXPECTED_LEADER_INDEX);
                let octets_after = message.to_octets().unwrap();
                assert_eq!(octets, octets_after.as_slice());
            }
            // _ => assert!(false, "should be room info command"),
        }
    }
}
