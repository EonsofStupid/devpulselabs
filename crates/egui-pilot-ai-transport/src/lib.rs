//! Bounded, nonblocking EguiPilotAI transport boundary.
//!
//! The UI/runtime sees only [`Transport`]. Platform adapters may use worker
//! threads or browser callbacks, but every call made from an egui pass is
//! nonblocking.

use egui_pilot_ai_protocol::{ClientEnvelope, ServerEnvelope, MAX_ENVELOPE_BYTES};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Closed,
    Backpressure { capacity: usize },
    EnvelopeTooLarge { actual: usize, maximum: usize },
    Io(String),
    Protocol(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => f.write_str("transport is closed"),
            Self::Backpressure { capacity } => {
                write!(f, "transport queue is full (capacity {capacity})")
            }
            Self::EnvelopeTooLarge { actual, maximum } => write!(
                f,
                "transport envelope is {actual} bytes; maximum is {maximum}"
            ),
            Self::Io(message) => write!(f, "transport I/O: {message}"),
            Self::Protocol(message) => write!(f, "transport protocol: {message}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// Application-side endpoint. It receives client commands and emits server
/// events. The trait intentionally has no `Send`, async, or native-I/O bound.
pub trait Transport {
    fn try_receive(&mut self) -> Result<Option<ClientEnvelope>, TransportError>;
    fn try_send(&mut self, envelope: ServerEnvelope) -> Result<(), TransportError>;
    fn close(&mut self) -> Result<(), TransportError>;
    fn is_closed(&self) -> bool;
}

struct MemoryState {
    to_runtime: VecDeque<ClientEnvelope>,
    to_client: VecDeque<ServerEnvelope>,
    capacity: usize,
    closed: bool,
}

/// Runtime half of a deterministic in-memory transport pair.
pub struct InMemoryTransport {
    state: Rc<RefCell<MemoryState>>,
}

/// Client half used by runner and protocol conformance tests.
#[derive(Clone)]
pub struct InMemoryClient {
    state: Rc<RefCell<MemoryState>>,
}

pub fn in_memory_pair(capacity: usize) -> (InMemoryClient, InMemoryTransport) {
    let state = Rc::new(RefCell::new(MemoryState {
        to_runtime: VecDeque::new(),
        to_client: VecDeque::new(),
        capacity: capacity.max(1),
        closed: false,
    }));
    (
        InMemoryClient {
            state: Rc::clone(&state),
        },
        InMemoryTransport { state },
    )
}

impl InMemoryClient {
    pub fn try_send(&self, envelope: ClientEnvelope) -> Result<(), TransportError> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(TransportError::Closed);
        }
        if state.to_runtime.len() >= state.capacity {
            return Err(TransportError::Backpressure {
                capacity: state.capacity,
            });
        }
        state.to_runtime.push_back(envelope);
        Ok(())
    }

    pub fn try_receive(&self) -> Result<Option<ServerEnvelope>, TransportError> {
        let mut state = self.state.borrow_mut();
        if state.closed && state.to_client.is_empty() {
            return Err(TransportError::Closed);
        }
        Ok(state.to_client.pop_front())
    }

    pub fn close(&self) {
        self.state.borrow_mut().closed = true;
    }
}

impl Transport for InMemoryTransport {
    fn try_receive(&mut self) -> Result<Option<ClientEnvelope>, TransportError> {
        let mut state = self.state.borrow_mut();
        if state.closed && state.to_runtime.is_empty() {
            return Err(TransportError::Closed);
        }
        Ok(state.to_runtime.pop_front())
    }

    fn try_send(&mut self, envelope: ServerEnvelope) -> Result<(), TransportError> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(TransportError::Closed);
        }
        if state.to_client.len() >= state.capacity {
            return Err(TransportError::Backpressure {
                capacity: state.capacity,
            });
        }
        state.to_client.push_back(envelope);
        Ok(())
    }

    fn close(&mut self) -> Result<(), TransportError> {
        self.state.borrow_mut().closed = true;
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.state.borrow().closed
    }
}

