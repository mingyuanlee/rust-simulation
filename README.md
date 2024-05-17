Basic Commands

```
cargo run -- start-node

cargo run -- create-account <id-of-account> <starting-balance>

cargo run -- transfer <from-account> <to-account> <amount>

cargo run -- balance <account>
```

Example

```
# In the first terminal run
cargo run -- start-node

# In the second terminal run
cargo run -- create-account alice 1000
cargo run -- create-account bob 1000
cargo run -- transfer alice bob 500

# This should give 500
cargo run -- balance alice

# This should give 1500
cargo run -- balance bob
```