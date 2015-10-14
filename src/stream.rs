use std::collections::HashMap;
use std::time::Duration;

pub trait Stream<T> : Default {
    type PushErr;
    fn push(&mut self, Duration, T) -> Result<(), Self::PushErr>;

    type Extract;
    type ExtractErr;
    fn extract(self) -> Result<Self::Extract, (Self, Self::ExtractErr)>;
}

pub trait Finder<T> {
    type Stream: Stream<T>;
    fn extract(&mut self, &[u8])
        -> Option<Result<<Self::Stream as Stream<T>>::Extract,
                         <Self::Stream as Stream<T>>::ExtractErr>>;
}

impl<T, V: Stream<T>> Finder<T> for HashMap<Vec<u8>, V> {
    type Stream = V;
    fn extract(&mut self, key: &[u8])
        -> Option<Result<<Self::Stream as Stream<T>>::Extract,
                         <Self::Stream as Stream<T>>::ExtractErr>> {
        self.remove(key)
            .map(V::extract)
            .map(|result| result.map_err(|(stream, err)| {
                self.insert(key.to_owned(), stream);
                err
            }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::time::Duration;

    use ::quickcheck::*;

    use super::*;

    #[derive(Default)]
    struct S;
    impl Stream<::Void> for S {
        type PushErr = ::Void;
        fn push(&mut self, _: Duration, _: ::Void) -> Result<(), Self::PushErr> {
            Ok(())
        }

        type Extract = ();
        type ExtractErr = ::Void;
        fn extract(self) -> Result<Self::Extract, (Self, Self::ExtractErr)> {
            Ok(())
        }
    }

    quickcheck_test! {
    missing_id(ids: HashSet<Vec<u8>>; TestResult) {
        let mut iter = ids.into_iter();
        if let Some(missing_id) = iter.next() {
            let mut streams: HashMap<_, _> = iter.map(|id| (id, S)).collect();
            TestResult::from_bool(streams.extract(&missing_id).is_none())
        } else {
            TestResult::discard()
        }
    }}

    quickcheck_test! {
    extract_error(id_to_lookup: Vec<u8>, other_ids: HashSet<Vec<u8>>; bool) {
        let mut ids = other_ids;
        ids.insert(id_to_lookup.clone());

        #[derive(Default)]
        struct S;
        impl Stream<::Void> for S {
            type PushErr = ::Void;
            fn push(&mut self, _: Duration, _: ::Void)
                -> Result<(), Self::PushErr> {
                Ok(())
            }

            type Extract = ::Void;
            type ExtractErr = ();
            fn extract(self) -> Result<Self::Extract, (Self, Self::ExtractErr)> {
                Err((self, ()))
            }
        }
        let mut streams: HashMap<_, _> = ids.into_iter().map(|id| (id, S)).collect();
        matches!(streams.extract(&id_to_lookup), Some(Err(_)))
            && matches!(streams.get(&id_to_lookup), Some(&S))
    }}

    quickcheck_test! {
    extract_ok(id_to_lookup: Vec<u8>, other_ids: HashSet<Vec<u8>>; bool) {
        let mut ids = other_ids;
        ids.insert(id_to_lookup.clone());

        let mut streams: HashMap<_, _> = ids.into_iter().map(|id| (id, S)).collect();
        matches!(streams.extract(&id_to_lookup), Some(Ok(_)))
            && streams.get(&id_to_lookup).is_none()
    }}
}
