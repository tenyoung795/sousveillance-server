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

#[cfg(test)]
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

#[cfg(test)]
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
