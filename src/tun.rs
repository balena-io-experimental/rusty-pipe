use mac_address::{mac_address_by_name, MacAddress};
use std::error::Error;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use tun_tap_mac::{Iface, Mode};

/// Run a shell command. Panic if it fails in any way.
fn cmd(cmd: &str, args: &[&str]) {
    let ecode = Command::new("ip")
        .args(args)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    assert!(ecode.success(), "Failed to execte {}", cmd);
}

pub struct TunIface {
    pub iface: tun_tap_mac::Iface,
    pub eth_mac_addr: MacAddress,
}

impl TunIface {
    pub fn create(cidr: &str) -> TunIface {
        // Create the tun interface.
        let iface = Iface::new("rusty%d", Mode::Tap).unwrap();

        // Get our local MAC address.
        let mac_addr = match mac_address_by_name(iface.name()) {
            Ok(Some(addr)) => addr,
            Ok(None) => panic!("Unable to find the MAC address for {}", iface.name()),
            Err(e) => panic!(e),
        };

        // Configure the „local“ (kernel) endpoint.
        cmd("ip", &["addr", "add", "dev", iface.name(), cidr]);
        cmd("ip", &["link", "set", "up", "dev", iface.name()]);

        TunIface {
            iface: iface,
            eth_mac_addr: mac_addr,
        }
    }

    pub fn listen_for_frames(
        self,
        send_to: crossbeam_channel::Sender<Vec<u8>>,
    ) -> std::result::Result<crossbeam_channel::Sender<Vec<u8>>, Box<Error>> {
        // That 1500 is a guess for the IFace's MTU (we probably could configure it explicitly). 4 more
        // for TUN's „header“.

        let iface = Arc::new(self.iface);
        let iface_writer = iface.clone();

        let (tun_tx, tun_rx) = crossbeam_channel::unbounded();

        thread::spawn(move || {
            let mut buffer = vec![0; 1504];
            loop {
                // every read is one packet. If the buffer is too small, bad luck, it gets truncated.
                let size = iface.recv(&mut buffer).expect("Error reading TAP");
                assert!(size >= 16);

                // send data to the caller...
                send_to
                    .send(buffer[..size].to_vec())
                    .expect("Error sending TAP to MQTT")
            }
        });

        thread::spawn(move || loop {
            select! {
                recv(tun_rx) -> frame => {
                    let frame: Result<std::vec::Vec<u8>, crossbeam_channel::RecvError> = frame;
                    match frame {
                        Ok(frame) => {
                            println!("VPN :: Dest: {:X?}, Src: {:X?}", &frame[4..10], &frame[10..16]);
                            let f = &frame[..];
                            match iface_writer.send(f) {
                                _ => {}
                            }
                        },
                        Err(e) => panic!(e),
                    }
                }
            }
        });

        Ok(tun_tx)
    }
}
