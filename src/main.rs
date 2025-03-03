use std::net::{IpAddr, Ipv4Addr, UdpSocket};

use artnet_protocol::{ArtCommand, PollReply, PortAddress};
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

fn universe_to_net_sub_and_uni(universe: u16) -> (u8, u8, u8) {
    let [net, sub_uni] = universe.to_be_bytes();
    (net & 0b01111111, sub_uni >> 4 & 0xF, sub_uni & 0xF)
}

fn ip_to_v4(ip: IpAddr) -> Ipv4Addr {
    match ip {
        IpAddr::V4(v4) => v4,
        _ => panic!("Expected IPv4 address"),
    }
}

fn main() {
    let args = Args::parse();

    let (net, sub, uni) = universe_to_net_sub_and_uni(args.universe);

    let universe_port_addr = PortAddress::try_from(args.universe).unwrap();

    let socket = UdpSocket::bind(("0.0.0.0", ARTNET_PORT)).unwrap();

    let mut serial = DMXSerial::open_sync(args.port.as_str()).unwrap();

    println!(
        "[demex-node] Listening on ::{} on port address {} (net {}, sub {}, uni {}), writing to serial port {}...",
        ARTNET_PORT, args.universe, net, sub, uni, serial.name()
    );

    let poll_reply = ArtCommand::PollReply(Box::new(PollReply {
        port_address: [net, sub],
        address: ip_to_v4(socket.local_addr().unwrap().ip()),
        swout: [uni, uni, uni, uni],
        ..Default::default()
    }));
    let poll_reply_str = format!("{:?}", poll_reply);
    let poll_reply_buffer = poll_reply.write_to_buffer().unwrap();

    loop {
        let mut buffer = [0u8; 1024];
        let (length, recv_addr) = socket.recv_from(&mut buffer).unwrap();
        let command = ArtCommand::from_buffer(&buffer[..length]).unwrap();

        match command {
            ArtCommand::Output(output) => {
                if output.port_address == universe_port_addr
                    && output.data.as_ref().len() == DMX_CHANNELS
                {
                    if args.verbose {
                        println!("[demex-node] Received relevant output command {:?}", output);
                    }

                    for i in 0..DMX_CHANNELS {
                        serial.set_channel(i + 1, output.data.as_ref()[i]).unwrap();
                    }

                    serial.update().unwrap();
                }
            }
            ArtCommand::Poll(_) => {
                println!(
                    "[demex-node] Received poll from {:?}, replying..",
                    recv_addr
                );

                if args.verbose {
                    println!("[demex-node] Answering to poll with: {}", poll_reply_str);
                }

                socket.send_to(&poll_reply_buffer, recv_addr).unwrap();
            }
            _ => {}
        }
    }
}
