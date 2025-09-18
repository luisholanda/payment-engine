use std::borrow::Cow;

use crate::Amount;

/// Represents a financial transaction in the payment engine.
#[derive(Debug, Clone, Copy)]
pub struct Transaction {
    /// The associated transaction ID.
    ///
    /// For deposits and withdrawals, this is the unique ID of the transaction.
    /// For disputes, resolves, and chargebacks, this refers to the original
    /// transaction ID being disputed, resolved, or charged back.
    pub(crate) id: u32,
    /// The client ID associated with the transaction.
    pub(crate) client: u16,
    /// The payload of the transaction, which varies based on the transaction type.
    pub(crate) payload: TxPayload,
}

/// Enum representing the different types of transaction payloads.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TxPayload {
    /// A deposit transaction with a specified amount.
    Deposit {
        /// The amount involved in the deposit transaction.
        amount: Amount,
    },
    /// A withdrawal transaction with a specified amount.
    Withdrawal {
        /// The amount involved in the withdrawal transaction.
        amount: Amount,
    },
    /// A dispute transaction referencing an existing transaction ID.
    ///
    /// The disputed transaction ID is stored in the `id` field of the enclosing `Transaction`.
    Dispute,
    /// A resolve transaction referencing an existing disputed transaction ID.
    ///
    /// The resolved transaction ID is stored in the `id` field of the enclosing `Transaction`.
    Resolve,
    /// A chargeback transaction referencing an existing disputed transaction ID.
    ///
    /// The chargeback transaction ID is stored in the `id` field of the enclosing `Transaction`.
    Chargeback,
}

impl Transaction {
    pub(crate) fn deposited_amount(&self) -> Option<Amount> {
        match self.payload {
            TxPayload::Deposit { amount } => Some(amount),
            _ => None,
        }
    }
}

impl<'de> serde::Deserialize<'de> for Transaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn deserialize_cow_str<'de, D>(deserializer: D) -> Result<Cow<'de, str>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct CowStrVisitor;

            impl<'de> serde::de::Visitor<'de> for CowStrVisitor {
                type Value = Cow<'de, str>;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a string")
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(Cow::Owned(value.to_string()))
                }

                fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(Cow::Owned(v))
                }

                fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(Cow::Borrowed(value))
                }
            }

            deserializer.deserialize_str(CowStrVisitor)
        }

        #[derive(serde::Deserialize)]
        struct Inner<'d> {
            #[serde(
                rename = "type",
                deserialize_with = "deserialize_cow_str",
                borrow = "'d"
            )]
            typ: Cow<'d, str>,
            client: u16,
            tx: u32,
            amount: Option<fastnum::D256>,
        }

        let helper = Inner::deserialize(deserializer)?;

        let amount = helper.amount.map(|amt| amt.rescale(4));

        let payload = match &*helper.typ {
            "deposit" => TxPayload::Deposit {
                amount: amount.ok_or_else(|| {
                    serde::de::Error::missing_field("amount for deposit transaction")
                })?,
            },
            "withdrawal" => TxPayload::Withdrawal {
                amount: amount.ok_or_else(|| {
                    serde::de::Error::missing_field("amount for withdrawal transaction")
                })?,
            },
            "dispute" => TxPayload::Dispute,
            "resolve" => TxPayload::Resolve,
            "chargeback" => TxPayload::Chargeback,
            _ => {
                return Err(serde::de::Error::unknown_variant(
                    &helper.typ,
                    &["deposit", "withdrawal", "dispute", "resolve", "chargeback"],
                ));
            }
        };

        Ok(Transaction {
            id: helper.tx,
            client: helper.client,
            payload,
        })
    }
}

#[cfg(test)]
use proptest::prelude::*;

#[cfg(test)]
prop_compose! {
    pub(crate) fn any_transaction()
                                 (tx in any_transaction_with_types(&["deposit", "withdrawal", "dispute", "resolve", "chargeback"]))
                                 -> Transaction {
        tx
    }
}

#[cfg(test)]
prop_compose! {
    pub(crate) fn any_transaction_with_types(types: &'static [&'static str])
                                            (id in any::<u32>(),
                                             client in any::<u16>(),
                                             amount in any::<f32>(),
                                             payload_type in prop::sample::select(types))
                                            -> Transaction {
        Transaction {
            id,
            client,
            payload: match &*payload_type {
                "deposit" => TxPayload::Deposit { amount: Amount::from(amount.abs()).rescale(4) },
                "withdrawal" => TxPayload::Withdrawal { amount: Amount::from(amount.abs()).rescale(4) },
                "dispute" => TxPayload::Dispute,
                "resolve" => TxPayload::Resolve,
                "chargeback" => TxPayload::Chargeback,
                _ => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_transaction_serialization(tx in any_transaction()) {
            let row = csv::StringRecord::from(vec![
                match tx.payload {
                    TxPayload::Deposit { .. } => "deposit",
                    TxPayload::Withdrawal { .. } => "withdrawal",
                    TxPayload::Dispute => "dispute",
                    TxPayload::Resolve => "resolve",
                    TxPayload::Chargeback => "chargeback",
                }.to_string(),
                tx.client.to_string(),
                tx.id.to_string(),
                match tx.payload {
                    TxPayload::Deposit { amount }
                    | TxPayload::Withdrawal { amount } => amount.to_string(),
                    _ => "".to_string(),
                }
                ]
            );

            let deserialized: Transaction = row.deserialize(Some(&csv::StringRecord::from(vec![
                "type", "client", "tx", "amount"
            ]))).unwrap();

            prop_assert_eq!(deserialized.id, tx.id);
            prop_assert_eq!(deserialized.client, tx.client);

            match (deserialized.payload, tx.payload) {
                (TxPayload::Deposit { amount: a1 }, TxPayload::Deposit { amount: a2 }) => {
                    prop_assert_eq!(a1, a2);
                }
                (TxPayload::Withdrawal { amount: a1 }, TxPayload::Withdrawal { amount: a2 }) => {
                    prop_assert_eq!(a1, a2);
                }
                (TxPayload::Dispute, TxPayload::Dispute) => {}
                (TxPayload::Resolve, TxPayload::Resolve) => {}
                (TxPayload::Chargeback, TxPayload::Chargeback) => {}
                _ => prop_assert!(false, "Mismatched payload types"),
            }
        }
    }
}
