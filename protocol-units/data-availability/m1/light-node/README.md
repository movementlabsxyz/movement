# `m1-da-light-node`
The `m1-da-light-node` is the light node server responsible for securely reading the order of the Movement Sequencer network. 

The `m1-da-light-node` has two modes:
- **blocky**: this mode is used for the standard operation of the light node. It will simply forward blobs in and blobs out of the network. This is suited to when you wish to handle all sequencing at a higher level without further delegation beyond the blob ordering of the Movement Network. The Movement Network will only sequencer blocks for you.
- **sequencer**: this mode regards input blobs as transactions and output blobs as blocks. That is, instead of a one-to-one mapping between input and output blobs, the light node will aggregate input blobs into a block and output results block-by-block. This is suited to when you wish to delegate sequencing to the Movement Network. The Movement Network will effectively sequencer transactions and blocks for you.

The `m1-da-light-node` should always be run in a trusted environment. It is a sidecar to services that wish to interact with the Movement Network.