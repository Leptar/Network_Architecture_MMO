use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer};

#[derive(Resource, Default)]
pub struct PlayerEntities {
    pub map: HashMap<u32, Entity>,
}
#[derive(Component)]
pub struct ClientId(pub u32);

#[derive(Component)]
pub struct Position(pub Vec2);

#[derive(Component)]
pub struct CurrentShard(pub Option<u32>);

#[derive(Component)]
pub struct NearbyShards(pub Vec<u32>);

#[derive(Resource, Default)]
pub struct BootQueue(pub Vec<(u32, Rect)>);
#[derive(Resource)]
pub struct SpatialSocket {
    pub peer: GamePeer,
    pub broker_conn: Option<GameConnection>,
    pub has_booted: bool,
}