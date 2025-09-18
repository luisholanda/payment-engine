use crate::Amount;

/// A client's account in the payment engine.
#[derive(Debug, Default)]
pub struct Account {
    /// The available funds in the account.
    available: Amount,
    /// The held funds in the account, typically due to disputes.
    held: Amount,
    /// Indicates whether the account is locked due to a chargeback.
    locked: bool,
}

macro_rules! debug_assert_not_locked {
    ($self: ident) => {
        debug_assert!(
            !$self.is_locked(),
            "Operation not allowed: account is locked due to a chargeback."
        );
    };
}

impl Account {
    pub fn total_funds(&self) -> Amount {
        self.available + self.held
    }

    pub fn available_funds(&self) -> Amount {
        self.available
    }

    pub fn held_funds(&self) -> Amount {
        self.held
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub(crate) fn deposit(&mut self, amount: Amount) {
        debug_assert_not_locked!(self);

        self.available += amount;
    }

    /// Withdrawal a given amount from the account.
    ///
    /// # Errors
    ///
    /// Returns an error if there are insufficient available funds for the transaction.
    pub(crate) fn withdraw(&mut self, amount: Amount) -> Result<(), ()> {
        debug_assert_not_locked!(self);

        if self.available >= amount {
            self.available -= amount;
            Ok(())
        } else {
            Err(())
        }
    }

    pub(crate) fn hold_funds(&mut self, amount: Amount) -> Result<(), ()> {
        debug_assert_not_locked!(self);

        if self.available >= amount {
            self.available -= amount;
            self.held += amount;
            Ok(())
        } else {
            Err(())
        }
    }

    pub(crate) fn release_funds(&mut self, amount: Amount) {
        debug_assert_not_locked!(self);
        debug_assert!(self.held >= amount, "Resolving more than held");

        self.held -= amount;
        self.available += amount;
    }

    pub(crate) fn chargeback(&mut self, amount: Amount) {
        debug_assert_not_locked!(self);
        debug_assert!(self.held >= amount, "Chargeback more than held");

        self.held -= amount;
        self.locked = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit() {
        let mut account = Account::default();
        account.deposit(Amount::from(100));
        assert_eq!(account.available_funds(), Amount::from(100));
        assert_eq!(account.held_funds(), Amount::from(0));
        assert_eq!(account.total_funds(), Amount::from(100));
    }

    #[test]
    fn test_withdraw() {
        let mut account = Account::default();
        account.deposit(Amount::from(100));
        assert!(account.withdraw(Amount::from(50)).is_ok());
        assert_eq!(account.available_funds(), Amount::from(50));
        assert_eq!(account.held_funds(), Amount::from(0));
        assert_eq!(account.total_funds(), Amount::from(50));

        assert!(account.withdraw(Amount::from(60)).is_err());
        assert_eq!(account.available_funds(), Amount::from(50));
    }

    #[test]
    fn test_hold_funds() {
        let mut account = Account::default();
        account.deposit(Amount::from(100));
        assert!(account.hold_funds(Amount::from(30)).is_ok());
        assert_eq!(account.available_funds(), Amount::from(70));
        assert_eq!(account.held_funds(), Amount::from(30));
        assert_eq!(account.total_funds(), Amount::from(100));

        assert!(account.hold_funds(Amount::from(80)).is_err());
        assert_eq!(account.available_funds(), Amount::from(70));
    }

    #[test]
    fn test_release_funds() {
        let mut account = Account::default();
        account.deposit(Amount::from(100));
        account.hold_funds(Amount::from(40)).unwrap();
        account.release_funds(Amount::from(20));
        assert_eq!(account.available_funds(), Amount::from(80));
        assert_eq!(account.held_funds(), Amount::from(20));
        assert_eq!(account.total_funds(), Amount::from(100));
    }

    #[test]
    fn test_chargeback() {
        let mut account = Account::default();
        account.deposit(Amount::from(100));
        account.hold_funds(Amount::from(50)).unwrap();
        account.chargeback(Amount::from(50));
        assert_eq!(account.available_funds(), Amount::from(50));
        assert_eq!(account.held_funds(), Amount::from(0));
        assert_eq!(account.total_funds(), Amount::from(50));
        assert!(account.is_locked());

        // Further operations should panic in debug mode
        #[cfg(debug_assertions)]
        {
            use std::panic::AssertUnwindSafe;

            fn test_panic<F: FnOnce() -> R, R>(f: F) -> Result<R, Box<dyn std::any::Any + Send>> {
                std::panic::catch_unwind(AssertUnwindSafe(f))
            }

            let result = test_panic(|| account.deposit(Amount::from(10)));
            assert!(result.is_err());

            let result = test_panic(|| account.withdraw(Amount::from(10)));
            assert!(result.is_err());

            let result = test_panic(|| account.hold_funds(Amount::from(10)));
            assert!(result.is_err());

            let result = test_panic(|| account.release_funds(Amount::from(10)));
            assert!(result.is_err());
        }
    }
}
