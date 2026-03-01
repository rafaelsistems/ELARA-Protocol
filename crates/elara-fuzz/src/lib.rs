//! Fuzzing infrastructure for ELARA Protocol
//!
//! This crate provides a trait-based framework for creating fuzz targets
//! that can discover edge cases, panics, and security vulnerabilities in
//! parsing and cryptographic code.
//!
//! # Architecture
//!
//! The fuzzing infrastructure consists of:
//! - **FuzzTarget trait**: Core abstraction for fuzz targets
//! - **FuzzResult enum**: Classification of fuzz outcomes
//! - **Concrete fuzzers**: Pre-built fuzzers for wire protocol, crypto, and state
//!
//! # Example
//!
//! ```rust
//! use elara_fuzz::{FuzzTarget, FuzzResult};
//! use arbitrary::Arbitrary;
//!
//! #[derive(Arbitrary, Debug)]
//! struct MyInput {
//!     data: Vec<u8>,
//! }
//!
//! struct MyFuzzer;
//!
//! impl FuzzTarget for MyFuzzer {
//!     type Input = MyInput;
//!
//!     fn fuzz_once(&mut self, input: Self::Input) -> FuzzResult {
//!         // Test your code with arbitrary input
//!         match process_data(&input.data) {
//!             Ok(_) => FuzzResult::Ok,
//!             Err(e) if e.is_expected() => FuzzResult::Invalid,
//!             Err(e) => FuzzResult::Bug(format!("Unexpected error: {}", e)),
//!         }
//!     }
//! }
//! # fn process_data(_data: &[u8]) -> Result<(), std::io::Error> { Ok(()) }
//! # trait ErrorExt { fn is_expected(&self) -> bool; }
//! # impl ErrorExt for std::io::Error { fn is_expected(&self) -> bool { true } }
//! ```
//!
//! # Integration with cargo-fuzz
//!
//! To use with cargo-fuzz, create fuzz targets in `fuzz/fuzz_targets/`:
//!
//! ```rust,no_run
//! #![no_main]
//! use libfuzzer_sys::fuzz_target;
//! use elara_fuzz::FuzzTarget;
//!
//! # struct MyFuzzer;
//! # impl elara_fuzz::FuzzTarget for MyFuzzer {
//! #     type Input = Vec<u8>;
//! #     fn fuzz_once(&mut self, _input: Self::Input) -> elara_fuzz::FuzzResult {
//! #         elara_fuzz::FuzzResult::Ok
//! #     }
//! # }
//! fuzz_target!(|data: <MyFuzzer as FuzzTarget>::Input| {
//!     let mut fuzzer = MyFuzzer;
//!     let _ = fuzzer.fuzz_once(data);
//! });
//! ```

use arbitrary::Arbitrary;

/// Result of a single fuzz iteration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzResult {
    /// Test passed - input was processed successfully
    Ok,
    
    /// Found a bug - panic, assertion failure, or unexpected error
    Bug(String),
    
    /// Invalid input - expected rejection, not a bug
    Invalid,
}

/// Trait for implementing fuzz targets
///
/// Implementors define an `Input` type that can be generated arbitrarily
/// and a `fuzz_once` method that tests the code with that input.
pub trait FuzzTarget {
    /// Input type for fuzzing - must implement `arbitrary::Arbitrary`
    /// so the fuzzer can generate random instances
    type Input: Arbitrary<'static>;
    
    /// Execute one fuzz iteration with the given input
    ///
    /// This method should:
    /// - Test the target code with the input
    /// - Return `FuzzResult::Ok` if processing succeeds
    /// - Return `FuzzResult::Invalid` if input is malformed (expected)
    /// - Return `FuzzResult::Bug` if an unexpected error occurs
    ///
    /// The method should catch panics and convert them to `FuzzResult::Bug`.
    fn fuzz_once(&mut self, input: Self::Input) -> FuzzResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Arbitrary, Debug)]
    struct TestInput {
        value: u8,
    }

    struct TestFuzzer;

    impl FuzzTarget for TestFuzzer {
        type Input = TestInput;

        fn fuzz_once(&mut self, input: Self::Input) -> FuzzResult {
            if input.value == 0 {
                FuzzResult::Invalid
            } else if input.value == 255 {
                FuzzResult::Bug("Found edge case".to_string())
            } else {
                FuzzResult::Ok
            }
        }
    }

    #[test]
    fn test_fuzz_result_ok() {
        let mut fuzzer = TestFuzzer;
        let result = fuzzer.fuzz_once(TestInput { value: 42 });
        assert_eq!(result, FuzzResult::Ok);
    }

    #[test]
    fn test_fuzz_result_invalid() {
        let mut fuzzer = TestFuzzer;
        let result = fuzzer.fuzz_once(TestInput { value: 0 });
        assert_eq!(result, FuzzResult::Invalid);
    }

    #[test]
    fn test_fuzz_result_bug() {
        let mut fuzzer = TestFuzzer;
        let result = fuzzer.fuzz_once(TestInput { value: 255 });
        assert_eq!(result, FuzzResult::Bug("Found edge case".to_string()));
    }
}
