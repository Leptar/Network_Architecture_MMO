use bevy::prelude::*;
use crate::quadtree::QuadTree;

#[derive(Component)]
pub struct ClientId(pub u32);

#[derive(Component)]
pub struct Position(pub Vec2);

#[derive(Component)]
pub struct CurrentShard(pub Option<u32>);

