use std::{collections::HashMap, rc::Rc};

use codingame::ligue1::{ai::*, atome::*, decision::*, molecule::*, state::*};
use rand::SeedableRng;

#[test]
fn count_harvesting() {
    let dimension = Dimension {
        height: 3,
        width: 3,
    };
    let ressources_ami = Ressource::new(1, 1, 1, 1);
    let ressources_ennemy = Ressource::new(1, 1, 1, 1);
    let action_count = ActionCount::new(1);
    let cells = vec![
        Cell {
            coord: Coord { x: 0, y: 0 },
            entity: Entity::Organe(Organe {
                dir: Direction::N,
                id: Id::new(0),
                parent_id: Id::new(0),
                root_id: Id::new(0),
                organe_type: OrganeType::Root,
                owner: Owner::Me,
            }),
        },
        Cell {
            coord: Coord { x: 1, y: 0 },
            entity: Entity::Organe(Organe {
                dir: Direction::E,
                id: Id::new(1),
                parent_id: Id::new(0),
                root_id: Id::new(0),
                organe_type: OrganeType::Harvester,
                owner: Owner::Me,
            }),
        },
        Cell {
            coord: Coord { x: 2, y: 0 },
            entity: Entity::Protein(Protein::A),
        },
        Cell {
            coord: Coord { x: 2, y: 2 },
            entity: Entity::Protein(Protein::B),
        },
    ];
    let state = InitState::new(
        dimension,
        ressources_ami,
        ressources_ennemy,
        action_count,
        cells,
    );
    let mut expected: CoordMap<Harvesting> = HashMap::new();
    expected.insert(
        Coord { x: 2, y: 0 },
        Harvesting {
            protein: Protein::A,
            direction: Direction::E,
            harvester_coord: Coord { x: 1, y: 0 },
        },
    );
    assert_eq!(state.harvesting(), expected);
}

#[test]
fn no_harvesting_when_no_prot() {
    let dimension = Dimension {
        height: 3,
        width: 3,
    };
    let ressources_ami = Ressource::new(1, 0, 0, 1);
    let ressources_ennemy = Ressource::new(1, 1, 1, 1);
    let action_count = ActionCount::new(1);
    let cells = vec![
        Cell {
            coord: Coord { x: 0, y: 0 },
            entity: Entity::Organe(Organe {
                dir: Direction::N,
                id: Id::new(0),
                parent_id: Id::new(0),
                root_id: Id::new(0),
                organe_type: OrganeType::Root,
                owner: Owner::Me,
            }),
        },
        Cell {
            coord: Coord { x: 2, y: 2 },
            entity: Entity::Protein(Protein::A),
        },
    ];
    let state = InitState::new(
        dimension,
        ressources_ami,
        ressources_ennemy,
        action_count,
        cells,
    );
    let mut managing = Managing::new()
        .with_rng(rand::rngs::StdRng::seed_from_u64(62))
        .with_nb_max_iteration(1);
    let decision = planifier(Rc::new(state), &mut managing)
        .take_first_turn()
        .into_iter()
        .next()
        .unwrap_or_default();
    let Decision::Grow(Grow { organe_type, .. }) = decision else {
        panic!("pas de grow")
    };
    assert_eq!(organe_type, OrganeType::Basic);
}

#[test]
fn bonnes_dimensions() {
    let dimension = Dimension {
        width: 4,
        height: 2,
    };
    let state = InitState::new(
        dimension,
        Ressource::default(),
        Ressource::default(),
        ActionCount::new(1),
        vec![],
    );

    let coo1 = Coord { x: 3, y: 1 };
    assert_eq!(
        state.get_by_coord(coo1),
        Some(Cell {
            coord: coo1,
            entity: Entity::Void
        })
    );

    let coo2 = Coord { x: 1, y: 3 };
    assert_eq!(state.get_by_coord(coo2), None);
}
