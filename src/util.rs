#[derive(Debug, PartialEq, Eq)]
pub enum Void { }

#[cfg(test)]
pub mod testing {
    use byteorder::{BigEndian, ByteOrder};

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
}
