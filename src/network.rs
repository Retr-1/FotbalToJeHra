use crate::game::{Command, Game};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};
#[derive(Serialize, Deserialize)]
pub enum Message {
    State(Game),
    Input(Command),
}
pub struct Network {
    listener: Option<TcpListener>,
    stream: Option<TcpStream>,
    incoming: Vec<u8>,
    outgoing: VecDeque<Vec<u8>>,
    offset: usize,
    pub status: String,
    pub host: bool,
}
impl Network {
    pub fn host(address: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener: Some(listener),
            stream: None,
            incoming: vec![],
            outgoing: VecDeque::new(),
            offset: 0,
            status: format!("Cakam na hraca: {address}"),
            host: true,
        })
    }
    pub fn join(address: &str) -> io::Result<Self> {
        use std::net::ToSocketAddrs;
        let addr = address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::other("Neplatna adresa"))?;
        let s = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
        s.set_nonblocking(true)?;
        s.set_nodelay(true)?;
        Ok(Self {
            listener: None,
            stream: Some(s),
            incoming: vec![],
            outgoing: VecDeque::new(),
            offset: 0,
            status: "Pripojeny".into(),
            host: false,
        })
    }
    pub fn connected(&self) -> bool {
        self.stream.is_some()
    }
    pub fn send(&mut self, m: Message) {
        if !self.connected() {
            return;
        }
        if self.outgoing.len() > 120 {
            self.disconnect("Pripojenie je prilis pomale");
            return;
        }
        if let Ok(mut bytes) = serde_json::to_vec(&m) {
            bytes.push(b'\n');
            self.outgoing.push_back(bytes);
        }
    }
    fn disconnect(&mut self, why: &str) {
        self.stream = None;
        self.listener = None;
        self.outgoing.clear();
        self.incoming.clear();
        self.offset = 0;
        self.status = why.into();
    }
    pub fn poll(&mut self) -> Vec<Message> {
        if self.stream.is_none() {
            if let Some(l) = &self.listener {
                match l.accept() {
                    Ok((s, _)) => {
                        let _ = s.set_nonblocking(true);
                        let _ = s.set_nodelay(true);
                        self.stream = Some(s);
                        self.status = "Pripojeny".into();
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => self.status = e.to_string(),
                }
            }
        }
        let mut messages = vec![];
        let mut failure = None;
        if let Some(s) = &mut self.stream {
            let mut buf = [0u8; 8192];
            for _ in 0..32 {
                match s.read(&mut buf) {
                    Ok(0) => {
                        failure = Some("Super sa odpojil".to_string());
                        break;
                    }
                    Ok(n) => {
                        self.incoming.extend_from_slice(&buf[..n]);
                        if self.incoming.len() > 262144 {
                            failure = Some("Prilis velka sprava".into());
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
            while let Some(end) = self.incoming.iter().position(|x| *x == b'\n') {
                let line: Vec<_> = self.incoming.drain(..=end).collect();
                match serde_json::from_slice(&line) {
                    Ok(m) => messages.push(m),
                    Err(_) => {
                        failure = Some("Neplatna sietova sprava".into());
                        break;
                    }
                }
            }
            while let Some(bytes) = self.outgoing.front() {
                match s.write(&bytes[self.offset..]) {
                    Ok(0) => {
                        failure = Some("Spojenie bolo uzavrete".into());
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
        if let Some(e) = failure {
            self.disconnect(&e)
        }
        messages
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tcp_round_trip() {
        let mut host = Network::host("127.0.0.1:0").unwrap();
        let address = host.listener.as_ref().unwrap().local_addr().unwrap();
        let mut client = Network::join(&address.to_string()).unwrap();
        host.poll();
        host.send(Message::State(Game::new()));
        host.poll();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            if client
                .poll()
                .iter()
                .any(|m| matches!(m,Message::State(g) if g.setup==15.))
            {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        client.send(Message::Input(Command {
            player: 5,
            target: crate::game::Point::new(4., 2.),
            round: 1,
        }));
        client.poll();
        loop {
            if host
                .poll()
                .iter()
                .any(|m| matches!(m,Message::Input(c) if c.player==5))
            {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
}
