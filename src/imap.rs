//! Module dedicated to the [`UpgradeTls`] coroutine for the IMAP
//! protocol.

use log::debug;
use memchr::{memchr, memmem};

use io_stream::{
    coroutines::{Read, Write},
    Io,
};

/// Internal state of the [`UpgradeTls`] flow.
#[derive(Debug)]
enum State {
    /// The greeting needs to be discarded.
    DiscardGreeting(Read),
    /// The STARTTLS command needs to be written.
    WriteStartTlsCommand(Write),
    /// The STARTTLS response needs to be discarded.
    DiscardResponse(Read),
}

/// The STARTTLS coroutine that upgrades a plain IMAP (TCP) stream to
/// a secure one.
#[derive(Debug)]
pub struct UpgradeTls {
    state: State,
    bytes: Vec<u8>,
}

impl UpgradeTls {
    /// The STARTTLS IMAP command.
    // TODO: make this customizable?
    const COMMAND: &'static str = "NGC6543 STARTTLS\r\n";

    /// Creates a new STARTTLS coroutine with sane defaults.
    pub fn new() -> Self {
        let state = State::WriteStartTlsCommand(Write::default());
        let bytes = Vec::new();
        Self { state, bytes }
    }

    /// Tells the coroutine how to handle the greeting.
    ///
    /// By default, the coroutine reads and discards the greeting from
    /// the plain stream. This setter may be useful if greeting has
    /// already been read before: in this case, the coroutine will
    /// directly write the STARTTLS command.
    ///
    /// See also [`UpgradeTls::with_discard_greeting`] for the builder
    /// alternative.
    pub fn discard_greeting(&mut self, discard: bool) {
        self.state = if discard {
            State::DiscardGreeting(Read::default())
        } else {
            State::WriteStartTlsCommand(Write::default())
        };
    }

    /// Builder alternative to [`UpgradeTls::discard_greeting`].
    pub fn with_discard_greeting(mut self, discard: bool) -> Self {
        self.discard_greeting(discard);
        self
    }

    /// Makes the coroutine progress.
    pub fn resume(&mut self, mut io: Option<Io>) -> Result<(), Io> {
        loop {
            match &mut self.state {
                State::DiscardGreeting(read) => {
                    let output = read.resume(io.take())?;
                    self.bytes.extend(output.bytes());

                    match memchr(b'\n', &self.bytes) {
                        Some(n) => {
                            let bytes = String::from_utf8_lossy(&self.bytes[..=n]);
                            debug!("discard greeting line {bytes:?}");
                        }
                        None => {
                            read.replace(output.buffer);
                            continue;
                        }
                    };

                    let bytes = Self::COMMAND.as_bytes().to_vec();
                    self.state = State::WriteStartTlsCommand(Write::new(bytes));
                    debug!("enqueue command {:?}", Self::COMMAND);
                }
                State::WriteStartTlsCommand(write) => {
                    write.resume(io.take())?;
                    self.bytes.clear();
                    self.state = State::DiscardResponse(Read::default());
                }
                State::DiscardResponse(read) => {
                    let output = read.resume(io.take())?;
                    self.bytes.extend(output.bytes());

                    // no response line found, keep reading
                    let Some(n) = memmem::find(&self.bytes, b"NGC6543 ") else {
                        read.replace(output.buffer);
                        continue;
                    };

                    match memchr(b'\n', &self.bytes[n..]) {
                        Some(m) => {
                            let bytes = String::from_utf8_lossy(&self.bytes[n..=m]);
                            debug!("discard line {bytes:?}");
                            break Ok(());
                        }
                        None => {
                            read.replace(output.buffer);
                            continue;
                        }
                    };
                }
            }
        }
    }
}
