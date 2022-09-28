use etherparse::{ether_type, EtherType};
use std::error::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_tun::TunBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO: Investigate if MQ is a better approach to get multiple handles to the same device
    // instead of splitting it later.
    let mut iface = TunBuilder::new()
        .name("styx")
        .tap(false)
        .mtu(1420)
        .packet_info(false)
        .up()
        .try_build()?;

    let mut iface2 = TunBuilder::new()
        .name("styx2")
        .tap(false)
        .mtu(1420)
        .packet_info(false)
        .up()
        .try_build()?;
    let (mut read1, mut write1) = tokio::io::split(iface);
    let (mut read2, mut write2) = tokio::io::split(iface2);

    // NOTE: As it turns out any attempt to work with any kind of `tokio::io::copy` or similar
    // directly on the `Tun` or the split thereof returns an IO Error 22. Manual copy however does
    // work.
    tokio::spawn(async move {
        let mut buf = [0; 65535];
        loop {
            let n = read1.read(&mut buf).await.unwrap();
            write2.write(&mut buf[..n]).await.unwrap();
        }
    });
    tokio::spawn(async move {
        let mut buf = [0; 65535];
        loop {
            let n = read2.read(&mut buf).await.unwrap();
            write1.write(&mut buf[..n]).await.unwrap();
        }
    });
    // tokio::spawn(async move { tokio::io::copy(&mut read1, &mut write2).await.unwrap() });
    // tokio::spawn(async move { tokio::io::copy(&mut read2, &mut write1).await.unwrap() });
    // tokio::io::copy_bidirectional(&mut iface, &mut iface2)
    //     .await
    //     .unwrap();

    // let mut buf = [0; 1500];
    // loop {
    //     let n = iface.read(&mut buf).await?;
    //     // SAFETY: The unwrap is safe as we statically slice the buffer to 2 bytes.
    //     let flags = u16::from_be_bytes(buf[0..2].try_into().unwrap());
    //     // SAFETY: The unwrap is safe as we statically slice the buffer to 2 bytes.
    //     let ether_type = get_ether_type(u16::from_be_bytes(buf[2..4].try_into().unwrap())).unwrap();
    //     println!(
    //         "Read packet ({:?}), of size {} with flags {}",
    //         ether_type,
    //         n - 4,
    //         flags
    //     );
    // }

    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    Ok(())
}

fn get_ether_type(input: u16) -> Option<EtherType> {
    Some(match input {
        ether_type::IPV4 => EtherType::Ipv4,
        ether_type::IPV6 => EtherType::Ipv6,
        ether_type::ARP => EtherType::Arp,
        ether_type::WAKE_ON_LAN => EtherType::WakeOnLan,
        ether_type::VLAN_TAGGED_FRAME => EtherType::VlanTaggedFrame,
        ether_type::PROVIDER_BRIDGING => EtherType::ProviderBridging,
        ether_type::VLAN_DOUBLE_TAGGED_FRAME => EtherType::VlanDoubleTaggedFrame,
        _ => return None,
    })
}
