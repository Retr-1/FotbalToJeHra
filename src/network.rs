use crate::{
    game::{Command, Game},
    udp_packet::{self, Payload, Token},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    time::{Duration, Instant},
};

pub enum Message {
    Snapshot { time: f64, state: Game, ack: u64 },
    InputSequenced { sequence: u64, command: Command },
}
#[derive(Serialize, Deserialize)]
enum Reliable {
    Welcome {
        version: u16,
        token: Token,
        udp_port: u16,
    },
    InputSequenced {
        sequence: u64,
        command: Command,
    },
}
const VERSION: u16 = 1;
const TIMEOUT: Duration = Duration::from_secs(8);

pub struct Network {
    listener: Option<TcpListener>,
    stream: Option<TcpStream>,
    udp: UdpSocket,
    tcp_peer: Option<SocketAddr>,
    udp_peer: Option<SocketAddr>,
    session: Option<Token>,
    udp_ready: bool,
    sequence: u64,
    last_sequence: u64,
    handshake: Option<Instant>,
    last_registration: Option<Instant>,
    last_udp: Option<Instant>,
    incoming: Vec<u8>,
    outgoing: VecDeque<Vec<u8>>,
    offset: usize,
    pub status: String,
    pub host: bool,
    pub host_address: Option<String>,
}
impl Network {
    pub fn host(address: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        let bound = listener.local_addr()?;
        // TCP and UDP have separate port namespaces; use the same port for both.
        let udp = UdpSocket::bind(bound)?;
        udp.set_nonblocking(true)?;
        let host_address = if !bound.ip().is_unspecified() {
            Some(bound.to_string())
        } else {
            (|| -> io::Result<String> {
                let route = UdpSocket::bind("0.0.0.0:0")?;
                route.connect("192.0.2.1:9")?;
                let local = route.local_addr()?;
                if local.ip().is_unspecified() || local.ip().is_loopback() {
                    return Err(io::Error::other("LAN adresa nie je dostupna"));
                }
                Ok(SocketAddr::new(local.ip(), bound.port()).to_string())
            })()
            .ok()
        };
        Ok(Self {
            listener: Some(listener),
            stream: None,
            udp,
            tcp_peer: None,
            udp_peer: None,
            session: None,
            udp_ready: false,
            sequence: 0,
            last_sequence: 0,
            handshake: None,
            last_registration: None,
            last_udp: None,
            incoming: vec![],
            outgoing: VecDeque::new(),
            offset: 0,
            status: format!("Cakam na hraca: {address}"),
            host: true,
            host_address,
        })
    }
    pub fn join(address: &str) -> io::Result<Self> {
        use std::net::ToSocketAddrs;
        let address = address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::other("Neplatna adresa"))?;
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;
        let peer = stream.peer_addr()?;
        let udp = UdpSocket::bind(if peer.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        })?;
        udp.set_nonblocking(true)?;
        Ok(Self {
            listener: None,
            stream: Some(stream),
            udp,
            tcp_peer: Some(peer),
            udp_peer: None,
            session: None,
            udp_ready: false,
            sequence: 0,
            last_sequence: 0,
            handshake: Some(Instant::now()),
            last_registration: None,
            last_udp: None,
            incoming: vec![],
            outgoing: VecDeque::new(),
            offset: 0,
            status: "Pripajam UDP...".into(),
            host: false,
            host_address: None,
        })
    }
    pub fn connected(&self) -> bool {
        self.stream.is_some() && self.udp_ready
    }
    pub fn waiting(&self) -> bool {
        self.host && self.listener.is_some() && !self.connected()
    }
    pub fn closed(&self) -> bool {
        self.stream.is_none() && self.listener.is_none()
    }
    fn queue(&mut self, message: Reliable) {
        if self.stream.is_none() {
            return;
        }
        if self.outgoing.len() >= 120 {
            self.disconnect("Pripojenie je prilis pomale");
            return;
        }
        if let Ok(mut bytes) = serde_json::to_vec(&message) {
            bytes.push(b'\n');
            self.outgoing.push_back(bytes);
        }
    }
    pub fn send(&mut self, message: Message) {
        match message {
            Message::InputSequenced { sequence, command } if !self.host => {
                self.queue(Reliable::InputSequenced { sequence, command })
            }
            Message::Snapshot { time, state, ack } if self.host && self.connected() => {
                self.sequence += 1;
                if let (Some(token), Some(peer)) = (self.session, self.udp_peer) {
                    if let Some(bytes) =
                        udp_packet::snapshot(token, self.sequence, time, &state, ack)
                    {
                        // A blocked/lost state is discarded. A newer complete
                        // snapshot follows; never queue or retransmit old state.
                        let _ = self.udp.send_to(&bytes, peer);
                    }
                }
            }
            _ => {}
        }
    }
    fn disconnect(&mut self, why: &str) {
        self.stream = None;
        self.listener = None;
        self.udp_peer = None;
        self.session = None;
        self.udp_ready = false;
        self.outgoing.clear();
        self.incoming.clear();
        self.offset = 0;
        self.status = why.into();
    }
    fn accept(&mut self) {
        if self.stream.is_some() {
            return;
        }
        let Some(listener) = &self.listener else {
            return;
        };
        match listener.accept() {
            Ok((stream, peer)) => {
                let mut token = [0; 16];
                if stream.set_nonblocking(true).is_err()
                    || stream.set_nodelay(true).is_err()
                    || getrandom::fill(&mut token).is_err()
                {
                    self.disconnect("Nepodarilo sa pripravit spojenie");
                    return;
                }
                self.stream = Some(stream);
                self.tcp_peer = Some(peer);
                self.session = Some(token);
                self.handshake = Some(Instant::now());
                self.status = "Pripajam UDP...".into();
                let port = self.udp.local_addr().unwrap().port();
                self.queue(Reliable::Welcome {
                    version: VERSION,
                    token,
                    udp_port: port,
                });
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => self.status = e.to_string(),
        }
    }
    pub fn poll(&mut self) -> Vec<Message> {
        self.accept();
        let mut messages = vec![];
        let mut failure = None;
        if let Some(stream) = &mut self.stream {
            let mut bytes = [0; 8192];
            for _ in 0..16 {
                match stream.read(&mut bytes) {
                    Ok(0) => {
                        failure = Some("Super sa odpojil".to_string());
                        break;
                    }
                    Ok(n) => {
                        self.incoming.extend_from_slice(&bytes[..n]);
                        if self.incoming.len() > 65536 {
                            failure = Some("Prilis velka TCP sprava".into());
                            break;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => {
                        failure = Some(e.to_string());
                        break;
                    }
                }
            }
        }
        while let Some(end) = self.incoming.iter().position(|b| *b == b'\n') {
            let line: Vec<_> = self.incoming.drain(..=end).collect();
            match serde_json::from_slice::<Reliable>(&line) {
                Ok(Reliable::Welcome {
                    version,
                    token,
                    udp_port,
                }) if !self.host => {
                    if version != VERSION || udp_port == 0 {
                        failure = Some("Nekompatibilna verzia hry".into());
                        break;
                    }
                    if self.session.is_none() {
                        self.session = Some(token);
                        self.udp_peer = self.tcp_peer.map(|p| SocketAddr::new(p.ip(), udp_port));
                    }
                }
                Ok(Reliable::InputSequenced { sequence, command }) if self.host => {
                    messages.push(Message::InputSequenced { sequence, command })
                }
                _ => {
                    failure = Some("Nekompatibilna TCP sprava - aktualizuj obe zariadenia".into());
                    break;
                }
            }
        }
        if let Some(error) = failure {
            self.disconnect(&error);
            return vec![];
        }
        self.register_udp();
        self.receive_udp(&mut messages);
        self.flush();
        let timeout = if self.udp_ready {
            self.last_udp.is_some_and(|t| t.elapsed() > TIMEOUT)
        } else {
            self.handshake.is_some_and(|t| t.elapsed() > TIMEOUT)
        };
        if self.stream.is_some() && timeout {
            self.disconnect(
                "UDP spojenie neodpoveda. Povol TCP aj UDP port hostitela vo firewalle.",
            );
        }
        messages
    }
    fn register_udp(&mut self) {
        if self.host || self.stream.is_none() {
            return;
        }
        let interval = if self.udp_ready {
            Duration::from_secs(1)
        } else {
            Duration::from_millis(200)
        };
        if self
            .last_registration
            .is_some_and(|t| t.elapsed() < interval)
        {
            return;
        }
        if let (Some(token), Some(peer)) = (self.session, self.udp_peer) {
            let _ = self.udp.send_to(&udp_packet::registration(token), peer);
            self.last_registration = Some(Instant::now());
        }
    }
    fn receive_udp(&mut self, messages: &mut Vec<Message>) {
        let Some(token) = self.session else { return };
        let mut bytes = [0; udp_packet::MAX_PACKET + 1];
        for _ in 0..64 {
            let (length, from) = match self.udp.recv_from(&mut bytes) {
                Ok(received) => received,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            };
            let Some(packet) = udp_packet::decode(&bytes[..length]) else {
                continue;
            };
            if packet.token != token {
                continue;
            }
            match packet.payload {
                Payload::Register
                    if self.host && self.tcp_peer.is_some_and(|p| p.ip() == from.ip()) =>
                {
                    if self.udp_peer.is_some_and(|p| p != from) {
                        continue;
                    }
                    self.udp_peer = Some(from);
                    self.mark_ready();
                    let _ = self.udp.send_to(&udp_packet::ready(token), from);
                }
                Payload::Ready if !self.host && self.udp_peer == Some(from) => self.mark_ready(),
                Payload::Snapshot { time, state, ack }
                    if !self.host && self.udp_peer == Some(from) =>
                {
                    if packet.sequence <= self.last_sequence {
                        continue;
                    }
                    self.last_sequence = packet.sequence;
                    self.mark_ready();
                    messages.push(Message::Snapshot { time, state, ack });
                }
                _ => {}
            }
        }
    }
    fn mark_ready(&mut self) {
        self.udp_ready = true;
        self.last_udp = Some(Instant::now());
        self.status = "Pripojeny (UDP)".into();
    }
    /// Flush commands without consuming any incoming TCP/UDP messages.
    pub fn flush(&mut self) {
        let mut failure = None;
        if let Some(stream) = &mut self.stream {
            while let Some(bytes) = self.outgoing.front() {
                match stream.write(&bytes[self.offset..]) {
                    Ok(0) => {
                        failure = Some("Spojenie bolo uzavrete".to_string());
                        break;
                    }
                    Ok(n) => {
                        self.offset += n;
                        if self.offset == bytes.len() {
                            self.outgoing.pop_front();
                            self.offset = 0;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => {
                        failure = Some(e.to_string());
                        break;
                    }
                }
            }
        }
        if let Some(error) = failure {
            self.disconnect(&error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> (Network, Network) {
        let mut host = Network::host("127.0.0.1:0").unwrap();
        let address = host.listener.as_ref().unwrap().local_addr().unwrap();
        assert_eq!(
            host.host_address.as_deref(),
            Some(address.to_string().as_str())
        );
        assert!(host.waiting());
        let mut client = Network::join(&address.to_string()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !host.connected() || !client.connected() {
            host.poll();
            client.poll();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(!host.waiting());
        (host, client)
    }
    #[test]
    fn udp_states_and_reliable_commands_round_trip() {
        let (mut host, mut client) = pair();
        assert_eq!(
            host.udp.local_addr().unwrap().port(),
            host.stream.as_ref().unwrap().local_addr().unwrap().port()
        );
        host.send(Message::Snapshot {
            time: 0.25,
            state: Game::new(),
            ack: 0,
        });
        assert!(host.outgoing.is_empty());
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if client
                .poll()
                .iter()
                .any(|m| matches!(m,Message::Snapshot{time,..} if *time==0.25))
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        client.send(Message::InputSequenced {
            sequence: 7,
            command: Command {
                player: crate::game::COUNT,
                target: crate::game::Point::new(4., 2.),
                round: 1,
            },
        });
        client.flush();
        loop {
            if host
                .poll()
                .iter()
                .any(|m| matches!(m, Message::InputSequenced { sequence: 7, .. }))
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        host.send(Message::Snapshot {
            time: 0.3,
            state: Game::new(),
            ack: 7,
        });
        loop {
            if client
                .poll()
                .iter()
                .any(|m| matches!(m, Message::Snapshot { ack: 7, .. }))
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    fn stale_duplicate_foreign_and_lost_datagrams_do_not_block_new_state() {
        let (host, mut client) = pair();
        let token = host.session.unwrap();
        let peer = host.udp_peer.unwrap();
        for sequence in [5, 3, 5] {
            host.udp
                .send_to(
                    &udp_packet::snapshot(token, sequence, sequence as f64, &Game::new(), 0)
                        .unwrap(),
                    peer,
                )
                .unwrap();
        }
        host.udp
            .send_to(
                &udp_packet::snapshot([0; 16], 99, 99., &Game::new(), 0).unwrap(),
                peer,
            )
            .unwrap();
        let foreign = UdpSocket::bind("127.0.0.1:0").unwrap();
        foreign
            .send_to(
                &udp_packet::snapshot(token, 100, 100., &Game::new(), 0).unwrap(),
                peer,
            )
            .unwrap();
        host.udp
            .send_to(
                &udp_packet::snapshot(token, 6, 6., &Game::new(), 0).unwrap(),
                peer,
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut times = vec![];
        while !times.contains(&6.) {
            for m in client.poll() {
                if let Message::Snapshot { time, .. } = m {
                    times.push(time);
                }
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(times, vec![5., 6.]);
    }
    #[test]
    fn lost_udp_registration_and_confirmation_are_retried() {
        fn discard(socket: &UdpSocket) {
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut bytes = [0; 1200];
            loop {
                match socket.recv_from(&mut bytes) {
                    Ok(_) => return,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => panic!("{e}"),
                }
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
        let mut host = Network::host("127.0.0.1:0").unwrap();
        let mut client = Network::join(
            &host
                .listener
                .as_ref()
                .unwrap()
                .local_addr()
                .unwrap()
                .to_string(),
        )
        .unwrap();
        host.poll();
        let deadline = Instant::now() + Duration::from_secs(2);
        while client.session.is_none() {
            client.poll();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        discard(&host.udp); // lose first registration
        client.last_registration = Some(Instant::now() - Duration::from_secs(1));
        client.poll();
        while !host.connected() {
            host.poll();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        discard(&client.udp); // lose first confirmation
        assert!(host.connected());
        assert!(!client.connected());
        client.last_registration = Some(Instant::now() - Duration::from_secs(1));
        let deadline = Instant::now() + Duration::from_secs(2);
        while !client.connected() {
            client.poll();
            host.poll();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    fn tcp_only_peer_times_out_with_udp_error() {
        let mut host = Network::host("127.0.0.1:0").unwrap();
        let _peer =
            TcpStream::connect(host.listener.as_ref().unwrap().local_addr().unwrap()).unwrap();
        host.poll();
        assert!(!host.connected());
        host.handshake = Some(Instant::now() - TIMEOUT - Duration::from_secs(1));
        host.poll();
        assert!(host.closed());
        assert!(host.status.contains("UDP"));
    }
}
