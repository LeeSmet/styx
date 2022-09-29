use etherparse::{ether_type, EtherType};
use std::{error::Error, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_tun::TunBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // See if a target is set on the cmd line
    let target = std::env::args().skip(1).next();
    // Create a listener on all interfaces, fixed port for now.
    let listener = TcpListener::bind("[::]:9651").await?;
    // TODO: Investigate if MQ is a better approach to get multiple handles to the same device
    // instead of splitting it later.
    let iface = Arc::new(
        TunBuilder::new()
            .name("styx")
            .tap(false)
            .mtu(1420)
            .packet_info(false)
            .up()
            .try_build()?,
    );

    tokio::spawn({
        let iface = iface.clone();
        async move {
            loop {
                // Accept new connections.
                let (con, _) = listener.accept().await.unwrap();
                let (mut reader, mut writer) = con.into_split();
                let iface_read = iface.clone();
                let iface_write = iface.clone();
                tokio::spawn(async move {
                    let mut buf = [0; 65535];
                    loop {
                        let n = iface_read.recv(&mut buf).await.unwrap();
                        let mut s = 0;
                        while s < n {
                            s += writer.write(&buf[s..n]).await.unwrap();
                        }
                    }
                });
                tokio::spawn(async move {
                    let mut buf = [0; 65535];
                    loop {
                        let n = reader.read(&mut buf).await.unwrap();
                        let mut s = 0;
                        while s < n {
                            s += iface_write.send(&buf[s..n]).await.unwrap();
                        }
                    }
                });
            }
        }
    });

    // If we set a target, connect to it.
    if let Some(target) = target {
        tokio::task::spawn(async move {
            let con = TcpStream::connect(target).await.unwrap();
            let (mut reader, mut writer) = con.into_split();
            let iface_read = iface.clone();
            let iface_write = iface.clone();
            tokio::spawn(async move {
                let mut buf = [0; 65535];
                loop {
                    let n = iface_read.recv(&mut buf).await.unwrap();
                    let mut s = 0;
                    while s < n {
                        s += writer.write(&buf[s..n]).await.unwrap();
                    }
                }
            });
            tokio::spawn(async move {
                let mut buf = [0; 65535];
                loop {
                    let n = reader.read(&mut buf).await.unwrap();
                    let mut s = 0;
                    while s < n {
                        s += iface_write.send(&buf[s..n]).await.unwrap();
                    }
                }
            });
        });
    };

    tokio::time::sleep(std::time::Duration::from_secs(60 * 60 * 24)).await;

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
