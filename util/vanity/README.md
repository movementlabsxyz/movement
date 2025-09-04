# Vanity 🧂⛏️

Vanity is a CLI for mining vanity move addresses.

Vanity is heavily inspired by [Create2Crunch](https://github.com/0age/create2crunch).

## Installation

```bash
git clone https://github.com/movementlabsxyz/movement.git
cd movement
# Run it directly
RUST_LOG=info cargo run -p vanity --release -- move --starts <STARTS_PATTERN> --ends <ENDS_PATTERN>

cd util/vanity
# Add it to your path
cargo install --path .
```
