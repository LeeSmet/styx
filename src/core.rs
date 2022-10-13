use std::{collections::HashSet, net::Ipv6Addr};

use tokio::net::TcpListener;

use crate::{
    crypto::ed25519::{PublicKey, SecretKey},
    peer::Peer,
};

/// The main controll structure of the network.
pub struct Core {
    identity: SecretKey,
    identity_public: PublicKey,

    listener: TcpListener,
    peer_cache: HashSet<Peer>,
}

impl Core {
    /// Create a new Core from the given secret key. The listener must be provided, and the Core
    /// will automatically start accepting requests once it is fully initialized.
    pub fn new(identity: SecretKey, listener: TcpListener) -> Self {
        let identity_public = identity.public_key();

        Self {
            identity,
            identity_public,
            listener,
            peer_cache: HashSet::new(),
        }
    }

    /// Get our own address as calculated from the public key of our identity.
    pub fn address(&self) -> Ipv6Addr {
        self.identity_public.address()
    }
}
