use bevy::prelude::*;
use crate::components::*;
use crate::quadtree::QuadTree;

pub(crate) fn setup_simulation(mut commands: Commands) {
    // A. Insertion de la Ressource QuadTree globale
    // On recrée l'arbre simple de 100x100 (le même que dans le test)
    let tree = QuadTree {
        bounds: Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)),
        depth: 0,
        max_depth: 1,
        shard_id: None,
        children: Some(Box::new([
            QuadTree { bounds: Rect::from_corners(Vec2::new(0.0, 50.0), Vec2::new(50.0, 100.0)), depth: 1, max_depth: 1, children: None, shard_id: Some(1) },
            QuadTree { bounds: Rect::from_corners(Vec2::new(50.0, 50.0), Vec2::new(100.0, 100.0)), depth: 1, max_depth: 1, children: None, shard_id: Some(2) },
            QuadTree { bounds: Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(50.0, 50.0)), depth: 1, max_depth: 1, children: None, shard_id: Some(3) },
            QuadTree { bounds: Rect::from_corners(Vec2::new(50.0, 0.0), Vec2::new(100.0, 100.0)), depth: 1, max_depth: 1, children: None, shard_id: Some(4) },
        ])),
    };
    commands.insert_resource(tree);

    // B. Création de l'entité joueur
    // Il commence à gauche (x=10.0), en haut (y=75.0) -> Shard 1
    commands.spawn((
        ClientId(999),
        Position(Vec2::new(10.0, 75.0)),
        CurrentShard(None), // Aucun shard au tout début
    ));
    println!("Simulation démarrée. Joueur instancié en x: 10.0, y: 75.0");
}

pub(crate) fn move_fake_player(mut query: Query<&mut Position, With<ClientId>>) {
    for mut pos in query.iter_mut() {
        // Le joueur avance de 0.5 unités vers la droite à chaque tick
        pos.0.x += 0.5;

        // On affiche sa position de temps en temps pour suivre
        if pos.0.x % 10.0 == 0.0 {
            println!("Le joueur est maintenant en x: {}", pos.0.x);
        }
    }
}