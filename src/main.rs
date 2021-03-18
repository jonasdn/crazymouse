mod crtp;
mod mouse;

use crazyflie_link::LinkContext;

fn main() {
    let context = LinkContext::new();
    let address = [0x00, 0xde, 0xad, 0xbe, 0xef];

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
        println!("* Receiving logging data from Crazyflie, you can now use it as a mouse");

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
