use byteorder::{BigEndian, ByteOrder};
use std::error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::prelude::*;
use std::mem;

use {message, server, Message, Server, Stream};

pub struct Session<'a, S: 'a, R> {
    server: &'a mut S,
    reader: R,
    buffer: Vec<u8>,
}

impl<'a, S: 'a , R> Session<'a, S, R> {
    pub fn new(server: &'a mut S, reader: R) -> Self {
        Session {
            server: server,
            reader: reader,
            buffer: vec![],
        }
    }
}

#[derive(Debug)]
pub enum Error<A, P> {
    Read(io::Error),
    OneByteMessageSize,
    Truncated {
        found: u16,
        remaining: u16,
    },
    Parse(message::Error),
    Consume(server::ConsumeError<A, P>),
}

impl<A, P> From<message::Error> for Error<A, P> {
    fn from(e: message::Error) -> Self {
        Error::Parse(e)
    }
}

impl<A, P> From<server::ConsumeError<A, P>> for Error<A, P> {
    fn from(e: server::ConsumeError<A, P>) -> Self {
        Error::Consume(e)
    }
}

impl<A, P> From<io::Error> for Error<A, P> {
    fn from(e: io::Error) -> Self {
        Error::Read(e)
    }
}

impl<A: Display, P: Display> Display for Error<A, P> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::Read(ref e) => e.fmt(f),
            Error::OneByteMessageSize => f.write_str("one-byte message size"),
            Error::Truncated { found, remaining } => write!(
                f, "{} bytes of message found; {} bytes remaining",
                found, remaining),
            Error::Parse(ref e) => e.fmt(f),
            Error::Consume(ref e) => e.fmt(f),
        }
    }
}

