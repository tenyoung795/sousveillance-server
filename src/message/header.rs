use byteorder::{BigEndian, ByteOrder};
use std::error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum Part {
    TokenSize,
    Token(u32),
    IdSize,
    Id(u32),
    Timestamp,
}

impl Part {
    fn size(&self) -> u32 {
        match *self {
            Part::TokenSize | Part::IdSize => 4,
            Part::Token(s) => s,
            Part::Id(s) => s,
            Part::Timestamp => 8,
        }
    }

    fn description(&self) -> &'static str {
        match *self {
            Part::TokenSize => "token size",
            Part::Token(_) => "token",
            Part::IdSize => "Id size",
            Part::Id(_) => "Id",
            Part::Timestamp => "timestamp",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Header<'a> {
    pub token: &'a [u8],
    pub id: &'a [u8],
    pub timestamp: Duration,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    pub remaining: u32,
    pub part: Part,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f,
               "missing {} of {} bytes; {} bytes remaining",
               self.part.description(),
               self.part.size(),
               self.remaining)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self.part {
            Part::TokenSize => "missing token size",
            Part::Token(_) => "missing token",
            Part::IdSize => "missing Id size",
            Part::Id(_) => "missing Id",
            Part::Timestamp => "missing timestamp",
        }
    }
}

