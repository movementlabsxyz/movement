# Vanity 🧂⛏️

Vanity is a CLI for mining vanity move addresses.

Vanity is heavely inspired by [Create2Crunch](https://github.com/0age/create2crunch).

## Installation

```bash
git clone https://github.com/movementlabsxyz/movement.git
cd movement
# Run it directly
cargo run -p vanity --release -- move --starts-pattern <STARTS_PATTERN> --ends-pattern <ENDS_PATTERN>

cd util/vanity
# Add it to your path
cargo install --path .
```

*Currently requires ignoring all instances of "-C", "link-arg=-fuse-ld=lld" flags in .cargo/config.toml*