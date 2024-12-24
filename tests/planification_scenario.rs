use std::rc::Rc;

use codingame::common::*;
use codingame::ligue1::{ai::*, atome::*, decision::*};

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
    let plan = planifier(state, 3);
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
