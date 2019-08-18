use rumqtt::{MqttClient, MqttOptions, QoS};
use std::{str, thread};

pub struct BackhaulClientBuilder {
    broker_addr: String,
    broker_port: u16,
    eth_mac_addr: String,
    eth_segment_addr: String,
}

impl BackhaulClientBuilder {
    pub fn new(
        broker_addr: String,
        broker_port: u16,
        eth_mac_addr: String,
        eth_segment_addr: String,
    ) -> BackhaulClientBuilder {
        BackhaulClientBuilder {
            broker_addr: broker_addr,
            broker_port: broker_port,
            eth_mac_addr: eth_mac_addr,
            eth_segment_addr: eth_segment_addr,
        }
    }

    pub fn build(&self) -> BackhaulClient {
        BackhaulClient {
            broker_addr: self.broker_addr.to_owned(),
            broker_port: self.broker_port,
            eth_mac_addr: self.eth_mac_addr.to_owned(),
            eth_segment_addr: self.eth_segment_addr.to_owned(),
        }
    }
}

pub struct BackhaulClient {
    broker_addr: String,
    broker_port: u16,
    eth_mac_addr: String,
    eth_segment_addr: String,
}

impl BackhaulClient {
    pub fn listen_for_frames(
        self,
        channel: crossbeam_channel::Sender<Vec<u8>>,
    ) -> Result<crossbeam_channel::Sender<Vec<u8>>, Box<std::error::Error>> {
        let mqtt_options = MqttOptions::new(
            format!("rusty-{}", &self.eth_mac_addr),
            self.broker_addr.to_string(),
            self.broker_port,
        );
        let (mut mqtt_client, notifications) = MqttClient::start(mqtt_options).unwrap();

        mqtt_client
            .subscribe(self.eth_segment_addr.to_string(), QoS::AtMostOnce)
            .unwrap();
        mqtt_client
            .subscribe(self.eth_mac_addr.to_string(), QoS::AtMostOnce)
            .unwrap();

        let (mqtt_tx, mqtt_rx) = crossbeam_channel::unbounded();

        thread::spawn(move || loop {
            select! {
                recv(notifications) -> notification => {
                    match notification {
                        Ok(n) => {
                            match n {
                                rumqtt::client::Notification::Publish(publish) => {
                                    let frame = match str::from_utf8(&publish.payload) {
                                        Ok(v) => {
                                            // println!("RAW: {:?}", v);

                                            match base64::decode(v) {
                                                Ok(f) => f,
                                                Err(e) => panic!("Invalid Base64 sequence: {}", e),
                                            }
                                        },
                                        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                                    };

                                    match channel.send(frame.to_owned()) {
                                        _ => {}
                                    }
                                },
                                _ => (),
                            };
                        },
                        Err(e) => panic!(e),
                    }
                }
            }
        });

        thread::spawn(move || loop {
            select! {
                recv(mqtt_rx) -> frame => {
                    let payload = base64::encode(&frame.unwrap());
                    mqtt_client.publish(self.eth_segment_addr.to_string(), QoS::AtMostOnce, false, payload).expect("Error publishing MQTT");
                }
            }
        });

        Ok(mqtt_tx)
    }
}
