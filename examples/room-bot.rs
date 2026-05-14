use anyhow::Result;
use kaya_sdk::{KayaClient, KayaConfig, KayaEvent};

#[tokio::main]
async fn main() -> Result<()> {
    let room = std::env::args().nth(1).unwrap_or_else(|| "geral".into());
    let client = KayaClient::new(KayaConfig::default()).await?;
    client.set_callsign("room-bot").await?;
    client.join_room(&room).await?;

    let mut events = client.subscribe_events();
    println!("room-bot listening in #{room}");

    loop {
        match events.recv().await? {
            KayaEvent::RoomMessageReceived {
                room: event_room,
                from_callsign,
                body,
                local,
                ..
            } if !local && event_room == room => {
                let reply = format!("ack {from_callsign}: {body}");
                client.send_room_message(&room, &reply).await?;
            }
            _ => {}
        }
    }
}
