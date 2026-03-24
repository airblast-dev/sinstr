all:
	just all-stable all-nightly

all-stable:
	just check-stable fmt-stable test-stable


check-stable:
	rustup component add clippy
	cargo clippy

fmt-stable:
	rustup component add rustfmt
	cargo fmt

test-stable:
	cargo test

all-nightly:
	just check-nightly fmt-nightly test-nightly fuzz-nightly

check-nightly:
	rustup toolchain install nightly --component clippy
	cargo +nightly clippy

fmt-nightly:
	rustup toolchain install nightly --component rustfmt
	cargo +nightly fmt

[env("MIRIFLAGS", "-Zmiri-permissive-provenance")]
[env("RUSTFLAGS", "-C target-feature=-crt-static")]
test-nightly:
	rustup toolchain install nightly --component miri
	cargo +nightly miri test

fuzz-nightly:
	cargo install cargo-fuzz
	cargo +nightly fuzz run collections -- -max_total_time=120