impl<'a> Header<'a> {
    pub fn parse(mut bytes: &'a [u8]) -> Result<(Self, &'a [u8]), Error> {
        let mut remaining = bytes.len() as u32;
        let mut check = |part: Part| {
            remaining = try!(remaining.checked_sub(part.size()).ok_or(Error {
                remaining: remaining,
                part: part,
            }));
            Ok(())
        };

        try!(check(Part::TokenSize));
        let token_size = BigEndian::read_u32(bytes);
        bytes = &bytes[4..];

        try!(check(Part::Token(token_size)));
        let token = &bytes[..token_size as usize];
        bytes = &bytes[token_size as usize..];

        try!(check(Part::IdSize));
        let id_size = BigEndian::read_u32(bytes);
        bytes = &bytes[4..];

        try!(check(Part::Id(id_size)));
        let id = &bytes[..id_size as usize];
        bytes = &bytes[id_size as usize..];

        try!(check(Part::Timestamp));
        let timestamp = Duration::from_millis(BigEndian::read_u64(bytes));
        bytes = &bytes[8..];

        let header = Header {
            token: token,
            id: id,
            timestamp: timestamp,
        };
        Ok((header, bytes))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use testing::*;

    #[test]
    fn none_of_token_size() {
        assert_eq!(Err(Error {
                       remaining: 0,
                       part: Part::TokenSize,
                   }),
                   Header::parse(&[]));
    }

    quickcheck_test! {
    one_of_token_size(byte: u8; bool) {
        Header::parse(&[byte]) == Err(Error {
            remaining: 1,
            part: Part::TokenSize,
        })
    }}

    quickcheck_test! {
    two_of_token_size(a: u8, b: u8; bool) {
        Header::parse(&[a, b]) == Err(Error {
            remaining: 2,
            part: Part::TokenSize,
        })
    }}

    quickcheck_test! {
    three_of_token_size(a: u8, b: u8, c: u8; bool) {
        Header::parse(&[a, b, c]) == Err(Error {
            remaining: 3,
            part: Part::TokenSize,
        })
    }}

    quickcheck_test! {
    partial_token(partial_token: Vec<u8>, needed: u32; TestResult) {
        let remaining = partial_token.len() as u32;
        if needed > 0 {
            if let Some(token_size) = remaining.checked_add(needed) {
                let buf: Vec<_> = token_size.to_bytes()
                    .into_copy_iter()
                    .chain(partial_token)
                    .collect();
                return TestResult::from_bool(
                    Header::parse(&buf) == Err(Error {
                        remaining: remaining,
                        part: Part::Token(token_size),
                    }));
            }
        }
        TestResult::discard()
    }}

    quickcheck_test! {
    none_of_id_size(token: Vec<u8>; bool) {
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 0,
            part: Part::IdSize,
        })
    }}

    quickcheck_test! {
    one_of_id_size(token: Vec<u8>, partial_id_size: u8; bool) {
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain([partial_id_size].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 1,
            part: Part::IdSize,
        })
    }}

    quickcheck_test! {
    two_of_id_size(token: Vec<u8>, partial_id_size: (u8, u8); bool) {
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain([partial_id_size.0, partial_id_size.1].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 2,
            part: Part::IdSize,
        })
    }}

    quickcheck_test! {
    three_of_id_size(token: Vec<u8>, partial_id_size: (u8, u8, u8); bool) {
        let (a, b, c) = partial_id_size;
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain([a, b, c].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 3,
            part: Part::IdSize,
        })
    }}

    quickcheck_test! {
    partial_id(token: Vec<u8>, partial_id: Vec<u8>, needed: u32; TestResult) {
        let remaining = partial_id.len() as u32;
        if needed > 0 {
            if let Some(id_size) = remaining.checked_add(needed) {
                let buf: Vec<_> = (token.len() as u32)
                    .to_bytes()
                    .into_copy_iter()
                    .chain(token)
                    .chain((id_size as u32).to_bytes().into_copy_iter())
                    .chain(partial_id)
                    .collect();
                return TestResult::from_bool(
                    Header::parse(&buf) == Err(Error {
                        remaining: remaining,
                        part: Part::Id(id_size),
                    }));
            }
        }
        TestResult::discard()
    }}

    quickcheck_test! {
    none_of_timestamp(token: Vec<u8>, id: Vec<u8>; bool) {
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id)
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 0,
            part: Part::Timestamp,
        })
    }}

    quickcheck_test! {
    one_of_timestamp(token: Vec<u8>, id: Vec<u8>, partial_timestamp: u8; bool) {
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id)
            .chain([partial_timestamp].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 1,
            part: Part::Timestamp,
        })
    }}

    quickcheck_test! {
    two_of_timestamp(token: Vec<u8>, id: Vec<u8>, partial_timestamp: (u8, u8);
                     bool) {
        let (a, b) = partial_timestamp;
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id)
            .chain([a, b].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 2,
            part: Part::Timestamp,
        })
    }}

    quickcheck_test! {
    three_of_timestamp(token: Vec<u8>, id: Vec<u8>,
                       partial_timestamp: (u8, u8, u8);
                       bool) {
        let (a, b, c) = partial_timestamp;
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id)
            .chain([a, b, c].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 3,
            part: Part::Timestamp,
        })
    }}

    quickcheck_test! {
    four_of_timestamp(token: Vec<u8>, id: Vec<u8>,
                      partial_timestamp: (u8, u8, u8, u8);
                      bool) {
        let (a, b, c, d) = partial_timestamp;
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id)
            .chain([a, b, c, d].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 4,
            part: Part::Timestamp,
        })
    }}

    quickcheck_test! {
    five_of_timestamp(token: Vec<u8>, id: Vec<u8>,
                      partial_timestamp: (u8, u8, u8, u8, u8);
                      bool) {
        let (a, b, c, d, e) = partial_timestamp;
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id)
            .chain([a, b, c, d, e].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 5,
            part: Part::Timestamp,
        })
    }}

    quickcheck_test! {
    six_of_timestamp(token: Vec<u8>, id: Vec<u8>,
                     partial_timestamp: (u8, u8, u8, u8, u8, u8);
                     bool) {
        let (a, b, c, d, e, f) = partial_timestamp;
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id)
            .chain([a, b, c, d, e, f].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 6,
            part: Part::Timestamp,
        })
    }}

    quickcheck_test! {
    seven_of_timestamp(token: Vec<u8>, id: Vec<u8>,
                       partial_timestamp: (u8, u8, u8, u8, u8, u8, u8);
                       bool) {
        let (a, b, c, d, e, f, g) = partial_timestamp;
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token)
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id)
            .chain([a, b, c, d, e, f, g].into_copy_iter())
            .collect();
        Header::parse(&buf) == Err(Error {
            remaining: 7,
            part: Part::Timestamp,
        })
    }}

    quickcheck_test! {
    ok_header(token: Vec<u8>, id: Vec<u8>, timestamp: u64, payload: Vec<u8>;
              bool) {
        let buf: Vec<_> = (token.len() as u32)
            .to_bytes()
            .into_copy_iter()
            .chain(token.into_copy_iter())
            .chain((id.len() as u32).to_bytes().into_copy_iter())
            .chain(id.into_copy_iter())
            .chain(timestamp.to_bytes().into_copy_iter())
            .chain(payload.into_copy_iter())
            .collect();
        let header = Header {
            token: &token,
            id: &id,
            timestamp: Duration::from_millis(timestamp),
        };
        Header::parse(&buf) == Ok((header, &payload))
    }}
}
