use bevy::prelude::{Changed, Query, Res};
use crate::components::{CurrentShard, Position};
use crate::quadtree::QuadTree;

pub(crate) fn check_shard_transitions(
    mut query: Query<(&Position, &mut CurrentShard), Changed<Position>>,
    quad_tree: Res<QuadTree>)
{

    for (position, mut currentShard) in query.iter_mut() {
        let actual_shard = quad_tree.shard_for(position.0);

        if actual_shard != currentShard.0{
            if let Some(ancien_id) = currentShard.0 {
                println!("Unsubscribe du shard: {}", ancien_id.to_string());
            }

            if let Some(new_Id) = actual_shard {
                println!("Subscribe du shard: {}", new_Id.to_string());
            }

            currentShard.0 = actual_shard;
        }

        let shard_near = quad_tree.shards_near(position.0, 5.0 ); //Nombre pas defini

        if shard_near.len() > 1 {
            println!("Alerte de frontière franchie pour le joueur ! Shards impliqués : {:?}", shard_near);
        }
    }
}