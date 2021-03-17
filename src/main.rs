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

        println!("Opening link for Crazyflie at {}", uri);
        let connection = match context.open_link(uri) {
            Ok(connection) => connection,
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        };

        //
        // Initiate logging rotation data over the Crazy Real Time Protocol
        //
        crtp::setup_logging(&connection);

        let mut device = match mouse::init() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("mouse: {}", e);
                std::process::exit(1);
            }
        };

        loop {
            let (roll, pitch) = crtp::get_rotation_data(&connection);

            if let Err(e) = mouse::update(&mut device, roll, pitch) {
                eprintln!("uinput: {}", e);
                std::process::exit(1);
            }
        }
    }
}
