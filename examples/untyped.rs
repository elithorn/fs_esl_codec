use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead};
use futures::stream::StreamExt;

use fs_esl_codec::EslCodec;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // Open socket
    let listen = std::env::args().nth(1).expect("Expected SockAddr of server");
    let mut stream = TcpStream::connect(&listen).await.unwrap();

    // Authorise against an ESL
    let pass = std::env::args().nth(2).expect("Expected ESL auth pass");
    let message = format!("auth {}\n\n", &pass);
    stream.write_all(message.as_bytes()).await.unwrap();

    // Subscribe to all types of events in json format
    let message = b"event json ALL\n\n";
    stream.write_all(message).await.unwrap();

    // Instantiate ESL parser
    let in_codec = EslCodec::new();

    // Create tokio framedreader using EslCodec
    let mut framed_read = FramedRead::new(stream, in_codec);

    // Read freeswitch-emitted messages one-by-one
    while let Some(Ok(data)) = framed_read.next().await {

        // Decode into a hashmap of string key-value pairs
        if let Ok(event_data) = serde_json::from_str::<HashMap<String,String>>(&data.payload.unwrap_or_default()) {

            match event_data.get("Event-Name").map(|v| v.as_ref()) {

                // First inbound INVITE received
                Some("CHANNEL_CREATE") if event_data.get("Call-Direction").map(|v| v.as_ref()) == Some("inbound") => {
                    println!("[{}] <{}> new incoming [{}] {}", 
                             event_data.get("Event-Date-GMT").unwrap(),
                             event_data.get("Channel-Call-UUID").unwrap(),
                             event_data.get("Call-Direction").unwrap(),
                             event_data.get("Channel-Name").unwrap(),
                             );
                },

                // leg b originated
                Some("CHANNEL_OUTGOING") => {
                    println!("[{}] <{}> trying   [{}] {} for {} {}", 
                             event_data.get("Event-Date-GMT").unwrap(),
                             event_data.get("Channel-Call-UUID").unwrap(),
                             event_data.get("Call-Direction").unwrap(),
                             event_data.get("Channel-Name").unwrap(),
                             event_data.get("Caller-Caller-ID-Name").unwrap(),
                             event_data.get("Caller-Caller-ID-Number").unwrap(),
                             );
                },


                // leg b answered
                Some("CHANNEL_ANSWER") => {
                    println!("[{}] <{}> answered [{}] {} for {} {}", 
                             event_data.get("Event-Date-GMT").unwrap(),
                             event_data.get("Channel-Call-UUID").unwrap(),
                             event_data.get("Call-Direction").unwrap(),
                             event_data.get("Channel-Name").unwrap(),
                             event_data.get("Caller-Caller-ID-Name").unwrap(),
                             event_data.get("Caller-Caller-ID-Number").unwrap(),
                             );
                },

                // Leg a bridged to leg b
                Some("CHANNEL_BRIDGE") => {
                    println!("[{}] <{}> bridge   [{}] {} for {}{}", 
                             event_data.get("Event-Date-GMT").unwrap(),
                             event_data.get("Channel-Call-UUID").unwrap(),
                             event_data.get("Call-Direction").unwrap(),
                             event_data.get("Channel-Name").unwrap(),
                             event_data.get("Caller-Caller-ID-Name").unwrap(),
                             event_data.get("Caller-Caller-ID-Number").unwrap(),
                             );
                },

                // Leg b hangup
                Some("CHANNEL_HANGUP_COMPLETE") => {
                    println!("[{}] <{}> hangup   [{}] {}", 
                             event_data.get("Event-Date-GMT").unwrap(),
                             event_data.get("Channel-Call-UUID").unwrap(),
                             event_data.get("Call-Direction").unwrap(),
                             event_data.get("Channel-Name").unwrap(),
                             );
                },

                // Periodic stats updates (good for prometheus metrics or smth)
                Some("HEARTBEAT") => {
                    println!("[{}] STAT {}", 
                             event_data.get("Event-Date-GMT").unwrap(),
                             event_data.get("Up-Time").unwrap(),
                             );
                },

                // mod Sofia and other events
                Some("CUSTOM") => match event_data.get("Event-Subclass").map(|v| v.as_str()) {

                    Some("sofia::gateway_state") => 
                        println!("[{}] Trunk {} (ping={}) changed state to {} with status {}",
                            event_data.get("Event-Date-GMT").unwrap(),
                            event_data.get("Gateway").unwrap(),
                            event_data.get("Ping-Status").unwrap(),
                            event_data.get("State").unwrap(),
                            event_data.get("Status").map_or("", |v| v)
                        ),

                    _ => {  }

                },

                _ => { }
            }
        }
    }

    Ok(())
}
