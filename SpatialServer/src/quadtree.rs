use bevy::prelude::*;

#[derive(Resource)]
pub struct QuadTree {
    pub bounds: Rect,
    pub depth: u8,
    pub max_depth: u8,
    pub children: Option<Box<[QuadTree; 4]>>,
    pub shard_id: Option<u32>,
}

impl QuadTree {
    /// Retourne le shard_id de la feuille contenant `pos`.
    pub fn shard_for(&self, pos: Vec2) -> Option<u32> {
        if !self.bounds.contains(pos) {
            return None;
        }

        // recursif
        if let Some(children) = &self.children {
            for child in children.iter() {
                if let Some(id) = child.shard_for(pos) {
                    return Some(id);
                }
            }
        }

        self.shard_id
    }

    /// Retourne les shard_ids distincts dans un rayon `margin` autour de `pos`.
    pub fn shards_near(&self, pos: Vec2, margin: f32) -> Vec<u32> {
        let mut result = Vec::new();

        // point le plus proche du rect
        let closest_x = pos.x.clamp(self.bounds.min.x, self.bounds.max.x);
        let closest_y = pos.y.clamp(self.bounds.min.y, self.bounds.max.y);

        // verifie si il touche la marge
        let distance_squared = Vec2::new(closest_x, closest_y).distance_squared(pos);
        if distance_squared > margin * margin {
            return result;
        }

        // touche la marge
        if let Some(children) = &self.children {
            for child in children.iter() {
                result.extend(child.shards_near(pos, margin));
            }
        }
        // si une feuille, on push l'id
        else if let Some(id) = self.shard_id {
            result.push(id);
        }

        // check le shard de pas l'avoir 2 fois
        result.sort();
        result.dedup();

        result
    }
}