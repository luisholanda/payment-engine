use std::collections::{HashMap, HashSet};

use crate::{Transaction, account::Account, transaction::TxPayload};

#[derive(Debug, Default)]
pub(crate) struct Client {
    account: Account,
    disputes: Disputes,
    txs: HashMap<u32, Transaction>,
}

impl Client {
    pub(crate) fn account(&self) -> &Account {
        &self.account
    }

    pub(super) fn process_transaction(&mut self, tx: Transaction) {
        if self.account.is_locked() {
            return;
        }

        match tx.payload {
            TxPayload::Deposit { amount } => {
                if self.txs.contains_key(&tx.id) {
                    return;
                }

                self.account.deposit(amount);
                self.txs.insert(tx.id, tx);
            }
            TxPayload::Withdrawal { amount } => {
                if self.txs.contains_key(&tx.id) {
                    return;
                }

                if self.account.withdraw(amount).is_ok() {
                    self.txs.insert(tx.id, tx);
                }
            }
            TxPayload::Dispute => {
                if !self.disputes.is_disputed(tx.id)
                    && let Some(original_tx) = self.txs.get(&tx.id)
                    && let Some(amount) = original_tx.deposited_amount()
                {
                    if self.account.hold_funds(amount).is_ok() {
                        self.disputes.dispute(tx.id);
                    }
                }
            }
            TxPayload::Resolve => {
                if let Some(original_tx) = self.disputed_transaction(tx.id)
                    && let Some(amount) = original_tx.deposited_amount()
                {
                    self.account.release_funds(amount);
                    self.disputes.resolve(tx.id);
                }
            }
            TxPayload::Chargeback => {
                if let Some(original_tx) = self.disputed_transaction(tx.id)
                    && let Some(amount) = original_tx.deposited_amount()
                {
                    self.account.chargeback(amount);
                    self.disputes.chargeback(tx.id);
                }
            }
        }
    }

    fn disputed_transaction(&self, id: u32) -> Option<&Transaction> {
        self.txs.get(&id).filter(|_| self.disputes.is_disputed(id))
    }
}

/// Tracks disputed transactions for a client.
#[derive(Debug, Default)]
pub(crate) struct Disputes {
    txs: HashSet<u32>,
    chargebacks: HashSet<u32>,
}

impl Disputes {
    pub(crate) fn is_disputed(&self, tx: u32) -> bool {
        self.txs.contains(&tx) || self.chargebacks.contains(&tx)
    }

    pub(crate) fn dispute(&mut self, tx: u32) -> bool {
        self.txs.insert(tx)
    }

    pub(crate) fn resolve(&mut self, tx: u32) -> bool {
        self.txs.remove(&tx)
    }

    pub(crate) fn chargeback(&mut self, tx: u32) -> bool {
        let found = self.txs.remove(&tx);

        if found {
            self.chargebacks.insert(tx);
        }

        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_withdrawal() {
        let txs = vec![
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Deposit { amount: 10.into() },
            },
            Transaction {
                id: 2,
                client: 1,
                payload: TxPayload::Withdrawal { amount: 5.into() },
            },
            Transaction {
                id: 3,
                client: 1,
                payload: TxPayload::Withdrawal { amount: 15.into() },
            },
        ];

        let mut client = Client::default();
        for tx in &txs {
            client.process_transaction(*tx);
        }

        assert_eq!(client.account.total_funds(), 5.into());
        assert!(!client.account.is_locked());
        assert!(client.txs.contains_key(&1));
        assert!(client.txs.contains_key(&2));
        assert!(!client.txs.contains_key(&3));
    }

