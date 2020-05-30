use std::net::TcpListener;
use std::thread::spawn;
use std::env::args;
use log::info;

mod handler;

fn main() {
    simple_logger::init().unwrap();
    info!("Starting recording microservice");
    let port = args().nth(1).expect("No port given");
    let port = port.parse::<u16>().expect("Didn't pass a valid number");
    assert!(port < 65535);

    let server = TcpListener::bind(format!("record-service:{}", port)).expect("Failed to bind");

    info!("Listening on port {}", port);
    for stream in server.incoming()
    {
        match stream {
            Ok(s) => {
                let mut h = handler::WAVReceiver::new();
                info!("Serving: {}", s.peer_addr().expect("Peer doesn't have address"));
                spawn(move || h.handle(s));
            },
            Err(_) => break
        }
    }
}
