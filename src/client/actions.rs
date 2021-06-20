use serde_json::json;

use crate::errors::Result;
use crate::packet::{raw::RawPacket, Operation, Protocol};
use crate::Client;

impl Client {
    pub(crate) async fn enter_room(&self) -> Result<()> {
        let req = json!({
            "uid": self.uid,
            "roomid": self.room_id,
            "protover": 2,
            "platform": "web",
            "clientver": "1.8.2",
            "type": 2,
            "key": self.token
        });

        // TODO buffer proto
        let pack = RawPacket::new(
            Operation::RoomEnter,
            Protocol::Json,
            serde_json::to_vec(&req).unwrap(),
        );
        println!("sending room enter package");
        self.send_raw(pack).await?;
        println!("room enter package sent");
        Ok(())
    }
}