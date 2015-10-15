use std::collections::HashSet;
use std::iter::Fuse;

use super::{server, Message, Server, Stream};

pub struct Session<'a, S: 'a, I> {
    server: &'a mut S,
    messages: Fuse<I>,
    ids: HashSet<Vec<u8>>,
}

impl<'a, 'b, T: 'a + 'b, S: 'a + Server<T>, I: 'a + Iterator<Item=Message<'b, T>>>
    Iterator for Session<'a, S, I> {
    type Item = server::ConsumeResult<
        S::AuthErr, <S::Stream as Stream<T>>::PushErr>;
    fn next(&mut self) -> Option<Self::Item> {
        self.messages.next().map(|msg| {
            let id = msg.header.id;
            let result = self.server.consume(msg);
            // Check for existence first before insertion to avoid unnecessary allocation.
            if result.is_ok() && !self.ids.contains(id) {
                self.ids.insert(id.to_owned());
            }
            result
        })
    }
}

impl<'a, 'b, T: 'a + 'b, S: 'a + Server<T>, I: 'a + Iterator<Item=Message<'b, T>>>
    Session<'a, S, I> {
    pub fn new(server: &'a mut S, messages: I) -> Self {
        Session {
            server: server,
            messages: messages.fuse(),
            ids: HashSet::with_capacity(1),
        }
    }

    pub fn ids_to_extract(mut self) -> HashSet<Vec<u8>> {
        // I need a Fuse iterator to ensure the user can safely call this method
        // even after having finished iterating. So this should be safe:
        // ```
        // for result in &mut session {
        //    // do stuff with result
        // }
        // let ids = session.ids_to_extract();
        // ```
        for _ in &mut self {
        }
        self.ids
    }
}

#[cfg(test)]
mod tests {
    use std::iter;
    use std::collections::HashSet;
    use std::time::Duration;

    use super::*;
    use super::super::{message, server, stream, Message};

    #[test]
    fn next_none() {
        let empty = iter::empty::<Message<'static, &'static [u8]>>();
        assert!(Session::new(&mut server::mocks::CannotAuth, empty).next().is_none());
    }

    quickcheck_test! {
    next_some_err(token: Vec<u8>, id: Vec<u8>, millis: u64, payload: Vec<u8>;
                  bool) {
        let msg = Message {
            header: message::Header {
                token: &token,
                id: &id,
                timestamp: Duration::from_millis(millis),
            },
            payload: &*payload,
        };
        matches!(
            Session::new(&mut server::mocks::CannotAuth, iter::once(msg)).next(),
            Some(Err(_)))
    }}

    quickcheck_test! {
    next_some_ok(token: Vec<u8>, id: Vec<u8>, millis: u64, payload: Vec<u8>;
                 bool) {
        let msg = Message {
            header: message::Header {
                token: &token,
                id: &id,
                timestamp: Duration::from_millis(millis),
            },
            payload: &*payload,
        };
        let mut finder = server::Finder::new();
        finder.insert(id.clone(), stream::mocks::Ok);
        matches!(Session::new(&mut server::mocks::Ok(finder), iter::once(msg)).next(),
                 Some(Ok(_)))
    }}

    quickcheck_test! {
    ids_to_extract(script: Vec<((Vec<u8>, Vec<u8>, u64, Vec<u8>), bool)>; bool) {
        let present_ids: HashSet<_> = script.iter()
            .filter(|command| command.1)
            .map(|command| (command.0).1.clone())
            .collect();
        let finder: server::Finder<_> = present_ids.iter()
            .map(|id| (id.clone(), stream::mocks::Ok))
            .collect();
        let mut server = server::mocks::Ok(finder);
        let messages = script.iter()
            .map(|command| Message {
                header: message::Header {
                    token: &(command.0).0,
                    id: &(command.0).1,
                    timestamp: Duration::from_millis((command.0).2),
                },
                payload: &*(command.0).3,
            });

        let mut session = Session::new(&mut server, messages);
        for _ in &mut session { }

        session.ids_to_extract() == present_ids
    }}
}
