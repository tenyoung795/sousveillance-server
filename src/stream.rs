use std::collections::HashMap;
use std::time::Duration;

pub trait Stream<T> : Default {
    type PushErr;
    fn push(&mut self, Duration, T) -> Result<(), Self::PushErr>;

    type Extract;
    type ExtractErr;
    fn extract(self) -> Result<Self::Extract, (Self, Self::ExtractErr)>;
}

pub type FoundResult<T, S> = Result<
    <S as Stream<T>>::Extract, <S as Stream<T>>::ExtractErr>;
pub trait Finder<T> {
    type Stream: Stream<T>;
    fn extract(&mut self, &[u8]) -> Option<FoundResult<T, Self::Stream>>;
}

impl<T, V: Stream<T>> Finder<T> for HashMap<Vec<u8>, V> {
    type Stream = V;
    fn extract(&mut self, key: &[u8]) -> Option<FoundResult<T, Self::Stream>> {
        self.remove(key)
            .map(V::extract)
            .map(|result| {
                result.map_err(|(stream, err)| {
                    self.insert(key.to_owned(), stream);
                    err
                })
            })
    }
}

#[cfg(test)]
pub mod mocks {
    use std::time::Duration;

    use super::*;

    #[allow(dead_code)]
    pub enum Impossible { }
    impl Default for Impossible {
        fn default() -> Self {
            unreachable!()
        }
    }
    impl<T> Stream<T> for Impossible {
        type PushErr = ::Void;
        fn push(&mut self, _: Duration, _: T) -> Result<(), Self::PushErr> {
            match *self { }
        }

        type Extract = ::Void;
        type ExtractErr = ::Void;
        fn extract(self) -> Result<Self::Extract, (Self, Self::ExtractErr)> {
            match self { }
        }
    }

    #[derive(Default)]
    pub struct Broken;
    impl<T> Stream<T> for Broken {
        type PushErr = ();
        fn push(&mut self, _: Duration, _: T) -> Result<(), Self::PushErr> {
            Err(())
        }

        type Extract = ::Void;
        type ExtractErr = ();
        fn extract(self) -> Result<Self::Extract, (Self, Self::ExtractErr)> {
            Err((self, ()))
        }
    }

    #[derive(Default)]
    pub struct Ok;
    impl<T> Stream<T> for Ok {
        type PushErr = ::Void;
        fn push(&mut self, _: Duration, _: T) -> Result<(), Self::PushErr> {
            Result::Ok(())
        }

        type Extract = ();
        type ExtractErr = ::Void;
        fn extract(self) -> Result<Self::Extract, (Self, Self::ExtractErr)> {
            Result::Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use ::quickcheck::*;

    use super::*;

    quickcheck_test! {
    missing_id(missing_id: Vec<u8>, present_ids: HashSet<Vec<u8>>; TestResult) {
        if present_ids.contains(&missing_id) {
            TestResult::discard()
        } else {
            let mut streams: HashMap<_, _> = present_ids.into_iter()
                .map(|id| (id, mocks::Ok))
                .collect();
            let finder = &mut streams as &mut Finder<::Void, Stream=mocks::Ok>;
            TestResult::from_bool(finder.extract(&missing_id).is_none())
        }
    }}

    quickcheck_test! {
    extract_error(id_to_lookup: Vec<u8>, other_ids: HashSet<Vec<u8>>; bool) {
        let mut ids = other_ids;
        ids.insert(id_to_lookup.clone());

        let mut streams: HashMap<_, _> = ids.into_iter()
            .map(|id| (id, mocks::Broken))
            .collect();
        let finder = &mut streams as &mut Finder<::Void, Stream=mocks::Broken>;
        matches!(finder.extract(&id_to_lookup), Some(Err(_)))
    }}

    quickcheck_test! {
    extract_error_reinserts(id_to_lookup: Vec<u8>, other_ids: HashSet<Vec<u8>>;
                            bool) {
        let mut ids = other_ids;
        ids.insert(id_to_lookup.clone());

        let mut streams: HashMap<_, _> = ids.into_iter()
            .map(|id| (id, mocks::Broken))
            .collect();
        {
            let finder = &mut streams as &mut Finder<::Void, Stream=mocks::Broken>;
            finder.extract(&id_to_lookup);
        }
        matches!(streams.get(&id_to_lookup), Some(&mocks::Broken))
    }}

    quickcheck_test! {
    extract_ok(id_to_lookup: Vec<u8>, other_ids: HashSet<Vec<u8>>; bool) {
        let mut ids = other_ids;
        ids.insert(id_to_lookup.clone());

        let mut streams: HashMap<_, _> = ids.into_iter()
            .map(|id| (id, mocks::Ok))
            .collect();
        let finder = &mut streams as &mut Finder<::Void, Stream=mocks::Ok>;
        matches!(finder.extract(&id_to_lookup), Some(Ok(_)))
    }}

    quickcheck_test! {
    extract_ok_removes(id_to_lookup: Vec<u8>, other_ids: HashSet<Vec<u8>>; bool) {
        let mut ids = other_ids;
        ids.insert(id_to_lookup.clone());

        let mut streams: HashMap<_, _> = ids.into_iter()
            .map(|id| (id, mocks::Ok))
            .collect();
        {
            let finder = &mut streams as &mut Finder<::Void, Stream=mocks::Ok>;
            finder.extract(&id_to_lookup);
        }
        streams.get(&id_to_lookup).is_none()
    }}
}
