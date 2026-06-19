use bevy::prelude::*;

#[derive(Message, Debug)]
pub struct SubscribeMessage {
    pub client_id: u32,
    pub topic: u32,
}

#[derive(Message, Debug)]
pub struct UnsubscribeMessage {
    pub client_id: u32,
    pub topic: u32,
}

#[derive(Message, Debug)]
pub struct CrossingAlertMessage {
    pub client_id: u32,
    pub source_shard: u32,
    pub involved_shards: Vec<u32>,
}

#[derive(Message, Debug)]
pub struct BootShardEvent {
    pub shard_id: u32,
    pub bounds: Rect,
}