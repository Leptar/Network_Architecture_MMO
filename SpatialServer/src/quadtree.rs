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

    /// Génère un arbre complet à partir d'une zone initiale
    pub fn generate(bounds: Rect, max_depth: u8) -> Self {
        let mut next_id = 1; // On commence à assigner au Shard 1
        Self::subdivide(bounds, 0, max_depth, &mut next_id)
    }

    /// division récursive interne
    /// Méthode que j'utilise au startup
    fn subdivide(bounds: Rect, depth: u8, max_depth: u8, next_id: &mut u32) -> Self {
        // Condition d'arrêt : on a atteint la profondeur voulue
        if depth == max_depth {
            let current_id = *next_id;
            *next_id += 1;
            return QuadTree {
                bounds,
                depth,
                max_depth,
                children: None,
                shard_id: Some(current_id),
            };
        }

        let center = bounds.center();

        let tl = Rect::from_corners(Vec2::new(bounds.min.x, center.y), Vec2::new(center.x, bounds.max.y));
        let tr = Rect::from_corners(center, bounds.max);
        let bl = Rect::from_corners(bounds.min, center);
        let br = Rect::from_corners(Vec2::new(center.x, bounds.min.y), Vec2::new(bounds.max.x, center.y));

        // Appel récursif
        let children = Box::new([
            Self::subdivide(tl, depth + 1, max_depth, next_id),
            Self::subdivide(tr, depth + 1, max_depth, next_id),
            Self::subdivide(bl, depth + 1, max_depth, next_id),
            Self::subdivide(br, depth + 1, max_depth, next_id),
        ]);

        QuadTree {
            bounds,
            depth,
            max_depth,
            children: Some(children),
            shard_id: None,
        }
    }

    /// Tente de diviser un shard existant en 4 nouveaux shards.
    /// Retourne la liste des nouveaux shards créés (ID et Zone) pour l'Orchestrator.
    /// Methode que j'utilise au runtime si un shard doit être subdivisé
    pub fn split_shard(&mut self, target_shard: u32, next_id: &mut u32) -> Option<Vec<(u32, Rect)>> {
        // Cas de base : on a trouvé la feuille à diviser
        println!("Tentative de division du Shard {}...", target_shard);
        
        if self.shard_id == Some(target_shard) {
            // On vérifie la limite de profondeur pour éviter un arbre infini
            if self.depth >= self.max_depth {
                println!("Impossible de diviser le shard {} : profondeur maximale atteinte.", target_shard);
                return None;
            }

            let center = self.bounds.center();
            let b = self.bounds;

            // Fonction utilitaire pour créer un enfant rapidement
            let mut create_child = |rect| {
                let id = *next_id;
                *next_id += 1;
                QuadTree {
                    bounds: rect,
                    depth: self.depth + 1,
                    max_depth: self.max_depth,
                    children: None,
                    shard_id: Some(id),
                }
            };

            // Création des 4 quadrants
            let tl = create_child(Rect::from_corners(Vec2::new(b.min.x, center.y), Vec2::new(center.x, b.max.y)));
            let tr = create_child(Rect::from_corners(center, b.max));
            let bl = create_child(Rect::from_corners(b.min, center));
            let br = create_child(Rect::from_corners(Vec2::new(center.x, b.min.y), Vec2::new(b.max.x, center.y)));

            // L'ancien shard devient un noeud parent (il perd son autorité)
            self.shard_id = None;
            self.children = Some(Box::new([tl, tr, bl, br]));

            // On retourne les infos des 4 nouveaux shards (pour prévenir l'Orchestrator)
            if let Some(children) = &self.children {
                let mut new_leaves = Vec::new();
                for child in children.iter() {
                    new_leaves.push((child.shard_id.unwrap(), child.bounds));
                }
                return Some(new_leaves);
            }
        }

        // Appel récursif : si ce n'est pas cette feuille, on cherche dans les enfants
        if let Some(children) = &mut self.children {
            for child in children.iter_mut() {
                if let Some(new_shards) = child.split_shard(target_shard, next_id) {
                    return Some(new_shards); // On fait remonter le résultat
                }
            }
        }

        None // Shard non trouvé
    }


    /// Récupère la liste de tous les Shards générés (ID + Zone couverte)
    pub fn get_leaves(&self) -> Vec<(u32, Rect)> {
        let mut leaves = Vec::new();

        // feuille = add
        if let Some(id) = self.shard_id {
            leaves.push((id, self.bounds));
        }

        // child = recursif
        if let Some(children) = &self.children {
            for child in children.iter() {
                leaves.extend(child.get_leaves());
            }
        }

        leaves
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper pour créer un arbre de test très simple (100x100 divisé en 4)
    fn create_test_tree() -> QuadTree {
        QuadTree {
            bounds: Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)),
            depth: 0,
            max_depth: 1,
            shard_id: None,
            children: Some(Box::new([
                // Enfant 0 : Top-Left (Shard 1)
                QuadTree { bounds: Rect::from_corners(Vec2::new(0.0, 50.0), Vec2::new(50.0, 100.0)), depth: 1, max_depth: 1, children: None, shard_id: Some(1) },
                // Enfant 1 : Top-Right (Shard 2)
                QuadTree { bounds: Rect::from_corners(Vec2::new(50.0, 50.0), Vec2::new(100.0, 100.0)), depth: 1, max_depth: 1, children: None, shard_id: Some(2) },
                // Enfant 2 : Bottom-Left (Shard 3)
                QuadTree { bounds: Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(50.0, 50.0)), depth: 1, max_depth: 1, children: None, shard_id: Some(3) },
                // Enfant 3 : Bottom-Right (Shard 4)
                QuadTree { bounds: Rect::from_corners(Vec2::new(50.0, 0.0), Vec2::new(100.0, 50.0)), depth: 1, max_depth: 1, children: None, shard_id: Some(4) },
            ])),
        }
    }

    #[test]
    fn test_shard_for_resolves_correctly() {
        let tree = create_test_tree();
        // Vérification stricte : le point (25, 75) doit être dans le Shard 1
        assert_eq!(tree.shard_for(Vec2::new(25.0, 75.0)), Some(1));
        // Le point (75, 25) doit être dans le Shard 4
        assert_eq!(tree.shard_for(Vec2::new(75.0, 25.0)), Some(4));
    }

    #[test]
    fn test_shards_near_detects_boundaries() {
        let tree = create_test_tree();
        // Un joueur en (48, 50) avec une marge de 5.0 chevauche la ligne verticale du milieu (x=50)
        let near = tree.shards_near(Vec2::new(48.0, 75.0), 5.0);
        // Il devrait détecter les shards 1 et 2
        assert!(near.contains(&1));
        assert!(near.contains(&2));
        assert_eq!(near.len(), 2);
    }
}