/// Encode one length-prefixed JSON envelope. A fixed four-byte big-endian
/// length makes partial reads and payload limits unambiguous.
pub fn write_frame<W: std::io::Write, T: serde::Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), TransportError> {
    let payload =
        serde_json::to_vec(value).map_err(|error| TransportError::Protocol(error.to_string()))?;
    if payload.len() > MAX_ENVELOPE_BYTES {
        return Err(TransportError::EnvelopeTooLarge {
            actual: payload.len(),
            maximum: MAX_ENVELOPE_BYTES,
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| TransportError::EnvelopeTooLarge {
        actual: payload.len(),
        maximum: MAX_ENVELOPE_BYTES,
    })?;
    writer
        .write_all(&length.to_be_bytes())
        .and_then(|()| writer.write_all(&payload))
        .and_then(|()| writer.flush())
        .map_err(|error| TransportError::Io(error.to_string()))
}

/// Decode one length-prefixed JSON envelope. Clean EOF before a new frame is
/// `Ok(None)`; EOF inside a frame is an I/O failure.
pub fn read_frame<R: std::io::Read, T: serde::de::DeserializeOwned>(
    reader: &mut R,
) -> Result<Option<T>, TransportError> {
    let mut prefix = [0u8; 4];
    match reader.read_exact(&mut prefix) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(TransportError::Io(error.to_string())),
    }
    let length = u32::from_be_bytes(prefix) as usize;
    if length > MAX_ENVELOPE_BYTES {
        return Err(TransportError::EnvelopeTooLarge {
            actual: length,
            maximum: MAX_ENVELOPE_BYTES,
        });
    }
    let mut payload = vec![0; length];
    reader
        .read_exact(&mut payload)
        .map_err(|error| TransportError::Io(error.to_string()))?;
    serde_json::from_slice(&payload)
        .map(Some)
        .map_err(|error| TransportError::Protocol(error.to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
mod stdio {
    use super::{read_frame, write_frame, Transport, TransportError};
    use egui_pilot_ai_protocol::{ClientEnvelope, ServerEnvelope};
    use std::io::{Read, Write};
    use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};

    enum Outbound {
        Envelope(Box<ServerEnvelope>),
        Close,
    }

    /// Native framed transport. Reader and writer block only on dedicated
    /// worker threads; runtime calls interact with bounded channels.
    pub struct StdioTransport {
        inbound: Receiver<Result<ClientEnvelope, TransportError>>,
        outbound: SyncSender<Outbound>,
        capacity: usize,
        closed: bool,
    }

    impl StdioTransport {
        pub fn stdio(capacity: usize) -> Self {
            Self::from_io(std::io::stdin(), std::io::stdout(), capacity)
        }

        pub fn from_io<R, W>(mut reader: R, mut writer: W, capacity: usize) -> Self
        where
            R: Read + Send + 'static,
            W: Write + Send + 'static,
        {
            let capacity = capacity.max(1);
            let (inbound_tx, inbound) = mpsc::sync_channel(capacity);
            let (outbound, outbound_rx) = mpsc::sync_channel(capacity);

            std::thread::Builder::new()
                .name("eguipilotai-stdio-read".into())
                .spawn(move || loop {
                    match read_frame(&mut reader) {
                        Ok(Some(envelope)) => {
                            if inbound_tx.send(Ok(envelope)).is_err() {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(error) => {
                            let _ = inbound_tx.send(Err(error));
                            break;
                        }
                    }
                })
                .expect("EguiPilotAI stdio reader thread");

            std::thread::Builder::new()
                .name("eguipilotai-stdio-write".into())
                .spawn(move || {
                    while let Ok(message) = outbound_rx.recv() {
                        match message {
                            Outbound::Envelope(envelope) => {
                                if write_frame(&mut writer, envelope.as_ref()).is_err() {
                                    break;
                                }
                            }
                            Outbound::Close => break,
                        }
                    }
                })
                .expect("EguiPilotAI stdio writer thread");

            Self {
                inbound,
                outbound,
                capacity,
                closed: false,
            }
        }
    }

    impl Transport for StdioTransport {
        fn try_receive(&mut self) -> Result<Option<ClientEnvelope>, TransportError> {
            if self.closed {
                return Err(TransportError::Closed);
            }
            match self.inbound.try_recv() {
                Ok(result) => result.map(Some),
                Err(TryRecvError::Empty) => Ok(None),
                Err(TryRecvError::Disconnected) => {
                    self.closed = true;
                    Err(TransportError::Closed)
                }
            }
        }

        fn try_send(&mut self, envelope: ServerEnvelope) -> Result<(), TransportError> {
            if self.closed {
                return Err(TransportError::Closed);
            }
            match self
                .outbound
                .try_send(Outbound::Envelope(Box::new(envelope)))
            {
                Ok(()) => Ok(()),
                Err(TrySendError::Full(_)) => Err(TransportError::Backpressure {
                    capacity: self.capacity,
                }),
                Err(TrySendError::Disconnected(_)) => {
                    self.closed = true;
                    Err(TransportError::Closed)
                }
            }
        }

        fn close(&mut self) -> Result<(), TransportError> {
            if !self.closed {
                let _ = self.outbound.try_send(Outbound::Close);
                self.closed = true;
            }
            Ok(())
        }

        fn is_closed(&self) -> bool {
            self.closed
        }
    }

    impl Drop for StdioTransport {
        fn drop(&mut self) {
            let _ = self.close();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use stdio::StdioTransport;

#[cfg(test)]
mod tests {
    use super::*;
    use egui_pilot_ai_protocol::{
        ClientCommand, ClientEnvelope, ServerEnvelope, ServerEvent, SessionId,
    };
    use std::io::Cursor;

    fn client(sequence: u64) -> ClientEnvelope {
        ClientEnvelope {
            sequence,
            session: SessionId::new("test"),
            command: ClientCommand::Status,
        }
    }

    fn server(sequence: u64) -> ServerEnvelope {
        ServerEnvelope {
            sequence,
            session: SessionId::new("test"),
            event: ServerEvent::Stopped,
        }
    }

    #[test]
    fn in_memory_pair_is_bidirectional() {
        let (client_side, mut runtime) = in_memory_pair(2);
        client_side.try_send(client(1)).unwrap();
        assert_eq!(runtime.try_receive().unwrap().unwrap().sequence, 1);
        runtime.try_send(server(2)).unwrap();
        assert_eq!(client_side.try_receive().unwrap().unwrap().sequence, 2);
    }

    #[test]
    fn in_memory_pair_reports_backpressure() {
        let (client_side, _runtime) = in_memory_pair(1);
        client_side.try_send(client(1)).unwrap();
        assert_eq!(
            client_side.try_send(client(2)),
            Err(TransportError::Backpressure { capacity: 1 })
        );
    }

    #[test]
    fn framed_json_roundtrips() {
        let expected = client(42);
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &expected).unwrap();
        let decoded: ClientEnvelope = read_frame(&mut Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn oversized_prefix_is_rejected_before_allocation() {
        let bytes = ((MAX_ENVELOPE_BYTES as u32) + 1).to_be_bytes();
        let error = read_frame::<_, ClientEnvelope>(&mut Cursor::new(bytes)).unwrap_err();
        assert!(matches!(error, TransportError::EnvelopeTooLarge { .. }));
    }
}
