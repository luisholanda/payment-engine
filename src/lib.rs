use bit_set::BitSet;

use crate::{account::Account, client::Client};

mod account;
mod client;
mod transaction;

type Amount = fastnum::D256;

#[doc(inline)]
pub use self::transaction::Transaction;

/// Main payment engine structure.
///
/// Allows processing transactions and querying client accounts.
pub struct Engine {
    // PERF: Given that clients are indexed by u16, we can use a Vec for O(1) access
    //       and avoid the overhead of a HashMap.
    clients: Vec<Client>,
    seem_clients: BitSet<u64>,
}

impl Default for Engine {
    fn default() -> Self {
        let mut this = Self {
            clients: vec![],
            seem_clients: BitSet::default(),
        };

        this.seem_clients.reserve_len(u16::MAX as _);
        this.clients.resize_with(u16::MAX as _, Client::default);

        this
    }
}

impl Engine {
    /// Process a transaction.
    ///
    /// It will route the transaction to the appropriate client based on the client ID
    /// in the transaction.
    pub fn process_transaction(&mut self, tx: Transaction) {
        self.seem_clients.insert(tx.client as _);
        self.clients[tx.client as usize].process_transaction(tx);
    }

    /// All client accounts in the engine.
    pub fn accounts(&self) -> impl Iterator<Item = (u16, &Account)> {
        self.seem_clients
            .iter()
            .map(|client_id| (client_id as u16, self.clients[client_id].account()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_goes_to_right_client() {
        let mut engine = Engine::default();

        let txs = vec![
            Transaction {
                id: 1,
                client: 1,
                payload: transaction::TxPayload::Deposit {
                    amount: Amount::from(100).rescale(4),
                },
            },
            Transaction {
                id: 2,
                client: 2,
                payload: transaction::TxPayload::Deposit {
                    amount: Amount::from(200).rescale(4),
                },
            },
            Transaction {
                id: 3,
                client: 1,
                payload: transaction::TxPayload::Withdrawal {
                    amount: Amount::from(50).rescale(4),
                },
            },
        ];

        for tx in txs {
            engine.process_transaction(tx);
        }

        let acc1 = engine.clients[1].account();
        let acc2 = engine.clients[2].account();

        assert_eq!(acc1.available_funds(), Amount::from(50).rescale(4));
        assert_eq!(acc2.available_funds(), Amount::from(200).rescale(4));

        let counts = engine.accounts().count();
        assert_eq!(counts, 2);
    }
}
