# `movement-sequencer`
The movement sequencer.  Initially built as a Snowman subnet, also known as `M1`.  

`M1` will be a scalable, secure, and decentralized Layer 1 solution for the Movement network.

We use `nix` to make things a little easier in creating a reproducible and determistic environment to run 
the subnet.  

You will need to have `nix` installed for this to all work which you can find here https://nixos.org/download

Once installed update update the file `~/.config/nix/nix.conf`

with:

`experimental-features = nix-command flakes configurable-impure-env`

The sequencer can be built with `nix build` which will run all tests, unit and e2e.

To develop you can jump into a development environment using `nix develop`
