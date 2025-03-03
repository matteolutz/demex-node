use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

use artnet_protocol::{ArtCommand, PollReply, PortAddress};
use clap::Parser;
use open_dmx::{DMXSerial, DMX_CHANNELS};
use socket2::{Domain, Protocol, Socket, Type};

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

    /// Interface to bind to (defaults to 0.0.0.0)
    #[arg(short, long)]
    interface: Option<String>,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    env_logger::init();

    let args = Args::parse();

    let (net, sub, uni) = universe_to_net_sub_and_uni(args.universe);

    let universe_port_addr = PortAddress::try_from(args.universe)?;

    let socket_addr: SocketAddr = format!(
        "{}:{}",
        args.interface.unwrap_or_else(|| "0.0.0.0".to_owned()),
        ARTNET_PORT
    )
    .parse()?;

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    // socket.set_reuse_address(true)?;
    // socket.set_broadcast(true)?;

    // socket.bind(&socket_addr.into())?;

    let socket: UdpSocket = UdpSocket::bind(socket_addr)?;

    let mut serial = DMXSerial::open_sync(args.port.as_str()).unwrap();

    log::info!(
        "Listening on {} on port address {} (net {}, sub {}, uni {}), writing to serial port {}...",
        socket.local_addr().unwrap(),
        args.universe,
        net,
        sub,
        uni,
        serial.name()
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
                    log::debug!("Received relevant output command {:?}", output);

                    for i in 0..DMX_CHANNELS {
                        serial.set_channel(i + 1, output.data.as_ref()[i]).unwrap();
                    }

                    serial.update().unwrap();
                }
            }
            ArtCommand::Poll(_) => {
                log::debug!("Received poll from {:?}, replying..", recv_addr);

                log::debug!("Answering to poll with: {}", poll_reply_str);

                socket.send_to(&poll_reply_buffer, recv_addr).unwrap();
            }
            _ => {}
        }
    }
}
