# pulsar2db

Subscribe to Pulsar topics and build final state (or a materialized view) for a
certain object or key.

Since a state building application is, in effect, a tight coupling between it's
input (CloudEvents in Pulsar, in this case) and it's output (a Postgres table),
a SQL DDL file for the target table is included.

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
