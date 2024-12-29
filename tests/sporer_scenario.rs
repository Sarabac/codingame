use std::{fmt::Debug, rc::Rc};
mod verification;
use codingame::{
    common::*,
    ligue1::{ai::*, atome::*},
};
use rand::SeedableRng;
use verification::{random_testing::{random_testing, RandomTestingResult}, GrowChecker, PlanificationChecker};

fn creer_sporer_quand_seule_ressource_dispo(rng: u64) -> Result<(), Box<dyn Debug>> {
    let builder = StateBuilder::new_ligne_de_3_root_a_gauche()
        .with_ressources_ami(Ressource::new(0, 1, 0, 1));

    let state = Rc::new(builder.build());

    let mut managing = Managing::new()
        .with_rng(rand::rngs::StdRng::seed_from_u64(rng))
        .with_nb_to_choose(50)
        .with_nb_max_iteration(2);

    // THEN
    let planification = planifier(state, &mut managing);

    PlanificationChecker::default()
        .then_grow(
            GrowChecker::default()
                .coord(Coord { x: 1, y: 0 })
                .organ_type(OrganeType::Sporer),
        )
        .then_wait()
        .verify(planification)
}

#[test]
fn creer_sporer_quand_seule_ressource_dispo_repeated() -> RandomTestingResult {
    random_testing(creer_sporer_quand_seule_ressource_dispo, 100, 0)
}