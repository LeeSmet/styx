# Styx

This is a small research project aimed to implement an IPv6 overlay network. The
underlay follows a basic model consisting of private and public nodes. A connection
is either direct or via a public node acting as a relay. The primary goal is to
have at least 2 nodes on different network communicate to each other over the overlay,
with and without the help of a public 3rd peer in between.

## Interesting features

These may or may not be added, depending on the evolution of the codebase, but are
nonetheless interesting for the continuation of the project, either as continued
development here or as a new project. In any case, the POC implementation should
at the very least consider their impact and requirement on the codebase, and plan
accordingly

- Encrypted traffic between 2 end nodes, such that intermediates cannot intercept it.
  - Double ratchet algorithm might be an interesting choice here.
- Closest intermediate selection based on the endpoint. Closest in this case would
refer to latency rather than physical distance.
- Automatic peer discovery, i.e. automatic connection to public nodes which are known
by peers but not currently known by us.
- Asynchronous connection attempt to a remote peer if we are currently connected
through an intermediary. In this case, already existing connections in the overlay
need to be preserved if a new underlay connection is made.
