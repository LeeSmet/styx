use ed25519_dalek::{Keypair, PublicKey as DalekPublicKey, SecretKey as DalekSecretKey};
use std::net::Ipv6Addr;

/// Length in bytes of an Ed25519 public key.
pub const PUBLIC_KEY_LENGTH: usize = ed25519_dalek::PUBLIC_KEY_LENGTH;

/// Length in bytes of an Ed25519 secret key.
pub const SECRET_KEY_LENGTH: usize = ed25519_dalek::SECRET_KEY_LENGTH;

/// Ported from <https://github.com/yggdrasil-network/yggdrasil-go/blob/8c454a146cb70aa07ee2c87af964f5c1394da299/src/address/address.go#L19>.
const PREFIX: [u8; 1] = [0x02];

/// Amount of bytes in an IPv6 address.
const IPV6_OCTETS: usize = 16;

/// An Ed25519 secret key.
pub struct SecretKey(DalekSecretKey);

/// An Ed25519 public key.
#[derive(Clone)]
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

    /// Derive the IPv6 address from the [`PublicKey`].
    ///
    /// This is ported from <https://github.com/yggdrasil-network/yggdrasil-go/blob/8c454a146cb70aa07ee2c87af964f5c1394da299/src/address/address.go#L51>.
    /// It is not entirely clear why this function works like this, perhaps there are better ways.
    pub fn address(&self) -> Ipv6Addr {
        let mut working_buffer = [0; PUBLIC_KEY_LENGTH];
        for (b, o) in working_buffer.iter_mut().zip(self.0.as_bytes()) {
            *b = !*o;
        }

        let mut done = false;
        let mut ones = 0u8;
        let mut bits = 0u8;
        let mut nbits = 0;

        let mut temp = [0; PUBLIC_KEY_LENGTH];
        // Workaround to allow temp to be stack allocated - manually keep track of which byte
        // to set.
        let mut temp_idx = 0;

        for idx in 0..working_buffer.len() * 8 {
            let bit = (working_buffer[idx / 8] & (0x80 >> (idx % 8) as u8)) >> (7 - idx % 8) as u8;
            if !done && bit != 0 {
                ones += 1;
                continue;
            }
            if !done && bit == 0 {
                done = true;
                continue;
            }
            bits = (bits << 1) | bit;
            nbits += 1;
            if nbits == 8 {
                nbits = 0;
                temp[temp_idx] = bits;
                temp_idx += 1;
            }
        }

        let mut raw_addr = [0; IPV6_OCTETS];
        // SAFETY: Panic only happens if the slices have different length, but raw_addr is sliced
        // to the size of PREFIX.
        raw_addr[..PREFIX.len()].copy_from_slice(&PREFIX[..]);
        raw_addr[PREFIX.len()] = ones;
        // SAFETY: Panic only happens if the slices have different length, but temp is sliced to the
        // same size of the raw_addr slice.
        raw_addr[PREFIX.len() + 1..].copy_from_slice(&temp[..IPV6_OCTETS - (PREFIX.len() + 1)]);

        Ipv6Addr::from(raw_addr)
    }
}

impl SecretKey {
    /// Creates a new instance of [`SecretKey`] from the given bytes.
    pub fn from_bytes(raw: [u8; SECRET_KEY_LENGTH]) -> Self {
        // We can ignore the invalid lenght error here since we take a fixed length slice of the
        // correct length as argument.
        // SAFETY: this only returns an error if the slice is not  of lenght SECRET_KEY_LENGTH,
        // which can't happen as we have an array of that exact length and slice over its full
        // length.
        Self(DalekSecretKey::from_bytes(&raw[..]).unwrap())
    }

    /// View this secret key as a byte array
    pub fn as_bytes(&self) -> &[u8; SECRET_KEY_LENGTH] {
        self.0.as_bytes()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey((&self.0).into())
    }
}

#[cfg(test)]
mod tests {
    use super::PublicKey;
    use std::net::Ipv6Addr;

    #[test]
    /// Test ported from
    /// <https://github.com/yggdrasil-network/yggdrasil-go/blob/8c454a146cb70aa07ee2c87af964f5c1394da299/src/address/address_test.go#L56>.
    fn address_derive() {
        let key: PublicKey = PublicKey::from_bytes([
            189, 186, 207, 216, 34, 64, 222, 61, 205, 18, 57, 36, 203, 181, 82, 86, 251, 141, 171,
            8, 170, 152, 227, 5, 82, 138, 184, 79, 65, 158, 110, 25,
        ])
        .unwrap();

        let expected_ip = Ipv6Addr::from([
            2, 0, 132, 138, 96, 79, 187, 126, 67, 132, 101, 219, 141, 182, 104, 149,
        ]);

        assert_eq!(key.address(), expected_ip)
    }
}
