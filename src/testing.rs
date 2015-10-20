use byteorder::{BigEndian, ByteOrder};
pub use quickcheck::*;

pub trait ToBytes {
    type Bytes;
    fn to_bytes(self) -> Self::Bytes;
}

impl ToBytes for u32 {
    type Bytes = [u8; 4];
    fn to_bytes(self) -> Self::Bytes {
        let mut bytes = [0_u8; 4];
        BigEndian::write_u32(&mut bytes, self);
        bytes
    }
}

impl ToBytes for u64 {
    type Bytes = [u8; 8];
    fn to_bytes(self) -> Self::Bytes {
        let mut bytes = [0_u8; 8];
        BigEndian::write_u64(&mut bytes, self);
        bytes
    }
}

pub trait IntoCopyIterator: IntoIterator {
    fn into_copy_iter(self) -> ::std::iter::Cloned<Self::IntoIter>;
}

impl<'a, T: 'a + Copy, I: IntoIterator<Item=&'a T>> IntoCopyIterator for I {
    fn into_copy_iter(self) -> ::std::iter::Cloned<Self::IntoIter> {
        self.into_iter().cloned()
    }
}

#[macro_export]
macro_rules! quickcheck_test {
    ($test_name:ident ($($param_name:ident: $param_type:ty),+; $return_type:ty)
     $body:block) => {
        #[test]
        fn $test_name() {
            fn $test_name($($param_name: $param_type),+) -> $return_type
                $body
            ::quickcheck::quickcheck(
                $test_name as fn($($param_type),+) -> $return_type);
        }
    };
}

#[macro_export]
macro_rules! assert_match {
    ($p:pat, $e:expr) => {
        match $e {
            $p => {},
            bad => panic!("assertion failed: expected {}; got {:?}", stringify!($p), bad),
        }
    };
    ($p:pat if $c:expr, $e:expr) => {
        match $e {
            $p if $c => {},
            bad => panic!("assertion failed: expected {} if {}; got {:?}",
                          stringify!($p),
                          stringify!($c),
                          bad),
        }
    };
}

#[macro_export]
macro_rules! test_result_match {
    ($p:pat, $e:expr) => {
        match $e {
            $p => ::quickcheck::TestResult::passed(),
            bad => ::quickcheck::TestResult::error(
                format!("expected {}; got {:?}", stringify!($p), bad)),
        }
    };
    ($p:pat if $c:expr, $e:expr) => {
        match $e {
            $p if $c => ::quickcheck::TestResult::passed(),
            bad => ::quickcheck::TestResult::error(
                format!("expected {} if {}; got {:?}",
                        stringify!($p), stringify!($c), bad)),
        }
    }
}
