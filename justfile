py := "python/refix-engine"

# List available recipes
default:
    just --list

# Install or refresh the Python dev environment
sync:
    uv sync --project {{py}}

# Compile and install the extension module into the venv
develop:
    uv run --project {{py}} maturin develop --uv

# Run the Python test suite
pytest *args:
    uv run --project {{py}} pytest {{args}}

# Run the Rust test suite
cargo-test:
    cargo test

# Run all tests
test: cargo-test pytest

# Format Rust code
fmt:
    cargo fmt --all

# Check formatting and lints without modifying anything
lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets

# Build a release wheel
wheel:
    uv run --project {{py}} maturin build --release