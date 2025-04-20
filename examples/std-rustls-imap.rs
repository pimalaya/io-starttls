#![cfg(feature = "imap")]

use std::{
    env,
    io::{stdin, stdout, Write as _},
    net::TcpStream,
    sync::Arc,
};

use io_starttls::imap::UpgradeTls;
use io_stream::{
    coroutines::{Read, Write},
    runtimes::std::handle,
};
use log::info;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use rustls_platform_verifier::ConfigVerifierExt;

fn main() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "debug");
    }

    env_logger::init();

    let host = match env::var("HOST") {
        Ok(host) => host,
        Err(_) => prompt("TCP server host?"),
    };

    let port: u16 = match env::var("PORT") {
        Ok(port) => port.parse().unwrap(),
        Err(_) => prompt("TCP server port?").parse().unwrap(),
    };

    let mut tcp = TcpStream::connect((host.as_str(), port)).unwrap();

    let mut input = None;
    let mut starttls = UpgradeTls::new().with_discard_greeting(true);

    while let Err(io) = starttls.resume(input) {
        input = Some(handle(&mut tcp, io).unwrap());
    }

    info!("upgrade current TCP stream to TLS");
    let config = ClientConfig::with_platform_verifier();
    let server_name = host.to_string().try_into().unwrap();
    let conn = ClientConnection::new(Arc::new(config), server_name).unwrap();
    let mut tls = StreamOwned::new(conn, tcp);

    info!("send NOOP command via TLS");
    let mut input = None;
    let mut write = Write::new(b"A NOOP\r\n".to_vec());

    while let Err(io) = write.resume(input) {
        input = Some(handle(&mut tls, io).unwrap());
    }

    let mut input = None;
    let mut read = Read::default();

    let output = loop {
        match read.resume(input) {
            Ok(output) => break output,
            Err(io) => input = Some(handle(&mut tls, io).unwrap()),
        }
    };

    let bytes = String::from_utf8_lossy(output.bytes());
    info!("receive NOOP response via TLS: {bytes:?}");
}

fn prompt(message: &str) -> String {
    print!("{message} ");
    stdout().flush().unwrap();

    let mut line = String::new();
    stdin().read_line(&mut line).unwrap();

    line.trim().to_owned()
}
