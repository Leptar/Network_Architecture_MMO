use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer};
use crate::quadtree::QuadTree;

#[derive(Component)]
pub struct ClientId(pub u32);

#[derive(Component)]
pub struct Position(pub Vec2);

#[derive(Component)]
pub struct CurrentShard(pub Option<u32>);

#[derive(Component)]
pub struct NearbyShards(pub Vec<u32>);

#[derive(Resource)]
pub struct SpatialSocket {
    pub peer: GamePeer,
    pub broker_conn: GameConnection,
}