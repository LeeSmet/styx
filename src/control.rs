use bytes::{Buf, BufMut, BytesMut};
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

/// A [`Codec`](tokio_util::codec) for control frames.
pub struct ControlCodec {
    /// Save a header after we decode one, even if we didn't receive the remainder of the data yet.
    header: Option<FrameHeader>,
}

impl ControlCodec {
    /// Create a new [`ControlCodec`].
    pub fn new() -> Self {
        Self { header: None }
    }
}

impl Decoder for ControlCodec {
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
            let version = src.get_u8();
            let _type = src.get_u8();
            let len = src.get_u16();

            // Don't advance the buffer manually as that is already done by reading the individual
            // header pieces.

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
                    // bytes large, and header.len is at least 4 bytes to decode the ID).
                    let id = src.get_u32();
                    // Remove bytes from the buffer. As explained we remove the amount of bytes as
                    // indicated in the header, not just the bytes for the ID. Keep in mind that we
                    // already advanced 4 bytes by reading the ID. This subtraction is safe as we
                    // checked header.len() is at least this large.
                    src.advance(header.len as usize - 4);
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

impl Encoder<ControlFrame> for ControlCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: ControlFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Get type of the frame
        let (_type, len) = match item {
            ControlFrame::Ping(_) => (TYPE_PING, MINIMAL_PING_FRAME_SIZE),
        };

        // Reserve sufficient data in the buffer.
        dst.reserve(HEADER_WIRE_SIZE + len as usize);

        // Don't create a header, just write out the data in the correct order.
        // - 1 byte version
        // - 1 byte type
        // - 2 byte frame length
        dst.put_u8(PROTO_VERSION);
        dst.put_u8(_type);
        dst.put_u16(len);

        match item {
            ControlFrame::Ping(id) => {
                // write the ID
                dst.put_u32(id)
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{sink::SinkExt, stream::StreamExt};
    use tokio::io;
    use tokio_util::codec;

    #[tokio::test]
    async fn can_send_ping_frame() {
        let (client, server) = io::duplex(1024);

        let mut client_sink = codec::Framed::new(client, ControlCodec::new());
        let mut server_stream = codec::Framed::new(server, ControlCodec::new());

        let ping_frame = ControlFrame::Ping(1);
        client_sink.send(ping_frame).await.unwrap();
        let received_frame = server_stream.next().await.unwrap().unwrap();
        // We don't really want to implement PartialEq just for this.
        match received_frame {
            ControlFrame::Ping(1) => (),
            _ => panic!("Received frame is not a Ping frame with ID 1"),
        }
    }
}