impl<A: error::Error, P: error::Error> error::Error for Error<A, P> {
    fn description(&self) -> &str {
        match *self {
            Error::Read(ref e) => e.description(),
            Error::OneByteMessageSize => "one-byte message size",
            Error::Truncated { .. } => "truncated message",
            Error::Parse(ref e) => e.description(),
            Error::Consume(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Read(ref e) => Some(e),
            Error::Parse(ref e) => Some(e),
            Error::Consume(ref e) => Some(e),
            _ => None,
        }
    }
}

impl<'a, S: 'a + Server, R: Read> Iterator for Session<'a, S, R> {
    type Item = Result<
        Vec<u8>,
        Error<S::AuthErr, <S::Stream as Stream>::PushErr>>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut bytes: [u8; 2] = unsafe { mem::uninitialized() };
        match self.reader.read(&mut bytes) {
            Err(e) => Some(Err(e.into())),
            Ok(n) => match n {
                0 => None,
                1 => Some(Err(Error::OneByteMessageSize)),
                2 => Some({
                    let size = BigEndian::read_u16(&bytes) as usize;
                    if let Some(additional) = size.checked_sub(self.buffer.len()) {
                        self.buffer.reserve(additional);
                    }
                    unsafe {
                        self.buffer.set_len(size);
                    }
                    match self.reader.read(&mut self.buffer) {
                        Err(e) => Err(e.into()),
                        Ok(found) if found < size => Err(Error::Truncated {
                            found: found as u16,
                            remaining: (size - found) as u16,
                        }),
                        Ok(n) if n == size => {
                            let server = &mut *self.server;
                            Message::parse(&mut self.buffer)
                                .map_err(Into::into)
                                .and_then(|msg| {
                                    let id = msg.header.id;
                                    server.consume(msg)
                                          .map_err(Into::into)
                                          .map(|()| id.to_owned())
                                })
                        }
                        Ok(n) => unreachable!("{} should be <= {}", n, size),
                    }
                }),
                n => unreachable!("{} should be <= 2", n),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::io::prelude::*;
    use std::io::Cursor;

    use super::*;
    use {server, stream};
    use testing::*;

    #[derive(Clone, Debug, Default)]
    struct Packet {
        token: Vec<u8>,
        id: Vec<u8>,
        millis: u64,
        payload: Vec<u8>,
    }
    impl Packet {
        fn into_bytes(self) -> Vec<u8> {
            let msg: Vec<_> = (self.token.len() as u16)
                                  .to_bytes()
                                  .into_copy_iter()
                                  .chain(self.token)
                                  .chain((self.id.len() as u16).to_bytes().into_copy_iter())
                                  .chain(self.id)
                                  .chain(self.millis.to_bytes().into_copy_iter())
                                  .chain(self.payload)
                                  .collect();
            (msg.len() as u16)
                .to_bytes()
                .into_copy_iter()
                .chain(msg)
                .collect()
        }
    }
    impl Arbitrary for Packet {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Packet {
                token: Arbitrary::arbitrary(g),
                id: Arbitrary::arbitrary(g),
                millis: Arbitrary::arbitrary(g),
                payload: Arbitrary::arbitrary(g),
            }
        }

        fn shrink(&self) -> Box<Iterator<Item = Self>> {
            Box::new(self.token
                         .shrink()
                         .zip(self.id.shrink())
                         .zip(self.millis.shrink())
                         .zip(self.payload.shrink())
                         .map(|(((token, id), millis), payload)| {
                             Packet {
                                 token: token,
                                 id: id,
                                 millis: millis,
                                 payload: payload,
                             }
                         }))
        }
    }

    #[test]
    fn next_none() {
        let mut server = server::mocks::Unreachable;
        let mut session = Session::new(&mut server, io::empty());
        assert_match!(None, session.next());
    }

    #[test]
    fn next_some_err_read() {
        let mut server = server::mocks::Unreachable;
        struct BrokenRead;
        impl Read for BrokenRead {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::Other, ""))
            }
        }
        let mut session = Session::new(&mut server, BrokenRead);
        assert_match!(Some(Err(Error::Read(_))), session.next());
    }

    quickcheck_test! {
    next_some_err_one_byte_message_size(partial_message_size: u8; TestResult) {
        let mut server = server::mocks::Unreachable;
        let packet = [partial_message_size];
        let mut session = Session::new(&mut server, &packet as &[_]);
        test_result_match!(Some(Err(Error::OneByteMessageSize)), session.next())
    }}

    quickcheck_test! {
    next_some_err_truncated(partial_message: Vec<u8>, expected_remaining: u16; TestResult) {
        if expected_remaining == 0 {
            return TestResult::discard();
        }

        let expected_found = partial_message.len() as u16;
        if let Some(n) = expected_found.checked_add(expected_remaining) {
            let mut server = server::mocks::Unreachable;
            let len_bytes = &n.to_bytes();
            let bytes = len_bytes.chain(Cursor::new(partial_message));
            let mut session = Session::new(&mut server, bytes);
            test_result_match!(Some(Err(Error::Truncated {
                found,
                remaining,
            })) if found == expected_found && remaining == expected_remaining, session.next())
        } else {
            TestResult::discard()
        }
    }}

    quickcheck_test! {
    next_some_err_parse(partial_token: Vec<u8>, missing: u16; TestResult) {
        if missing == 0 {
            return TestResult::discard();
        }

        let found = partial_token.len() as u16;
        if let Some(token_len) = found.checked_add(missing) {
            let mut server = server::mocks::Unreachable;
            let msg: Vec<_> = token_len.to_bytes()
                .into_copy_iter()
                .chain(partial_token)
                .collect();
            let msg_len = &(msg.len() as u16).to_bytes();
            let packet = msg_len.chain(Cursor::new(msg));
            let mut session = Session::new(&mut server, packet);
            test_result_match!(Some(Err(Error::Parse(_))), session.next())
        } else {
            TestResult::discard()
        }
    }}

    quickcheck_test! {
    next_some_err_consume(packet: Packet; TestResult) {
        let mut server = server::mocks::RefuseToAuth;
        let mut session = Session::new(&mut server, Cursor::new(packet.into_bytes()));
        test_result_match!(Some(Err(Error::Consume(_))), session.next())
    }}

    quickcheck_test! {
    next_some_ok(packet: Packet; TestResult) {
        let mut finder = server::Finder::new();
        let expected_id = packet.id.clone();
        finder.insert(expected_id.clone(), stream::mocks::Ok);
        let mut server = server::mocks::Ok(finder);
        let mut session = Session::new(&mut server, Cursor::new(packet.into_bytes()));
        test_result_match!(Some(Ok(ref id)) if id == &expected_id, session.next())
    }}
}
