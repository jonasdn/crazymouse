use crazyflie_link::{Connection, LinkContext};

use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashMap;

use uinput::event::controller::Controller::Mouse;
use uinput::event::controller::Mouse::Left;
use uinput::event::relative::Position::{X, Y};
use uinput::event::relative::Relative::Position;
use uinput::event::Event::{Controller, Relative};

mod packet;
use packet::Packet;

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
fn expect_reply(connection: &Connection, channel: u8, cmd: u8) -> Packet {
    loop {
        let received;
        match connection.recv_packet_timeout(std::time::Duration::from_secs(10)) {
            Ok(v) => received = Packet::from_vec(v),
            Err(_) => {
                eprintln!("failed to setup logging");
                std::process::exit(1);
            }
        }

        if received.channel == channel && received.data[0] == cmd {
            return received;
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
    return name;
}

//
// Send packet or die.
//
fn send_packet(connection: &Connection, packet: &Packet) {
    match connection.send_packet(packet.to_vec()) {
        Ok(_) => return,
        Err(_) => {
            eprintln!("failed to send packet over radio");
            std::process::exit(1);
        }
    }
}

//
// Use the CRTP protocol to get the list of all varialbes and their coresponding ids
//
fn fetch_toc(connection: &Connection) -> HashMap<String, u16> {
    let packet = Packet::new(CRTP_LOGGING_PORT, CRTP_TOC_CHANNEL, vec![CRTP_CMD_TOC_INFO]);

    send_packet(connection, &packet);
    let packet = expect_reply(connection, CRTP_TOC_CHANNEL, CRTP_CMD_TOC_INFO);
    let items = (packet.data[2] as u16) << 8 | packet.data[1] as u16;

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
        send_packet(connection, &packet);
        let packet = expect_reply(connection, CRTP_TOC_CHANNEL, CRTP_CMD_TOC_ITEM);
        //
        // Pack the u16 ident in two bytes of the packet, little-endian style.
        //
        let ident = (packet.data[2] as u16) << 8 | packet.data[1] as u16;
        let name = parse_name(&packet.data[3..]);
        toc.insert(name, ident);
    }
    toc
}

fn setup_logging(connection: &Connection) {
    let toc = fetch_toc(connection);

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
                eprintln!("variables not found!");
                std::process::exit(1);
            }
        };
        let mut v = vec![0x07, (id & 0xff) as u8, ((id >> 8) & 0xff) as u8];
        packet.data.append(&mut v);
    }
    send_packet(connection, &packet);
    expect_reply(connection, CRTP_SETTINGS_CHANNEL, CRTP_CMD_CREATE_BLOCK);

    let packet = Packet::new(
        CRTP_LOGGING_PORT,
        CRTP_SETTINGS_CHANNEL,
        vec![
            CRTP_CMD_START_LOGGING,
            CRAZYMOUSE_ID,
            CRTP_LOGGING_PERIOD_MS / 10,
        ],
    );
    send_packet(connection, &packet);
    expect_reply(connection, CRTP_SETTINGS_CHANNEL, CRTP_CMD_START_LOGGING);
}

fn get_roll_pitch_data(connection: &Connection) -> (f32, f32) {
    loop {
        let received = match connection.recv_packet_timeout(std::time::Duration::from_secs(10)) {
            Ok(v) => Packet::from_vec(v),
            Err(_) => {
                eprintln!("timeout waiting for logdata");
                std::process::exit(1);
            }
        };

        if received.channel != CRTP_LOGDATA_CHANNEL {
            continue;
        }

        let block_id = received.data[0];
        if block_id != CRAZYMOUSE_ID {
            continue;
        }

        if received.data.len() != 12 {
            eprintln!("invalid logdata length");
            std::process::exit(1);
        }

        return (
            LittleEndian::read_f32(&received.data[4..8]),
            LittleEndian::read_f32(&received.data[8..12]),
        );
    }
}

fn main() {
    let context = LinkContext::new();
    let address = [0x00, 0xde, 0xad, 0xbe, 0xef];

    if let Ok(found) = context.scan(address) {
        let uri = found.first().unwrap();

        println!("Opening link for Crazyflie at {}", uri);
        let connection = match context.open_link(uri) {
            Ok(connection) => connection,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        setup_logging(&connection);

        let mut device = uinput::default()
            .unwrap()
            .name("test")
            .unwrap()
            .event(Controller(Mouse(Left)))
            .unwrap() // It's necessary to enable any mouse button. Otherwise Relative events would not work.
            .event(Relative(Position(X)))
            .unwrap()
            .event(Relative(Position(Y)))
            .unwrap()
            .create()
            .unwrap();

        loop {
            let (roll, pitch) = get_roll_pitch_data(&connection);

            device.send(X, roll as i32).unwrap();
            device.send(Y, pitch as i32).unwrap();
            device.synchronize().unwrap();
        }
    }
}
