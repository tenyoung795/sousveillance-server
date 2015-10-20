pub use self::header::Header;
pub use self::header::Error;

pub mod header;

#[derive(Debug, PartialEq, Eq)]
pub struct Message<'a> {
    pub header: Header<'a>,
    pub payload: &'a [u8],
}

impl<'a> Message<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        let (header, payload) = try!(Header::parse(bytes));
        Ok(Message {
            header: header,
            payload: payload,
        })
    }
}
