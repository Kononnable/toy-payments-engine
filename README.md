# toy-payments-engine

## Assumptions
- Dispute is available only on deposit transactions. Dispute transaction description doesn't precise on which type of transaction it's applicable, however this description makes sense only for deposit transactions. If other types of transactions can be disputed different business logic should be used.
- Freezing(locking) client account doesn't change how transactions are processed(transactions are still processed on frozen account)