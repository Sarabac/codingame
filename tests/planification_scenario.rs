use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use codingame::common::*;
use codingame::ligue1::{ai::*, atome::*, decision::*};
use rand::SeedableRng;
use random_testing::{random_testing, RandomTestingResult};
use verification::*;

mod verification;

/**
 * -----------
 * | | | | | |
 * -----------
 * | | | | | |
 * -----------
 * |r|b|b|h|A|
 * -----------
 * | | | | | |
 * -----------
 * | | | | | |
 * -----------
 */
#[test]
fn va_harvester() {
    let state = Rc::new(StateBuilder::new_a_gauche_prot_a_a_droite().build());
    let mut managing = Managing::new()
        .with_rng(rand::rngs::StdRng::seed_from_u64(81))
        .with_nb_max_iteration(3);
    let plan = planifier(state, &mut managing);
    let grow1 = Grow {
        coord: Coord { x: 1, y: 2 },
        direction: Direction::N,
        parent_id: Id::new(0),
        organe_type: OrganeType::Basic,
    };
    let grow2 = Grow {
        coord: Coord { x: 2, y: 2 },
        direction: Direction::N,
        parent_id: Id::new(1),
        organe_type: OrganeType::Basic,
    };
    let grow3 = Grow {
        coord: Coord { x: 3, y: 2 },
        direction: Direction::E,
        parent_id: Id::new(2),
        organe_type: OrganeType::Harvester,
    };
    let expected = Planification::default()
        .add_decision(Decision::Grow(grow1))
        .new_turn()
        .add_decision(Decision::Grow(grow2))
        .new_turn()
        .add_decision(Decision::Grow(grow3))
        .new_turn();

    assert_eq!(plan, expected);
    assert_eq!(
        plan.take_first_turn().into_iter().next(),
        Some(Decision::Grow(grow1))
    )
}

fn simple_va_harvester(rng: u64) -> Result<(), Box<dyn Debug>> {
    // GIVEN
    let builder = StateBuilder::new_ligne_de_3_root_a_gauche()
        .with_ressources_ami(Ressource::new(0, 0, 1, 1))
        .add_cell(Cell {
            coord: Coord { x: 2, y: 0 },
            entity: Entity::Protein(Protein::A),
        });
    let state = Rc::new(builder.build());
    let mut managing = Managing::new()
        .with_rng(rand::rngs::StdRng::seed_from_u64(rng))
        .with_nb_max_iteration(3);

    // THEN
    let planif = planifier(state, &mut managing);

    // WHEN
    PlanificationChecker::default()
        .then_grow(
            GrowChecker::default()
                .coord(Coord { x: 1, y: 0 })
                .direction(Direction::E)
                .organ_type(OrganeType::Harvester),
        )
        .then_grow(
            GrowChecker::default()
                .coord(Coord { x: 2, y: 0 })
                .organ_type(OrganeType::Basic),
        )
        .then_wait()
        .then_finis()
        .verify(planif)
}

#[test]
fn simple_va_harvester_repeated() -> RandomTestingResult {
    random_testing(simple_va_harvester, 1000, 200)
}

fn va_attaquer(rng: u64) -> Result<(), Box<dyn Debug>> {
    // GIVEN
    let friend = OrganismBuilder::default().build(Owner::Me, Coord { x: 0, y: 1 });
    let ennemy = OrganismBuilder::default()
    .add_basic(Direction::E)
    .add_basic(Direction::E)
    .add_basic(Direction::N)
    .add_basic(Direction::N)
    .add_basic(Direction::W)
    .add_basic(Direction::W)
    .build(Owner::Ennemy, Coord { x: 0, y: 2 });
    let builder = StateBuilder::carre_vide_3()
        .with_ressources_ami(Ressource::new(50, 1, 1, 0))
        .add_cells(friend)
        .add_cells(ennemy);
    let state = Rc::new(builder.build());
    let mut managing = Managing::new()
        .with_rng(rand::rngs::StdRng::seed_from_u64(rng))
        .with_nb_to_choose(50)
        .with_nb_max_iteration(6);

    // THEN
    let planif = planifier(state, &mut managing);

    // WHEN
    PlanificationChecker::default()
        .then_grow(
            GrowChecker::default()
                .coord(Coord { x: 1, y: 1 })
                .direction(Direction::S)
                .organ_type(OrganeType::Tentacle)
        )
        .then_grow(GrowChecker::default().organ_type(OrganeType::Basic))
        .then_grow(GrowChecker::default().organ_type(OrganeType::Basic))
        .then_grow(GrowChecker::default().organ_type(OrganeType::Basic))
        .then_grow(GrowChecker::default().organ_type(OrganeType::Basic))
        .then_grow(GrowChecker::default().organ_type(OrganeType::Basic))
        .then_finis()
        .verify(planif)
}

#[test]
fn va_attaquer_repeated() -> RandomTestingResult {
    random_testing(va_attaquer, 100, 10)
}