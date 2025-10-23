// Integration test module
// This file is the entry point for integration tests
// Cargo will compile this as a separate binary for testing

#[cfg(feature = "messaging")]
mod integration {
    mod test_helpers;
    mod nats_basic_test;
    mod read_methods_test;
}

