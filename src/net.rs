/// Length of the unique part of a subnet.
pub const SUBNET_LENGTH: usize = 8;

/// Subnet used in the overlay, this is always a /64.
pub struct Subnet([u8; SUBNET_LENGTH]);
