use crate::crypto::ed25519::PublicKey;
use std::net::SocketAddr;

/// A remote client identified by a public key.
pub struct Peer {
    public_key: PublicKey,
    listen_addrs: Vec<SocketAddr>,
}

impl Peer {
    /// Construst a new [`Peer`] with the given [`PublicKey`], and known listening addresses.
    pub fn new(public_key: PublicKey, listen_addrs: Vec<SocketAddr>) -> Self {
        Self {
            public_key,
            listen_addrs,
        }
    }

    /// Get a reference to the [`PublicKey`] associated with this peer.
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
}
