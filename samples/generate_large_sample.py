#!/usr/bin/env python3
"""
Generate large-scale test data for the payment engine CLI.

This script creates a comprehensive test case with thousands of transactions
across hundreds of clients, testing the engine's performance and correctness
at scale.
"""

import random
import csv
from decimal import Decimal, ROUND_HALF_UP

# Configuration
NUM_CLIENTS = 1000
NUM_TRANSACTIONS = 10_000_000
DEPOSIT_PROBABILITY = 0.4
WITHDRAWAL_PROBABILITY = 0.35
DISPUTE_PROBABILITY = 0.15
RESOLVE_PROBABILITY = 0.06
CHARGEBACK_PROBABILITY = 0.04

# Amount ranges
MIN_AMOUNT = Decimal('0.01')
MAX_AMOUNT = Decimal('10000.00')
SMALL_AMOUNT_THRESHOLD = Decimal('100.00')
SMALL_AMOUNT_PROBABILITY = 0.7

class TransactionGenerator:
    def __init__(self):
        self.next_tx_id = 1
        self.transactions = []
        self.client_deposits = {}  # client_id -> list of deposit tx_ids
        self.client_disputed = {}  # client_id -> set of disputed tx_ids
        self.client_chargebacks = {}  # client_id -> set of chargeback tx_ids

    def generate_amount(self) -> Decimal:
        """Generate a realistic transaction amount."""
        if random.random() < SMALL_AMOUNT_PROBABILITY:
            # Small amounts (0.01 to 100.00)
            amount = random.uniform(float(MIN_AMOUNT), float(SMALL_AMOUNT_THRESHOLD))
        else:
            # Larger amounts (100.01 to 10000.00)
            amount = random.uniform(float(SMALL_AMOUNT_THRESHOLD), float(MAX_AMOUNT))

        # Round to 4 decimal places
        return Decimal(str(amount)).quantize(Decimal('0.0001'), rounding=ROUND_HALF_UP)

    def add_transaction(self, tx_type: str, client: int, tx_id: int, amount: str = ""):
        """Add a transaction to the list."""
        self.transactions.append({
            'type': tx_type,
            'client': client,
            'tx': tx_id,
            'amount': amount
        })

    def generate_deposit(self, client: int) -> int:
        """Generate a deposit transaction and return tx_id."""
        tx_id = self.next_tx_id
        self.next_tx_id += 1
        amount = self.generate_amount()

        self.add_transaction('deposit', client, tx_id, str(amount))

        # Track deposits for potential disputes
        if client not in self.client_deposits:
            self.client_deposits[client] = []
        self.client_deposits[client].append(tx_id)

        return tx_id

    def generate_withdrawal(self, client: int) -> int:
        """Generate a withdrawal transaction and return tx_id."""
        tx_id = self.next_tx_id
        self.next_tx_id += 1
        amount = self.generate_amount()

        self.add_transaction('withdrawal', client, tx_id, str(amount))
        return tx_id

    def generate_dispute(self, client: int) -> bool:
        """Generate a dispute transaction. Returns True if successful."""
        # Can only dispute existing deposits that aren't already disputed or chargedback
        available_deposits = []
        if client in self.client_deposits:
            disputed = self.client_disputed.get(client, set())
            chargebacks = self.client_chargebacks.get(client, set())
            available_deposits = [
                tx_id for tx_id in self.client_deposits[client]
                if tx_id not in disputed and tx_id not in chargebacks
            ]

        if not available_deposits:
            return False

        tx_id = random.choice(available_deposits)
        self.add_transaction('dispute', client, tx_id)

        # Track disputed transactions
        if client not in self.client_disputed:
            self.client_disputed[client] = set()
        self.client_disputed[client].add(tx_id)

        return True

    def generate_resolve(self, client: int) -> bool:
        """Generate a resolve transaction. Returns True if successful."""
        # Can only resolve currently disputed transactions
        if client not in self.client_disputed or not self.client_disputed[client]:
            return False

        tx_id = random.choice(list(self.client_disputed[client]))
        self.add_transaction('resolve', client, tx_id)

        # Remove from disputed
        self.client_disputed[client].remove(tx_id)

        return True

    def generate_chargeback(self, client: int) -> bool:
        """Generate a chargeback transaction. Returns True if successful."""
        # Can only chargeback currently disputed transactions
        if client not in self.client_disputed or not self.client_disputed[client]:
            return False

        # Chargebacks will lock the account, so we limit their frequency.
        if random.random() > CHARGEBACK_PROBABILITY:
            return False

        tx_id = random.choice(list(self.client_disputed[client]))
        self.add_transaction('chargeback', client, tx_id)

        # Move from disputed to chargebacks
        self.client_disputed[client].remove(tx_id)
        if client not in self.client_chargebacks:
            self.client_chargebacks[client] = set()
        self.client_chargebacks[client].add(tx_id)

        return True

    def generate_transactions(self) -> list[dict]:
        """Generate the full set of transactions."""
        print(f"Generating {NUM_TRANSACTIONS} transactions for {NUM_CLIENTS} clients...")

        # Generate transactions
        for i in range(NUM_TRANSACTIONS):
            if i % 1000 == 0:
                print(f"\rGenerated {i} transactions...", end='')

            client = random.randint(1, NUM_CLIENTS)
            rand = random.random()

            if rand < DEPOSIT_PROBABILITY:
                self.generate_deposit(client)
            elif rand < DEPOSIT_PROBABILITY + WITHDRAWAL_PROBABILITY:
                self.generate_withdrawal(client)
            elif rand < DEPOSIT_PROBABILITY + WITHDRAWAL_PROBABILITY + DISPUTE_PROBABILITY:
                if not self.generate_dispute(client):
                    # If we can't dispute, generate a deposit instead
                    self.generate_deposit(client)
            elif rand < DEPOSIT_PROBABILITY + WITHDRAWAL_PROBABILITY + DISPUTE_PROBABILITY + RESOLVE_PROBABILITY:
                if not self.generate_resolve(client):
                    # If we can't resolve, generate a deposit instead
                    self.generate_deposit(client)
            else:
                if not self.generate_chargeback(client):
                    # If we can't chargeback, generate a deposit instead
                    self.generate_deposit(client)

        print(f"\nGenerated {len(self.transactions)} transactions total")
        return self.transactions

def main():
    """Generate large-scale test data."""
    print("Starting large-scale test data generation...")

    generator = TransactionGenerator()
    transactions = generator.generate_transactions()

    # Write to CSV
    output_file = 'samples/large_scale/input.csv'
    print(f"Writing transactions to {output_file}...")

    with open(output_file, 'w', newline='') as csvfile:
        fieldnames = ['type', 'client', 'tx', 'amount']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)

        writer.writeheader()
        for tx in transactions:
            writer.writerow(tx)

    print(f"Successfully generated {len(transactions)} transactions")

    # Print statistics
    tx_types = {}
    for tx in transactions:
        tx_type = tx['type']
        tx_types[tx_type] = tx_types.get(tx_type, 0) + 1

    print("\nTransaction type distribution:")
    for tx_type, count in sorted(tx_types.items()):
        percentage = (count / len(transactions)) * 100
        print(f"  {tx_type}: {count} ({percentage:.1f}%)")

    print(f"\nClients with transactions: {len(set(tx['client'] for tx in transactions))}")
    print(f"Total transaction IDs used: {generator.next_tx_id - 1}")

    print("\nTo generate expected output, run:")
    print(f"cargo run --release -- {output_file} > samples/large_scale/output.csv")

if __name__ == '__main__':
    main()
