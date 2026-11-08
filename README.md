# Toy Payments Engine

A toy payments engine in Rust. It reads a CSV stream of transaction events,
keeps accounting for client accounts, and writes the final state of every
account as CSV to stdout.

## How to use

```sh
cargo build --release
cargo run -- transactions.csv > accounts.csv
```

The input file is the first and only argument. Missing or extra arguments
are rejected with a usage message.

Whitespace around fields is tolerated, amounts are decimals with up to
four places.

Input columns: `type,client,tx,amount`
Output columns: `client,available,held,total,locked`

Output contains one row per client, ordered by client ID. Invalid rows are
reported to stderr with line number and reason and do not stop the stream.

Exit status: 0 on success, 1 on fatal IO.

## How to test

```sh
cargo test                                 # unit + integration + exit codes
cargo test --lib                           # unit tests inside src/
cargo test --test cli                      # 39 directory-driven CLI cases
cargo test --test exit_codes               # exit-status contract
cargo test --test stress -- --ignored      # 1.36M-row stress case (needs python3)
```

`TEST_SPEC.md` lists what every case proves.

## Assumptions and decisions

- Only deposits can be disputed: the specification defines the fund movement
  for deposit disputes only.
- A transaction can be disputed again after being resolved.
- Disputing funds that were already withdrawn drives `available` negative;
  after the chargeback `total` is negative. `total = available + held` holds
  throughout.
- A withdrawal for an unknown client creates a zeroed account and is denied.
- Locked accounts deny deposits and withdrawals; dispute, resolve and
  chargeback accounting still applies.
- Zero, negative and more-than-4-decimal amounts, duplicate transaction IDs,
  and events referencing another client's transaction are rejected and
  logged.
- Unknown event types and malformed rows are rejected, never fatal.
- The amount field on dispute, resolve and chargeback rows is ignored.
- Rows are processed in file order; the input is assumed chronological.
- Output rows are ordered by client ID; the decimal scale follows the input.

## Completeness

All five event types are implemented: deposit, withdrawal, dispute, resolve,
chargeback.

The full dispute lifecycle: open, reopen after resolve, resolve, chargeback
as the terminal state.

Locked accounts deny deposits and withdrawals; dispute, resolve and
chargeback accounting still applies.

## Correctness

- Every module has unit tests for the complicated bits: balance guards
  and fund movement, the dispute state machine, transaction-ID
  uniqueness, row and amount validation, and event application.
- The type system carries part of the load: a typed `Event` enum matched
  exhaustively, `Decimal` instead of floats, typed errors, and client's
  `total` balance amount derived rather than stored.
- The 39 integration cases run the compiled program end to end against
  expectations written from the specification by hand, including the
  sample from the specification itself. A failure always means the code
  or the expectation is wrong; an expectation can never copy a bug from
  the program.
- The exit-status (0 on success, 1 on fatal IO, 2 on a bad
  invocation) has its own test cases.
- The 1.36M-row stress dataset covers every failure mode of the smaller
  cases, and its expected result is computed without the engine, so the
  check cannot repeat the engine's own bugs.
- A rejected event changes no account, stores no transaction, and never
  stops the stream; a multi-step event applies fully or not at all. The
  rule `total = available + held` holds on every emitted row.
- The results were also checked against a separate program written from
  the specification alone: the sample from the specification and the
  stress dataset agree exactly.

## Safety and robustness

- No `unwrap` or `panic` outside tests; errors are typed with `thiserror`
  and propagated.
- A rejected row is logged to stderr (`LN <row_line>: <reason>`), changes no
  account, stores no transaction, and never stops the stream.
- Only fatal IO (unreadable input, unwritable output) exits with status 1.

## Efficiency

- Input is streamed row by row; the file is never fully loaded.
- Accounts and transactions are keyed by `HashMap<u16, _>` and
  `HashMap<u32, _>`: integer keys, no string hashing.
- Measured on this machine (AMD Ryzen 7 PRO): the 1.36M-row stress dataset
  (29.8 MB, every failure mode included) processes in 2.8 s with a peak RSS
  of 24.6 MiB (release build; debug: 6.3 s, 25.7 MiB). Generator:
  `scripts/generate_stress.py`.
- Bundling on a server: event streams would be independent as no event crosses
  clients, so the engine scales by instantiation. One engine and store per
  stream, or sharded by client ID with no shared state to lock. A single
  shared store would need sharding by client ID to avoid serializing on one
  lock.

## Maintainability

Modules mirror the specification entities:

- `event` (wire row to typed event)
- `transaction` (transaction record and dispute state machine)
- `client` (balance accounting)
- `storage` (account and transaction maps)
- `engine` (event application)
- `main` (CLI and stream loop)
