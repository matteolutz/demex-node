use std::net::UdpSocket;

use artnet_protocol::{ArtCommand, PortAddress};
use clap::Parser;
use open_dmx::{DMXSerial, DMX_CHANNELS};

/// demex-node
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Serial port
    #[arg(short, long)]
    port: String,

    /// Universe (containg ArtNet subnet and universe)
    #[arg(short, long)]
    universe: u16,

    /// Verbose
    #[arg(short, long)]
    verbose: bool,
}

const ARTNET_PORT: u16 = 6454;

fn universe_to_sub_and_uni(universe: u16) -> (u8, u8) {
    let [uni, sub] = universe.to_be_bytes();
    (uni, sub & 0b01111111)
}

fn main() {
    let args = Args::parse();

    let (sub, uni) = universe_to_sub_and_uni(args.universe);

    let universe_port_addr = PortAddress::try_from(args.universe).unwrap();

    let socket = UdpSocket::bind(("0.0.0.0", ARTNET_PORT)).unwrap();

    let mut serial = DMXSerial::open_sync(args.port.as_str()).unwrap();

    println!(
        "[demex-node] Listening on ::{} on universe {} (sub {}, uni {}), writing to serial port {}...",
        ARTNET_PORT, args.universe, sub, uni, serial.name()
    );

    loop {
        let mut buffer = [0u8; 1024];
        let (length, _addr) = socket.recv_from(&mut buffer).unwrap();
        let command = ArtCommand::from_buffer(&buffer[..length]).unwrap();

        match command {
            ArtCommand::Output(output) => {
                if output.port_address == universe_port_addr
                    && output.data.as_ref().len() == DMX_CHANNELS
                {
                    if args.verbose {
                        println!("Received relevant output command {:?}", output);
                    }

                    for i in 0..DMX_CHANNELS {
                        serial.set_channel(i + 1, output.data.as_ref()[i]).unwrap();
                    }

                    serial.update().unwrap();
                }
            }
            _ => {}
        }
    }
}
