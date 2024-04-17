# `m1-da-light-node`
The `m1-da-light-node` is the light node server responsible for securely reading the order of the Movement Sequencer network. 

The `m1-da-light-node` has two modes:
- **blocky**: this mode is used for the standard operation of the light node. It will simply forward blobs in and blobs out of the network. 
- **sequencer**: this mode regards input blobs as transactions and output blobs as blocks. That is, instead of a one-to-one mapping between input and output blobs, the light node will aggregate input blobs into a block and output results block-by-block.