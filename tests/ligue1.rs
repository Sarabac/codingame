use std::rc::Rc;

use codingame::ligue1::{base_objects::*, main_objects::*, ai::*};


#[test]
fn count_harvesting() {
    let dimension =Dimension{height: 3, width: 3};
    let ressources_ami = Ressource::new(1, 1, 1, 1);
    let ressources_ennemy = Ressource::new(1, 1, 1, 1);
    let action_count = ActionCount::new(1);
    let cells = vec![
        Cell{coord: Coord{x:0, y:0}, entity: Entity::Organe(Organe{dir: Direction::N, id: Id::new(0), parent_id: Id::new(0), root_id: Id::new(0), organe_type: OrganeType::Root, owner: Owner::Me})},
        Cell{coord: Coord{x:1, y:0}, entity: Entity::Organe(Organe{dir: Direction::E, id: Id::new(1), parent_id: Id::new(0), root_id: Id::new(0), organe_type: OrganeType::Harvester, owner: Owner::Me})},
        Cell{coord: Coord{x:2, y:0}, entity: Entity::Protein(Protein::A)},
        Cell{coord: Coord{x:2, y:2}, entity: Entity::Protein(Protein::B)},
    ];
    let state = GameState::new(dimension, ressources_ami, ressources_ennemy, action_count, cells);
    assert_eq!(state.harvesting(), vec![Protein::A]);
}

#[test]
fn harvesting_is_best() {
    let dimension = Dimension{height: 3, width: 3};
    let ressources_ami = Ressource::new(1, 1, 1, 1);
    let ressources_ennemy = Ressource::new(1, 1, 1, 1);
    let action_count = ActionCount::new(1);
    let cells = vec![
        Cell{coord: Coord{x:0, y:0}, entity: Entity::Organe(Organe{dir: Direction::N, id: Id::new(0), parent_id: Id::new(0), root_id: Id::new(0), organe_type: OrganeType::Root, owner: Owner::Me})},
        Cell{coord: Coord{x:2, y:0}, entity: Entity::Protein(Protein::A)},
    ];
    let state = GameState::new(dimension, ressources_ami, ressources_ennemy, action_count, cells);
    let decision = generer(Rc::new(state));
    assert_eq!(decision, Decision::Grow(Id::new(0), Coord { x: 1, y: 0 }, OrganeType::Harvester, Direction::E));
}

#[test]
fn no_harvesting_when_no_prot() {
    let dimension = Dimension{height: 3, width: 3};
    let ressources_ami = Ressource::new(1, 1, 1, 1);
    let ressources_ennemy = Ressource::new(1, 1, 1, 1);
    let action_count = ActionCount::new(1);
    let cells = vec![
        Cell{coord: Coord{x:0, y:0}, entity: Entity::Organe(Organe{dir: Direction::N, id: Id::new(0), parent_id: Id::new(0), root_id: Id::new(0), organe_type: OrganeType::Root, owner: Owner::Me})},
        Cell{coord: Coord{x:2, y:2}, entity: Entity::Protein(Protein::A)},
    ];
    let state = GameState::new(dimension, ressources_ami, ressources_ennemy, action_count, cells);
    let decision = generer(Rc::new(state));
    let Decision::Grow(_, _, actual, _) = decision else {
        panic!("pas de grow")
    };
    assert_eq!(actual, OrganeType::Basic);
}