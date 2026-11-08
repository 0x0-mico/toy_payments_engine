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

- Every module is unit tested: balance guards and fund movement in
  `client`, the dispute state machine in `transaction`, tx-id uniqueness
  and account lookup in `storage`, row and amount validation in `event`,
  and event application in `engine`.
- The 39 integration cases run the compiled binary and compare stdout,
  stderr and exit status against hand-written expectations.
- The exit-status contract has its own cases, including usage errors.
- The 1.36M-row stress dataset embeds every failure mode of the smaller
  cases and is verified against a closed form.
- A rejected event changes no account, stores no transaction, and never
  stops the stream; multi-step events leave no half-applied state.
- The invariant `total = available + held` holds on every emitted row and
  is asserted on every row of the stress dataset.
- Results were cross-checked against an independent re-implementation of
  the specification: the sample from the specification and the stress
  dataset agree exactly.

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

## Maintainability

Modules mirror the specification entities:

- `event` (wire row to typed event)
- `transaction` (transaction record and dispute state machine)
- `client` (balance accounting)
- `storage` (account and transaction maps)
- `engine` (event application)
- `main` (CLI and stream loop)
