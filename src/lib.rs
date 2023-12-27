/*----------------------------------------------------------------------------------------------------------
 *  Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/piot/conclave-room-serialize-rs
 *  Licensed under the MIT License. See LICENSE in the project root for license information.
 *--------------------------------------------------------------------------------------------------------*/
use std::io::{Cursor, Read};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use conclave_room;
use conclave_room::{Knowledge, Term};

use crate::ServerReceiveCommand::PingCommandType;

#[derive(Debug, PartialEq)]
pub struct PingCommand {
    pub term: Term,
    pub knowledge: Knowledge,
}

impl PingCommand {
    pub fn to_octets(&self) -> Vec<u8> {
        let mut writer = vec![];

        writer.write_u16::<BigEndian>(self.term).unwrap();
        writer.write_u64::<BigEndian>(self.knowledge).unwrap();

        writer
    }

    pub fn from_cursor(reader: &mut Cursor<&[u8]>) -> Self {
        Self {
            term: reader.read_u16::<BigEndian>().unwrap(),
            knowledge: reader.read_u64::<BigEndian>().unwrap(),
        }
    }
}

#[derive(Debug)]
enum ServerReceiveCommand {
    PingCommandType(PingCommand),
}

impl ServerReceiveCommand {
    pub fn to_octets(&self) -> Result<Vec<u8>, String> {
        let command_type_id = match self {
            PingCommandType(ping_command) => PING_COMMAND_TYPE_ID,
            _ => return Err(format!("unsupported command {:?}", self)),
        };

        let mut writer = vec![];

        writer.write_u8(command_type_id).expect("could not write command type id");

        match self {
            PingCommandType(ping_command) => writer.extend_from_slice(ping_command.to_octets().as_slice()),
            _ => return Err(format!("unknown command enum {:?}", self)),
        }

        Ok(writer)
    }

    pub fn from_octets(input: &[u8]) -> Result<ServerReceiveCommand, String> {
        let mut rdr = Cursor::new(input);
        let command_type_id = rdr.read_u8().unwrap();
        match command_type_id {
            PING_COMMAND_TYPE_ID => Ok(PingCommandType(PingCommand::from_cursor(&mut rdr))),
            _ => Err(format!("unknown command 0x{:x}", command_type_id)),
        }
    }
}

const PING_COMMAND_TYPE_ID: u8 = 0x01;


#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{PING_COMMAND_TYPE_ID, PingCommand, ServerReceiveCommand};
    use crate::ServerReceiveCommand::PingCommandType;

    #[test]
    fn check_serializer() {
        let ping_command = PingCommand {
            term: 32,
            knowledge: 444441,
        };

        let encoded = ping_command.to_octets();
        let mut receive_cursor = Cursor::new(encoded.as_slice());
        let deserialized_ping_command = PingCommand::from_cursor(&mut receive_cursor);

        println!("before {:?}", &ping_command);
        println!("after {:?}", &deserialized_ping_command);
        assert_eq!(ping_command, deserialized_ping_command);
    }

    #[test]
    fn check_receive_message() {
        const EXPECTED_KNOWLEDGE_VALUE: u64 = 17718865395771014920;

        let octets = [PING_COMMAND_TYPE_ID,
            0x00, 0x20, // Term
            0xF5, 0xE6, 0x0E, 0x32, 0xE9, 0xE4, 0x7F, 0x08 // Knowledge
        ];

        let message = &ServerReceiveCommand::from_octets(&octets).unwrap();

        match message {
            PingCommandType(ping_command) => {
                println!("received {:?}", &ping_command);
                assert_eq!(0x20, ping_command.term);
                assert_eq!(EXPECTED_KNOWLEDGE_VALUE, ping_command.knowledge);
                let octets_after = message.to_octets().unwrap();
                assert_eq!(octets, octets_after.as_slice());
            }
            _ => assert!(false, "should be ping command")
        }
    }
}
