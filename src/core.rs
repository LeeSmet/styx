use std::collections::HashMap;
use std::{collections::HashSet, net::Ipv6Addr, sync::Arc};

use log::{debug, error};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

use crate::crypto::ed25519::PUBLIC_KEY_LENGTH;
use crate::net::Subnet;
use crate::{
    crypto::ed25519::{PublicKey, SecretKey},
    peer::Peer,
};

/// Magic number to identify a control connection. Value is the ASCII byte value of CTRL.
const CONTROL_MAGIC: u32 = 0x43_54_52_4C;

/// Magic number to identify a data connection. Value is the ASCII byte value of DATA.
const DATA_MAGIC: u32 = 0x44_41_54_41;

/// Different types of connection which can be mad.
enum Connection {
    /// The remote indicates this is a control connection, originating from the given peer.
    Control(TcpStream, PublicKey),
    /// The remote indicates this is a data connection, originating from the given peer.
    Data(TcpStream, PublicKey),
}

/// The main control structure of the network.
pub struct Core {
    identity: SecretKey,
    identity_public: PublicKey,

    listener: Arc<TcpListener>,
    peer_cache: HashSet<Peer>,
    /// Keep track of active control connections
    active_peers: HashMap<PublicKey, TcpStream>,
    /// Keep track of active data connections
    active_data_peers: HashMap<Subnet, TcpStream>,
}

impl Core {
    /// Create a new Core from the given secret key. The listener must be provided, and the Core
    /// will automatically start accepting requests once it is fully initialized.
    ///
    /// # Panics
    ///
    /// This function will panic if not called from withing a tokio runtime.
    pub fn new(identity: SecretKey, listener: TcpListener) -> Arc<Self> {
        let identity_public = identity.public_key();

        let (tx, con_receiver) = mpsc::channel(10);
        let listener = Arc::new(listener);

        let core = Arc::new(Self {
            identity,
            identity_public,
            listener,
            peer_cache: HashSet::new(),
            active_peers: HashMap::new(),
            active_data_peers: HashMap::new(),
        });

        tokio::spawn(Core::start_listener(core.listener.clone(), tx));
        tokio::spawn(Core::handle_connections(core.clone(), con_receiver));

        core
    }

    /// Get our own address as calculated from the public key of our identity.
    pub fn address(&self) -> Ipv6Addr {
        self.identity_public.address()
    }

    /// Drive the core. This future does not resolve until the listener is shut down.
    async fn handle_connections(self: Arc<Self>, mut con_receiver: mpsc::Receiver<Connection>) {
        while let Some(connection) = con_receiver.recv().await {
            match connection {
                Connection::Control(con, peer) => {
                    tokio::spawn(Core::spawn_control_con());
                }
                Connection::Data(con, peer) => {
                    tokio::spawn(Core::spawn_data_con());
                }
            }
        }
    }

    async fn spawn_control_con() {
        todo!();
    }

    async fn spawn_data_con() {
        todo!();
    }

    /// Start listening for new inbound connections.
    async fn start_listener(listener: Arc<TcpListener>, tx: mpsc::Sender<Connection>) {
        loop {
            let (mut con, remote) = listener.accept().await.unwrap();
            debug!("Accepted new connection from {}", remote);
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buffer = [0; PUBLIC_KEY_LENGTH];
                if let Err(e) = con.read_exact(&mut buffer[..]).await {
                    debug!("Connection closed while reading remote public key: {}", e);
                    return;
                }
                let pk = match PublicKey::from_bytes(buffer) {
                    Ok(pk) => pk,
                    Err(e) => {
                        debug!(
                            "Closing connection after client sent invalid public key: {}",
                            e
                        );
                        return;
                    }
                };
                let magic = match con.read_u32().await {
                    Ok(m) => m,
                    Err(e) => {
                        // It could be that the remote closed the connection, which is fine
                        debug!("Connection to {} closed because of {}", remote, e);
                        return;
                    }
                };
                if let Err(e) = match magic {
                    CONTROL_MAGIC => tx.send(Connection::Control(con, pk)).await,
                    DATA_MAGIC => tx.send(Connection::Data(con, pk)).await,
                    _ => {
                        debug!("Connection closed after sending unexpected identification data");
                        return;
                    }
                } {
                    // Couldn't send data to core
                    error!("Could not pass connection to core: {}", e);
                }
            });
        }
    }
}
