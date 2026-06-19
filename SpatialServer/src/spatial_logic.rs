use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use crate::components::{CurrentShard, Position, ClientId, NearbyShards};
use crate::messages::{BootShardMessage, CrossingAlertMessage, SubscribeMessage, UnsubscribeMessage};
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

            if let Some(new_id) = actual_shard {
                sub_evts.write(SubscribeMessage {
                    client_id: client_id.0,
                    topic: new_id,
                });
                println!("Subscribe du shard: {}", new_id.to_string());
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

// STUB : ptetre def le nombre max dans lib
const MAX_PLAYERS_PER_SHARD: usize = 1000;

pub fn monitor_shard_capacity(
    mut quad_tree: ResMut<QuadTree>,
    query: Query<&CurrentShard>,
    mut boot_evts: MessageWriter<BootShardMessage>,
) {
    // dictionnaire pour faire le décompte
    let mut shard_counts: HashMap<u32, usize> = HashMap::new();

    // joueurs du serveur spatial
    for current_shard in query.iter() {
        if let Some(shard_id) = current_shard.0 {
            *shard_counts.entry(shard_id).or_insert(0) += 1;
        }
    }

    // Calcule un ID libre à partir de l'état actuel de l'arbre (évite les collisions)
    let mut next_available_id = quad_tree
        .get_leaves()
        .into_iter()
        .map(|(id, _)| id)
        .max()
        .unwrap_or(0)
        + 1;
    for (shard_id, count) in shard_counts {
        if count >= MAX_PLAYERS_PER_SHARD {
            println!("SURCHARGE DÉTECTÉE : Le Shard {} a {} joueurs (Max: {})", shard_id, count, MAX_PLAYERS_PER_SHARD);

            if let Some(new_shards) = quad_tree.split_shard(shard_id, &mut next_available_id) {
                println!("QuadTree mis à jour. Nouveaux Shards à lancer :");
                for (new_id, bounds) in new_shards {
                    println!("Shard {} -> Zone: {:?}", new_id, bounds);

                    boot_evts.write(BootShardMessage {
                        shard_id: new_id,
                        bounds,
                    });
                }
            }
        }
    }
}