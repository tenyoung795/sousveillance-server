use std::error;
use std::fmt;
use std::fmt::{Display, Formatter};

pub mod header;

pub use self::header::Header;

pub trait Payload<'a> {
    type Err;
    fn parse(&'a [u8]) -> Result<Self, Self::Err>;
}

impl<'a> Payload<'a> for &'a [u8] {
    type Err = ::Void;
    fn parse(bytes: &'a [u8]) -> Result<Self, Self::Err> {
        Ok(bytes)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    HeaderError(header::Error),
    PayloadError(E),
}

impl<E> From<header::Error> for Error<E> {
    fn from(err: header::Error) -> Self {
        Error::HeaderError(err)
    }
}

impl<E: Display> Display for Error<E> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::HeaderError(ref err) => err.fmt(f),
            Error::PayloadError(ref err) => err.fmt(f),
        }
    }
}

impl<E: error::Error> error::Error for Error<E> {
    fn description(&self) -> &str {
        match *self {
            Error::HeaderError(ref err) => err.description(),
            Error::PayloadError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        Some(match *self {
            Error::HeaderError(ref err) => err,
            Error::PayloadError(ref err) => err,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message<'a, P: 'a> {
    pub header: Header<'a>,
    pub payload: P,
}

impl<'a, P: Payload<'a>> Message<'a, P> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error<P::Err>> {
        let (header, payload_bytes) = try!(Header::parse(bytes));
        P::parse(payload_bytes)
            .map(|payload| Message { header: header, payload: payload })
            .map_err(Error::PayloadError)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::RangeTo;
    use std::time::Duration;

    use ::quickcheck::*;

    use super::*;
    use ::util::testing::*;

    quickcheck_test! {
    bad_header(header: (Vec<u8>, Vec<u8>, u64), payload: Vec<u8>,
               range: RangeTo<usize>; TestResult) {
        let payload_len = payload.len();
        let buf: Vec<_> = (header.0.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(header.0)
            .chain((header.1.len() as u32).to_bytes().into_copy_iter())
            .chain(header.1)
            .chain(header.2.to_bytes().into_copy_iter())
            .chain(payload)
            .collect();

        if range.end >= buf.len() - payload_len {
            TestResult::discard()
        } else {
            type M<'a> = Message<'a, &'a [u8]>;
            TestResult::from_bool(
                matches!(M::parse(&buf[range]), Err(Error::HeaderError(_))))
        }
    }}

    quickcheck_test! {
    bad_payload(header: (Vec<u8>, Vec<u8>, u64), payload: Vec<u8>; bool) {
        #[allow(dead_code)]
        enum P { }
        impl<'a> Payload<'a> for P {
            type Err = ();
            fn parse(_: &'a [u8]) -> Result<Self, Self::Err> {
                Err(())
            }
        }
        type M<'a> = Message<'a, P>;
        let buf: Vec<_> = (header.0.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(header.0)
            .chain((header.1.len() as u32).to_bytes().into_copy_iter())
            .chain(header.1)
            .chain(header.2.to_bytes().into_copy_iter())
            .chain(payload)
            .collect();

        matches!(M::parse(&buf), Err(Error::PayloadError(_)))
    }}

    quickcheck_test! {
    ok_message(header: (Vec<u8>, Vec<u8>, u64), payload: Vec<u8>; bool) {
        let buf: Vec<_> = (header.0.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(header.0.into_copy_iter())
            .chain((header.1.len() as u32).to_bytes().into_copy_iter())
            .chain(header.1.into_copy_iter())
            .chain(header.2.to_bytes().into_copy_iter())
            .chain(payload.into_copy_iter())
            .collect();

        type M<'a> = Message<'a, &'a [u8]>;
        M::parse(&buf) == Ok(Message {
            header: Header {
                token: &header.0,
                id: &header.1,
                timestamp: Duration::from_millis(header.2),
            },
            payload: &*payload,
        })
    }}
}
