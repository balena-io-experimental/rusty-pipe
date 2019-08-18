#[macro_use]
extern crate crossbeam_channel;
extern crate base64;
extern crate clap;
extern crate mac_address;
extern crate tun_tap_mac;

mod mqtt;
mod tun;

use clap::{App, Arg};
use crossbeam_channel::unbounded;
use mqtt::BackhaulClientBuilder;
use tun::TunIface;
use std::{thread};

fn main() {
    println!("rusty-pipe - the simple VPN");

    let matches = App::new("The simple VPN")
        .version("1.0")
        .author("Rich Bayliss <rich@balena.io>")
        .about("Uses MQTT to build a Layer2 VPN")
        .arg(
            Arg::with_name("CIDR")
                .short("c")
                .long("cidr")
                .help("Specify the IP address/subnet for this node")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("SEGMENT")
                .short("s")
                .long("segment")
                .help("Specify the ethernet segment for this node")
                .required(true)
                .takes_value(true),
        )        
        .get_matches();

    let cidr = matches
        .value_of("CIDR")
        .expect("You must provide a CIDR value");

    let segment = matches
        .value_of("SEGMENT")
        .expect("You must provide a SEGMENT value");

    let (mqtt_tx, mqtt_rx) = unbounded();
    let (tun_tx, tun_rx) = unbounded();

    let tap_iface = TunIface::create(&cidr);
    let mac_address = tap_iface.eth_mac_addr;
    let mac_addr_str = format!("{}", mac_address);

    let backhaul = BackhaulClientBuilder::new(
        "broker.hivemq.com".to_owned(),
        1883,
        mac_addr_str,
        segment.to_owned(),
    )
    .build();
    let backhaul_out = backhaul.listen_for_frames(mqtt_tx).unwrap();
    let iface_out = tap_iface.listen_for_frames(tun_tx).unwrap();

    println!("MAC: {:X?}", mac_address.bytes());
    thread::spawn(move || { 
        loop {
            select! {
                recv(mqtt_rx) -> msg => {
                    match msg {
                        Ok(msg) => {
                            // println!("MQTT :: Dest: {:X?}, Src: {:X?}", &msg[4..10], &msg[10..16]);
                            if &msg[10..16] != &mac_address.bytes()[0..6] {
                                match iface_out.send(msg) {
                                    _ => {}
                                }
                            }
                        },
                        Err(_) => {}
                    }
                }
            }
        }
    });

    thread::spawn(move || {   
        loop {
            select! {
                recv(tun_rx) -> msg => {
                    match msg {
                        Ok(msg) => {
                            // println!("TAP :: Dest: {:X?}, Src: {:X?}", &msg[4..10], &msg[10..16]);
                            if &msg[4..10] != &mac_address.bytes()[0..6] {
                                match backhaul_out.send(msg) {
                                    _ => {}
                                }
                            }
                        },
                        Err(_) => {}
                    }
                }
            }
        }
    });

    loop {}
}
