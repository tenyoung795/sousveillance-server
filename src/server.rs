use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::error;

use super::{Stream, Message};

#[derive(Debug)]
pub enum AuthError<E> {
    InvalidToken,
    Other(E),
}

impl<E> From<E> for AuthError<E> {
    fn from(e: E) -> Self {
        AuthError::Other(e)
    }
}

impl<E: Display> Display for AuthError<E> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            AuthError::InvalidToken => f.write_str("invalid token"),
            AuthError::Other(ref e) => e.fmt(f),
        }
    }
}

impl<E: error::Error> error::Error for AuthError<E> {
    fn description(&self) -> &str {
        match *self {
            AuthError::InvalidToken => "invalid token",
            AuthError::Other(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            AuthError::InvalidToken => None,
            AuthError::Other(ref e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub enum ConsumeError<A, P> {
    AuthError(AuthError<A>),
    MissingID,
    PushError(P),
}

impl<A, P> From<AuthError<A>> for ConsumeError<A, P> {
    fn from(err: AuthError<A>) -> Self {
        ConsumeError::AuthError(err)
    }
}

impl<A: Display, P: Display> Display for ConsumeError<A, P> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            ConsumeError::AuthError(ref e) => e.fmt(f),
            ConsumeError::MissingID => f.write_str("missing ID"),
            ConsumeError::PushError(ref e) => e.fmt(f),
        }
    }
}

impl<A: error::Error, P: error::Error> error::Error for ConsumeError<A, P> {
    fn description(&self) -> &str {
        match *self {
            ConsumeError::AuthError(ref e) => e.description(),
            ConsumeError::MissingID => "missing ID",
            ConsumeError::PushError(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ConsumeError::AuthError(ref e) => Some(e),
            ConsumeError::MissingID => None,
            ConsumeError::PushError(ref e) => Some(e),
        }
    }
}

pub type Finder<S> = HashMap<Vec<u8>, S>;
pub type AuthResult<'a, S, A> = Result<&'a mut Finder<S>, AuthError<A>>;
pub type ConsumeResult<A, P> = Result<(), ConsumeError<A, P>>;
pub trait Server<T> {
    type Stream: Stream<T>;

    type AuthErr;
    fn auth(&mut self, token: &[u8]) -> AuthResult<Self::Stream, Self::AuthErr>;

    fn consume(&mut self,
               msg: Message<T>)
               -> ConsumeResult<Self::AuthErr, <Self::Stream as Stream<T>>::PushErr> {
        self.auth(msg.header.token)
            .map_err(Into::into)
            .and_then(|finder| finder.get_mut(msg.header.id).ok_or(ConsumeError::MissingID))
            .and_then(move |stream| {
                stream.push(msg.header.timestamp, msg.payload).map_err(ConsumeError::PushError)
            })
    }
}

#[cfg(test)]
pub mod mocks {
    use super::*;
    use super::super::{stream, Stream};

    pub struct RefuseToAuth;
    impl<T> Server<T> for RefuseToAuth {
        type Stream = stream::mocks::Impossible;
        type AuthErr = ::Void;
        fn auth(&mut self, _: &[u8]) -> AuthResult<Self::Stream, Self::AuthErr> {
            Err(AuthError::InvalidToken)
        }
    }

    pub struct CannotAuth;
    impl<T> Server<T> for CannotAuth {
        type Stream = stream::mocks::Impossible;
        type AuthErr = ();
        fn auth(&mut self, _: &[u8]) -> AuthResult<Self::Stream, Self::AuthErr> {
            Err(AuthError::Other(()))
        }
    }

    pub struct Ok<S>(pub Finder<S>);
    impl<T, S: Stream<T>> Server<T> for Ok<S> {
        type Stream = S;
        type AuthErr = ::Void;
        fn auth(&mut self, _: &[u8]) -> AuthResult<Self::Stream, Self::AuthErr> {
            Result::Ok(&mut self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::iter;
    use std::time::Duration;

    use super::*;
    use super::super::{message, stream, Message};

    quickcheck_test! {
    missing_token(token: Vec<u8>, id: Vec<u8>, millis: u64, payload: Vec<u8>;
                  bool) {
        let msg = Message {
            header: message::Header {
                token: &token,
                id: &id,
                timestamp: Duration::from_millis(millis),
            },
            payload: &*payload,
        };
        matches!(mocks::RefuseToAuth.consume(msg),
                 Err(ConsumeError::AuthError(AuthError::InvalidToken)))
    }}

    quickcheck_test! {
    other_auth_error(token: Vec<u8>, id: Vec<u8>, millis: u64, payload: Vec<u8>;
                     bool) {
        let msg = Message {
            header: message::Header {
                token: &token,
                id: &id,
                timestamp: Duration::from_millis(millis),
            },
            payload: &*payload,
        };
        matches!(mocks::CannotAuth.consume(msg),
                 Err(ConsumeError::AuthError(AuthError::Other(_))))
    }}

    quickcheck_test! {
    missing_id(token: Vec<u8>, id: Vec<u8>, millis: u64, payload: Vec<u8>;
               bool) {
        let msg = Message {
            header: message::Header {
                token: &token,
                id: &id,
                timestamp: Duration::from_millis(millis),
            },
            payload: &*payload,
        };
        let finder: Finder<stream::mocks::Impossible> = Finder::new();
        matches!(mocks::Ok(finder).consume(msg), Err(ConsumeError::MissingID))
    }}

    quickcheck_test! {
    push_error(token: Vec<u8>, id: Vec<u8>, millis: u64, payload: Vec<u8>;
               bool) {
        let finder: Finder<_> = iter::once(
            (id.clone(), stream::mocks::Broken)).collect();
        let msg = Message {
            header: message::Header {
                token: &token,
                id: &id,
                timestamp: Duration::from_millis(millis),
            },
            payload: &*payload,
        };
        matches!(mocks::Ok(finder).consume(msg), Err(ConsumeError::PushError(_)))
    }}

    quickcheck_test! {
    ok_consume(token: Vec<u8>, id: Vec<u8>, millis: u64, payload: Vec<u8>;
               bool) {
        let finder: Finder<_> = iter::once(
            (id.clone(), stream::mocks::Ok)).collect();
        let msg = Message {
            header: message::Header {
                token: &token,
                id: &id,
                timestamp: Duration::from_millis(millis),
            },
            payload: &*payload,
        };
        mocks::Ok(finder).consume(msg).is_ok()
    }}
}
