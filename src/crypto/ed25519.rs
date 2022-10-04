use ed25519_dalek::{Keypair, PublicKey as DalekPublicKey, SecretKey};

/// Length in bytes of an Ed25519 public key.
pub const PUBLIC_KEY_LENGTH: usize = ed25519_dalek::PUBLIC_KEY_LENGTH;

/// An Ed25519 public key.
pub struct PublicKey(DalekPublicKey);

impl PublicKey {
    /// Creates a new instance of [`PublicKey`] from the given bytes.
    pub fn from_bytes(raw: [u8; PUBLIC_KEY_LENGTH]) -> Result<Self, super::Error> {
        // We can ignore the invalid lenght error here since we take a fixed length slice of the
        // correct length as argument.
        Ok(Self(
            DalekPublicKey::from_bytes(&raw[..]).map_err(|_| super::Error::InvalidData)?,
        ))
    }

    /// View this public key as a byte array
    pub fn as_bytes(&self) -> &[u8; PUBLIC_KEY_LENGTH] {
        self.0.as_bytes()
    }
}
