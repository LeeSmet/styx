use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// Size of the header sent on the wire before every frame.
const HEADER_WIRE_SIZE: usize = 4;

// TODO: proper version, this is just a placeholder.
const PROTO_VERSION: u8 = 0;

// Types for different frames.

/// Type for the PING frame.
const TYPE_PING: u8 = 0;

/// Minimal size of an actual ping frame.
const MINIMAL_PING_FRAME_SIZE: u16 = 4;

/// Frames transmitted over a control connection to a peer. Control frames don't hold actual data,
/// as that is send and received over a dedicated connection.
pub enum ControlFrame {
    /// A ping frame, containing the ID of the ping.
    Ping(u32),
}

/// Header used to send frames on the wire.
struct FrameHeader {
    /// Version of the protocol.
    version: u8,
    /// Type of the frame.
    _type: u8,
    /// Length of the frame. Since we primarily use this protocol on command and control
    /// connections, which don't contain any actual data (only metadata), size is expected to be
    /// small.
    len: u16,
}

pub struct ControlDecoder {
    /// Save a header after we decode one, even if we didn't receive the remainder of the data yet.
    header: Option<FrameHeader>,
}

impl Decoder for ControlDecoder {
    type Item = ControlFrame;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let header = if let Some(header) = self.header.take() {
            header
        } else {
            // NOTE: Technically, we would first try to read the version byte to then decide how to
            // continue. Specifically, by reading the version byte first, we allow for modifications to
            // the actual header structure. This could go as far as modifying the version structure
            // itself. For instance, if the version is changed to an actual semver version of say 3
            // bytes, 1 byte for each field (1 for major, 1 for minor, 1 for patch), This could be
            // indicated by setting the version byte to some chosen value (say > 127, first bit set),
            // and then based on that read the _actual_ version from the following bytes.
            if src.len() < HEADER_WIRE_SIZE {
                // Insufficient data for the header.
                return Ok(None);
            }

            // We have sufficient data, decode it.
            // SAFETY: unwraps here are safe as we slice the buffer to the exact size, and we know
            // there is sufficient data available by virtue of the previous check.
            let version = u8::from_be_bytes(src[0..1].try_into().unwrap());
            let _type = u8::from_be_bytes(src[1..2].try_into().unwrap());
            let len = u16::from_be_bytes(src[2..4].try_into().unwrap());

            // Remove decoded bytes from the buffer.
            src.advance(HEADER_WIRE_SIZE);

            FrameHeader {
                version,
                _type,
                len,
            }
        };

        // Check if the buffer has enough data to decode the frame.
        // NOTE: we cast header len to usize for the comparison, as casting src.len() to u16 might
        // truncate the value of src if more than u16::MAX bytes are available, which could falsely
        // indicate that not enough data is available.
        if src.len() < header.len as usize {
            // Not enough data. Reserve sufficient data for the full frame, save the header, and exit.
            // SAFETY: this subtraction can't underflow as we just checked that src.len() is
            // smaller than header.size.
            src.reserve(header.len as usize - src.len());
            self.header = Some(header);
            return Ok(None);
        }

        // Decode the frame.
        match header._type {
            TYPE_PING => {
                // First 4 bytes are the ping ID.
                // NOTE: we need 4 bytes for the ping ID, but we will allow an arbitrary amount of
                // bytes to be passed after this. This _might_ be useful if at some point other data
                // is included, as older peers won't return a hard error when they fail to decode
                // the frame (although at this point the version field in the header should be
                // incremented to make this clear).
                if header.len < MINIMAL_PING_FRAME_SIZE {
                    // Malformed frame, remove the data and return an error. By removing the data
                    // we might be able to save the connection.
                    src.advance(header.len as usize);
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "insufficient data to decode a ping frame",
                    ))
                } else {
                    // SAFETY: we checked that we have sufficient data (buffer is at least header.len
                    // bytes large, and header.len is at least 4 bytes to decode the ID). Unwrap is
                    // safe as we convert slice with known size.
                    let id = u32::from_be_bytes(src[..4].try_into().unwrap());
                    // Remove bytes from the buffer. As explained we remove the amount of bytes as
                    // indicated in the header, not just the bytes for the ID.
                    src.advance(header.len as usize);
                    Ok(Some(ControlFrame::Ping(id)))
                }
            }
            _ => {
                // Unknown frame. This is an error. However, we clear the specified amount of bytes
                // from the buffer, as this might allow us to recover the connection. This is
                // helpful for instance, if the remote is on a newer version and didn't verify that
                // we can decode the frame.
                src.advance(header.len as usize);
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unknown version",
                ))
            }
        }
    }
}
