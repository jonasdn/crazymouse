mod crtp;
mod mouse;

use crazyflie_link::LinkContext;
use hex::FromHex;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut address = [0xe7, 0xe7, 0xe7, 0xe7, 0xe7];

    if args.len() > 1 {
        address = match <[u8; 5]>::from_hex(format!("{:0>10}", args[1])) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to parse address {}", e);
                std::process::exit(1);
            }
        };
    }

    println!("* Scanning for Crazyflie quad at address {:X?}", address);
    let context = LinkContext::new();
    if let Ok(found) = context.scan(address) {
        let uri = match found.first() {
            Some(uri) => uri,
            None => {
                eprintln!("error: no Crazyflie found");
                std::process::exit(1);
            }
        };

        println!("* Opening link for Crazyflie at {}", uri);
        let con = match context.open_link(uri) {
            Ok(con) => con,
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        };

        println!("* Initiating user input interface");
        let mut device = match mouse::init() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("mouse: {}", e);
                std::process::exit(1);
            }
        };

        //
        // Initiate logging rotation data over the Crazy Real Time Protocol
        //
        if let Err(e) = crtp::setup_logging(&con) {
            eprintln!("logging: {}", e);
            std::process::exit(1);
        }
        println!(
            "* Receiving data from Crazyflie, you can now use it as a mouse"
        );

        loop {
            let (roll, pitch) = match crtp::get_rotation_data(&con) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("logdata: {}", e);
                    std::process::exit(1);
                }
            };

            if let Err(e) = mouse::update(&mut device, roll, pitch) {
                eprintln!("uinput: {}", e);
                std::process::exit(1);
            }
        }
    }
}
