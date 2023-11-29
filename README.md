# Partial Block Building and TEEs

We propose a builder-relayer integration that runs partial block auctions inside a TEE. The main idea is that the TEE will securely hold auctions for the ToB, and the proposer has the option to request the insertion of specific transactions in the RoB.

The advantages of running inside a TEE are two-fold:

1. The relayer and the builder can be integrated without downsides to the proposer because the TEE has provable honesty; therefore, there is no need for the assumption that the relayer must exist as a third party that both the builder and proposer must trust
2. The TEE ensures a secure transaction ordering

We use EigenLayer to incentivize (and punish) the honesty of the party running the builder-relayer integration.

There are two ways in which the proposer can add its transactions:

## MEV-BooTEE

The TEE sends the ToB and the RoB to the proposer, and the proposer adds its transactions and publishes the block. To ensure that the proposer does not act maliciously, the proposer also has to be enrolled in an EigenLayer scheme to incentivize honest behavior.

Similar to [MEV-Boost+/++](https://research.eigenlayer.xyz/t/mev-boost-liveness-first-relay-design)

## PEPC-TEE

The proposer sends to the TEE a list of transactions that must be included in the block and the TEE ensures that the block it builds includes them. The TEE then sends the header to the proposer and once the header is signed by the proposer, the TEE broadcasts the block to the network.

Similar to [PECP-Boost](https://hackmd.io/@bchain/BJkarrEWp)