    #[test]
    fn test_dispute_resolve() {
        let txs = vec![
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Deposit { amount: 10.into() },
            },
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Dispute,
            },
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Resolve,
            },
        ];

        let mut client = Client::default();
        for tx in &txs {
            client.process_transaction(*tx);
        }

        assert_eq!(client.account.total_funds(), 10.into());
        assert!(!client.account.is_locked());
        assert!(!client.disputes.is_disputed(1));
        assert!(client.txs.contains_key(&1));
    }

    #[test]
    fn test_dispute_chargeback() {
        let txs = vec![
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Deposit { amount: 10.into() },
            },
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Dispute,
            },
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Chargeback,
            },
        ];

        let mut client = Client::default();
        for tx in &txs {
            client.process_transaction(*tx);
        }

        assert_eq!(client.account.total_funds(), 0.into());
        assert!(client.account.is_locked());
        assert!(client.disputes.is_disputed(1));
        assert!(client.txs.contains_key(&1));
    }

    #[test]
    fn test_dispute_missing_tx() {
        let txs = vec![
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Deposit { amount: 10.into() },
            },
            Transaction {
                id: 2,
                client: 1,
                payload: TxPayload::Dispute,
            },
        ];

        let mut client = Client::default();

        for tx in &txs {
            client.process_transaction(*tx);
        }

        assert_eq!(client.account.total_funds(), 10.into());
        assert!(!client.account.is_locked());
        assert!(!client.disputes.is_disputed(2));
        assert!(client.txs.contains_key(&1));
        assert!(!client.txs.contains_key(&2));
    }

    #[test]
    fn test_dispute_withdrawal() {
        let txs = vec![
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Deposit { amount: 10.into() },
            },
            Transaction {
                id: 2,
                client: 1,
                payload: TxPayload::Withdrawal { amount: 5.into() },
            },
            Transaction {
                id: 2,
                client: 1,
                payload: TxPayload::Dispute,
            },
        ];

        let mut client = Client::default();

        for tx in &txs {
            client.process_transaction(*tx);
        }

        assert_eq!(client.account.total_funds(), 5.into());
        assert!(!client.account.is_locked());
        assert!(!client.disputes.is_disputed(2));
        assert!(client.txs.contains_key(&1));
        assert!(client.txs.contains_key(&2));
    }

    #[test]
    fn test_no_dispute_if_not_enough_funds() {
        let txs = vec![
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Deposit { amount: 10.into() },
            },
            Transaction {
                id: 2,
                client: 1,
                payload: TxPayload::Withdrawal { amount: 5.into() },
            },
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Dispute,
            },
        ];

        let mut client = Client::default();

        for tx in &txs {
            client.process_transaction(*tx);
        }

        assert_eq!(client.account.total_funds(), 5.into());
        assert!(!client.account.is_locked());
        assert!(!client.disputes.is_disputed(1));
        assert!(client.txs.contains_key(&1));
        assert!(client.txs.contains_key(&2));
    }

    #[test]
    fn test_dont_process_tx_after_locked() {
        let txs = vec![
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Deposit { amount: 10.into() },
            },
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Dispute,
            },
            Transaction {
                id: 1,
                client: 1,
                payload: TxPayload::Chargeback,
            },
            Transaction {
                id: 4,
                client: 1,
                payload: TxPayload::Deposit { amount: 10.into() },
            },
        ];

        let mut client = Client::default();
        for tx in &txs {
            client.process_transaction(*tx);
        }

        assert_eq!(client.account.total_funds(), 0.into());
        assert!(client.account.is_locked());
        assert!(!client.txs.contains_key(&4));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::{
        Amount,
        transaction::{any_transaction, any_transaction_with_types},
    };

    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig {
            max_shrink_iters: 40_000,
            ..ProptestConfig::with_cases(2_000)
        })]

        #[test]
        fn test_client_process_transaction_no_disputes(
            txs in prop::collection::vec(any_transaction_with_types(&["deposit", "withdrawal"]), 0..1_000 as _))
        {
            let mut client = Client::default();

            for &tx in &txs {
                client.process_transaction(tx);
            }

            for tx in &txs {
                if let TxPayload::Deposit {..} = tx.payload {
                    prop_assert!(client.txs.contains_key(&tx.id));
                }
            }

            let mut expected_total = Amount::ZERO;
            for tx in client.txs.values() {
                match tx.payload {
                    TxPayload::Deposit { amount } => expected_total += amount,
                    TxPayload::Withdrawal { amount } => expected_total -= amount,
                    _ => {}
                }
            }

            prop_assert_eq!(client.account.total_funds(), expected_total);

            prop_assert!(client.account.available_funds() >= Amount::ZERO);
            prop_assert_eq!(client.account.total_funds(), client.account.available_funds());

            prop_assert!(!client.account.is_locked(), "account is locked with no chargeback");
            prop_assert_eq!(client.account.held_funds(), Amount::ZERO);
        }

        #[test]
        fn test_client_process_transaction_with_disputes(txs in any_ledger(10_000)) {
            let mut client = Client::default();

            for &tx in &txs {
                client.process_transaction(tx);
            }

            let mut expected_total = Amount::ZERO;
            for (idx, &tx) in txs.iter().enumerate() {
                match tx.payload {
                    TxPayload::Deposit {..} => {
                        prop_assert!(client.txs.contains_key(&tx.id));
                    },
                    TxPayload::Chargeback => {
                        if let Some(original_tx) = client.txs.get(&tx.id) {
                            let og_idx = txs.iter().position(|&t| t.id == tx.id).unwrap();
                            if og_idx >= idx {
                                continue;
                            }

                            if let TxPayload::Deposit { amount } = original_tx.payload {
                                expected_total -= amount;
                            }
                        }
                    }
                    _ => continue,
                }
                if let TxPayload::Deposit {..} = tx.payload {
                }
            }

            let mut expected_held = Amount::ZERO;
            for tx in client.txs.values() {
                match tx.payload {
                    TxPayload::Deposit { amount } => expected_total += amount,
                    TxPayload::Withdrawal { amount } => expected_total -= amount,
                    _ => {}
                }
            }

            for &disputed_tx in &client.disputes.txs {
                if let Some(tx) = client.txs.get(&disputed_tx) {
                    if let TxPayload::Deposit { amount } = tx.payload {
                        expected_held += amount;
                    }
                }
            }

            prop_assert_eq!(client.account.total_funds(), expected_total);
            prop_assert_eq!(client.account.held_funds(), expected_held);

            prop_assert!(client.account.available_funds() >= Amount::ZERO);
            prop_assert_eq!(client.account.total_funds(), client.account.available_funds() + client.account.held_funds());

            if client.disputes.txs.is_empty() {
                prop_assert!(!client.account.is_locked(), "account is locked with no chargeback");
            }
        }
    }

    fn any_ledger(max_size: usize) -> impl Strategy<Value = Vec<Transaction>> {
        prop::collection::vec(any_transaction(), 0..max_size).prop_perturb(|mut txs, mut rng| {
            let mut seen_deposits = HashSet::new();
            let mut disputed = HashSet::new();

            for tx in &mut txs {
                match tx.payload {
                    TxPayload::Deposit { .. } => {
                        seen_deposits.insert(tx.id);
                    }
                    TxPayload::Dispute => {
                        if !disputed.is_empty()
                            && !seen_deposits.contains(&tx.id)
                            && rng.random_bool(0.95)
                        {
                            let deposit_id = seen_deposits
                                .iter()
                                .nth(rng.random_range(0..seen_deposits.len()))
                                .copied()
                                .unwrap();
                            tx.id = deposit_id;
                            disputed.insert(deposit_id);
                        }
                    }
                    TxPayload::Resolve | TxPayload::Chargeback => {
                        if !disputed.is_empty()
                            && !disputed.contains(&tx.id)
                            && rng.random_bool(0.99)
                        {
                            let deposit_id = seen_deposits
                                .iter()
                                .nth(rng.random_range(0..seen_deposits.len()))
                                .copied()
                                .unwrap();
                            seen_deposits.remove(&deposit_id);
                            tx.id = deposit_id;
                        }
                    }
                    _ => (),
                }
            }

            txs
        })
    }
}
