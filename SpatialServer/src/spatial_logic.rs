use bevy::prelude::*;
use bevy::ecs::event::Event;
use crate::components::{CurrentShard, Position, ClientId, NearbyShards};
use crate::messages::{CrossingAlertMessage, SubscribeMessage, UnsubscribeMessage};
use crate::quadtree::QuadTree;

pub(crate) fn check_shard_transitions(
    mut query: Query<(&ClientId, &Position, &mut CurrentShard, &mut NearbyShards), Changed<Position>>,
    quad_tree: Res<QuadTree>,
    mut sub_evts: MessageWriter<SubscribeMessage>,
    mut unsub_evts: MessageWriter<UnsubscribeMessage>,
    mut alert_evts: MessageWriter<CrossingAlertMessage>,
)
{
    for (client_id, position, mut current_shard, mut nearby_shards) in query.iter_mut() {
        let actual_shard = quad_tree.shard_for(position.0);

        if actual_shard != current_shard.0{
            if let Some(ancien_id) = current_shard.0 {
                unsub_evts.write(UnsubscribeMessage {
                    client_id: client_id.0,
                    topic: ancien_id,
                });
                println!("Unsubscribe du shard: {}", ancien_id.to_string());
            }

            if let Some(new_Id) = actual_shard {
                sub_evts.write(SubscribeMessage {
                    client_id: client_id.0,
                    topic: new_Id,
                });
                println!("Subscribe du shard: {}", new_Id.to_string());
            }

            current_shard.0 = actual_shard;
        }
        if let Some(source_shard) = current_shard.0 {

            let current_shards_near = quad_tree.shards_near(position.0, 5.0);

            // Je n'envoie crossing alert que si les shards near est différent du dernier envoyé
            if current_shards_near != nearby_shards.0 {
                if current_shards_near.len() > 1 {
                    alert_evts.write(CrossingAlertMessage {
                        client_id: client_id.0,
                        source_shard,
                        involved_shards: current_shards_near.clone(),
                    });
                }

                nearby_shards.0 = current_shards_near;
            }
        }
    }
}