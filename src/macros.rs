#[macro_export]
macro_rules! matches {
    ($e:expr, $p:pat) => {
        if let $p = $e {
            true
        } else {
            false
        }
    };
}

#[cfg(test)]
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
