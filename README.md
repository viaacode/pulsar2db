# pulsar2db

Subscribe to Pulsar topics and build final state (or a materialized view) for a
certain object or key.

## Prerequisites

- Rust toolchain: see [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).
- Cargo (should be installed along with the Rust toolchain)

## Usage

- Clone this repository.
- Fill in and source an `.env` file (Example in `.env.example`):
  ```bash
  $ export $(grep -v '^#' .env | xargs)
  ```
- Run with `cargo run`.
