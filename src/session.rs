use std::collections::HashSet;
use std::iter::Fuse;

use super::{server, Message, Server, Stream};

pub struct Session<'a, S: 'a, I: 'a> {
    server: &'a mut S,
    messages: Fuse<I>,
    ids: HashSet<Vec<u8>>,
}

impl<'a, T: 'a, S: 'a + Server<T>, I: 'a + for<'b> Iterator<Item=Message<'b, T>>>
    Iterator for Session<'a, S, I> {
    type Item = server::ConsumeResult<
        S::AuthErr, <S::Stream as Stream<T>>::PushErr>;
    fn next(&mut self) -> Option<Self::Item> {
        self.messages.next()
            .map(|msg| {
                let id = msg.header.id;
                let result = self.server.consume(msg);
                if result.is_ok() {
                    self.ids.insert(id.to_owned());
                }
                result
            })
    }
}

impl<'a, T: 'a, S: 'a + Server<T>, I: 'a + for<'b> Iterator<Item=Message<'b, T>>>
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
