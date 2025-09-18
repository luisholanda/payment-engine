# Payment Engine CLI

This is a small toy project implementing a simple in-memory transaction processing
engine in Rust. It supports basic operations like deposits, withdrawals, disputes,
resolves, and chargebacks.

A small CLI is provided to read transactions from a CSV file and output the resulting
client account states.

It serves as a complex enough project to play around with `proptest` for property-based
testing of stateful structures.

## Features

The engine supports the following transaction types:

- **Deposit**: Adds funds to a client's available balance.
- **Withdrawal**: Removes funds from a client's available balance if sufficient funds exist.
- **Dispute**: Flags a transaction as disputed, moving the amount from available to held funds.
  - For now, only deposits can be disputed. Withdrawl disputes are ignored as bad transactions.
- **Resolve**: Resolves a dispute, moving the held funds back to available.
- **Chargeback**: Finalizes a dispute, removing held funds and locking the account.

"Bad" transactions (e.g., duplicate transaction IDs, insufficient funds, disputes on
non-existent transactions) are ignored, and processing continues.

## Structure

The main public API is the `Engine` struct, which manages client accounts and processes
transactions.

Each client has an associated `Account` struct that tracks available, held, and total
funds, as well as whether the account is locked. Throughout the code, we use a fixed-point
decimal representation for monetary values to avoid floating-point precision issues,
provided by the `fastnum` crate, this is "abstracted" via the `Amount` type alias.

Transactions themselves are represented by the `Transaction` type, which includes the
transaction type, client ID, transaction ID, and amount (if applicable). For now, the
only way to create transactions is by deserializing them from some input via `serde`.

The transaction processing logic is encapsulated in `Client`, which, in addition to
`Account`, also maintains a history of transactions for dispute handling.

### Performance

The engine explores the fact that clients' IDs are `u16` to use a `Vec<Client>` indexed
by the client ID, providing O(1) access to each client's data. To differentiate between
existing and non-existing clients, the engine uses a bitset to track which client IDs are
in use. This makes `Engine` use significantly more memory (15MB without any transactions),
but it also makes accessing the client data extremely fast.

This makes `Engine` structure a little more complex, but given that it is responsible
only for routing transactions to the appropriate client, it is a reasonable trade-off.
Note that, in a real-world scenario, is expected that most IDs will be used, so the memory
usage overhead will be insignificant.

As another simple optimization, deserializing transactions tries to avoid allocating
strings for the transaction type by using `serde`'s `borrow` feature.

The CLI itself streams transactions from the CSV file as they are read, avoiding loading
the entire file into memory. The output is also streamed to stdout as each client's data
is written.

### Concurrency

The code is single-threaded implementation. This is mainly to keep things simple
and also because processing a single transaction is extremely fast (a full CLI run with
a generated large scale sample with 10M transactions runs in 2 seconds in a M3 MBP).

In cases where multiple transaction streams need to be processed concurrently, one could
use a MPSC queue to feed the engine from multiple threads/tasks. If for an extremely high
load cases where this isn't enough, one could shard the clients across multiple engine
instances, each running in its own thread.

## Testing

The project includes a suite of unit tests covering various scenarios and edge cases. It
also uses `proptest` for property-based testing to ensure robustness. In addition to that,
there are several sample CSV files in the `samples` directory that can be used to manually
test the CLI. The script `test_samples.sh` runs the CLI against all sample files and compares
the output to the expected results.

Unit and property-based tests can be run with `cargo test`. The test samples can be executed
running `./test_samples.sh`.

Only small samples are included in the repository. To generate larger test samples,
you can use the `samples/generate_large_sample.py` script. It creates the expected
CSV files in `samples/large_scale`.
