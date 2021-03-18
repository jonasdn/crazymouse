use crazyflie_link::{Connection, Packet};

use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashMap;

use anyhow::{bail, Result};

const CRAZYMOUSE_ID: u8 = 0x42;

const CRTP_LOGGING_PORT: u8 = 0x5;

const CRTP_TOC_CHANNEL: u8 = 0x0;
const CRTP_SETTINGS_CHANNEL: u8 = 0x1;
const CRTP_LOGDATA_CHANNEL: u8 = 0x2;

const CRTP_CMD_TOC_INFO: u8 = 0x3;
const CRTP_CMD_TOC_ITEM: u8 = 0x2;
const CRTP_CMD_CREATE_BLOCK: u8 = 0x6;
const CRTP_CMD_START_LOGGING: u8 = 0x3;

const CRTP_LOGGING_PERIOD_MS: u8 = 10;

//
// Receive a packet of certain type, or bail out.
//
fn expect(con: &Connection, channel: u8, cmd: u8) -> Result<Packet> {
    loop {
        let timeout = std::time::Duration::from_secs(10);
        let received = con.recv_packet_timeout(timeout)?;
        let data = received.get_data();
        if received.get_channel() == channel && data[0] == cmd {
            return Ok(received);
        } else {
            continue;
        }
    }
}

//
// The CRTP protocol will encode the variable name as:
//
//   [char, char, char, ..., 0, char, char, char, ..., 0]
//
// Where the first group of chars are the logging group and the second is
// the variable name.
//
fn parse_name(data: &[u8]) -> String {
    let mut found_dot = false;
    let mut name = String::new();

    for byte in data {
        let ch = *byte as char;
        if ch == '\0' {
            if found_dot {
                break;
            }
            found_dot = true;
            name.push('.');
        } else if ch.is_ascii_alphabetic() {
            name.push(ch);
        }
    }
    name
}

//
// Send packet or die.
//
fn send_packet(con: &Connection, packet: Packet) -> Result<()> {
    con.send_packet(packet)?;
    Ok(())
}

//
// Use the CRTP protocol to get the list of all varialbes and their coresponding ids
//
fn fetch_toc(con: &Connection) -> Result<HashMap<String, u16>> {
    let packet = Packet::new(
        CRTP_LOGGING_PORT,
        CRTP_TOC_CHANNEL,
        vec![CRTP_CMD_TOC_INFO],
    );
    send_packet(con, packet)?;
    let packet = expect(con, CRTP_TOC_CHANNEL, CRTP_CMD_TOC_INFO)?;
    let data = packet.get_data();
    let items = (data[2] as u16) << 8 | data[1] as u16;

    let mut toc = HashMap::new();
    for element in 0..items {
        let packet = Packet::new(
            CRTP_LOGGING_PORT,
            CRTP_TOC_CHANNEL,
            vec![
                CRTP_CMD_TOC_ITEM,
                (element & 0xff) as u8,
                ((element >> 8) & 0xff) as u8,
            ],
        );
        send_packet(con, packet)?;
        let packet = expect(con, CRTP_TOC_CHANNEL, CRTP_CMD_TOC_ITEM)?;
        //
        // Pack the u16 ident in two bytes of the packet, little-endian style.
        //
        let data = packet.get_data();
        let ident = (data[2] as u16) << 8 | data[1] as u16;
        let name = parse_name(&data[3..]);
        toc.insert(name, ident);
    }
    Ok(toc)
}

pub fn setup_logging(con: &Connection) -> Result<()> {
    let toc = fetch_toc(con)?;

    let mut packet = Packet::new(
        CRTP_LOGGING_PORT,
        CRTP_SETTINGS_CHANNEL,
        vec![CRTP_CMD_CREATE_BLOCK, CRAZYMOUSE_ID],
    );

    //
    // We tell the Crazyflie that we want the roll and the pitch.
    //
    let variables = vec!["stabilizer.roll", "stabilizer.pitch"];
    for &var in variables.iter() {
        let s = String::from(var);
        let id = match toc.get(&s) {
            Some(id) => id,
            None => {
                bail!("variables not found!");
            }
        };
        let mut v = vec![0x07, (id & 0xff) as u8, ((id >> 8) & 0xff) as u8];
        packet.append_data(&mut v);
    }
    send_packet(con, packet)?;
    expect(con, CRTP_SETTINGS_CHANNEL, CRTP_CMD_CREATE_BLOCK)?;

    let packet = Packet::new(
        CRTP_LOGGING_PORT,
        CRTP_SETTINGS_CHANNEL,
        vec![
            CRTP_CMD_START_LOGGING,
            CRAZYMOUSE_ID,
            CRTP_LOGGING_PERIOD_MS / 10,
        ],
    );
    send_packet(con, packet)?;
    expect(con, CRTP_SETTINGS_CHANNEL, CRTP_CMD_START_LOGGING)?;
    Ok(())
}

pub fn get_rotation_data(con: &Connection) -> Result<(f32, f32)> {
    loop {
        let timeout = std::time::Duration::from_secs(10);
        let received = con.recv_packet_timeout(timeout)?;

        if received.get_channel() != CRTP_LOGDATA_CHANNEL {
            continue;
        }

        let data = received.get_data();
        let block_id = data[0];
        if block_id != CRAZYMOUSE_ID {
            continue;
        }

        if data.len() != 12 {
            bail!("invalid logdata length");
        }

        return Ok((
            LittleEndian::read_f32(&data[4..8]),
            LittleEndian::read_f32(&data[8..12]),
        ));
    }
}
