# ToDo

- when receiving frames from the MQTT subscription, check the MAC isn't the TUN device (prevent spamming self)
- make a nice container to run this in
- accept args on the command line for configuration elements
- allow use of existing TUN device, rather than creating one each time
- send MQTT payloads without base64 encoding
