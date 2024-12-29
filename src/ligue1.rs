use ai::{make_decision, Managing};
use parsing::{parser_dimension, parser_tour};
use rand::prelude::*;

pub fn main() {
    let dimension = parser_dimension();
    let mut managing = Managing::new().with_rng(rand::rngs::StdRng::seed_from_u64(33));
    loop {
        managing.restart();
        let game_state = parser_tour(dimension);
        eprintln!("Game State: {}", &game_state);
        let decision = make_decision(game_state, &mut managing);
        println!("{}", decision.to_string());
        managing.next_turn();
    }
}

pub mod ai {
    use rand::prelude::*;
    use std::{
        ops::{Range, Sub},
        rc::Rc,
        time::{Duration, Instant},
    };

    use itertools::iproduct;

    use super::{atome::*, decision::*, molecule::*, state::*};

    pub fn make_decision(state: InitState, managing: &mut Managing) -> Decision {
        planifier(Rc::new(state), managing)
            .take_first_turn()
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    pub fn planifier(state: Rc<dyn State>, managing: &mut Managing) -> Planification {
        let mut states: Vec<WeightedState> = vec![WeightedState { state, weight: 1 }];
        for _i in managing.iterations() {
            let nouveau_tour = states.into_iter().map(|w| w.state).collect();
            let intermediaire: Vec<WeightedState> = realiser_tour(nouveau_tour)
                .into_iter()
                .map(|s| Rc::new(s) as Rc<dyn State>)
                .map(|s| juger(s))
                .collect();
            let nb_to_choose = managing.nb_to_choose();
            states = intermediaire
                .choose_multiple_weighted(managing.rng(), nb_to_choose, |w| w.weight)
                .expect("Erreur dans le choose")
                .cloned()
                .collect();

            if managing.is_finished() {
                break;
            };
        }

        states
            .choose_weighted(managing.rng(), |w| w.weight)
            .map(|s| s.state.planification())
            .unwrap_or_default()
    }

    fn realiser_tour(mut process: Vec<Rc<dyn State>>) -> Vec<EndTurn> {
        let mut retour: Vec<EndTurn> = Vec::new();
        while !process.is_empty() {
            let (finis, encore) = process
                .into_iter()
                .flat_map(generer_step)
                .partition(|s| s.action_count().is_null());
            process = encore;
            retour.extend(finis.into_iter().map(EndTurn::new));
        }
        retour
    }

    fn generer_step(state: Rc<dyn State>) -> impl Iterator<Item = Rc<dyn State>> {
        let wait_step = Rc::new(WaitStep::new(state.clone())) as Rc<dyn State>;
        state
            .fertile_coords()
            .into_iter()
            .flat_map(|(coord, Fertilite { parent_id })| generer_grow(coord, parent_id))
            .filter_map(move |grow| GrowStep::try_new(state.clone(), grow))
            .map(|s| Rc::new(s) as Rc<dyn State>)
            .chain(std::iter::once(wait_step))
    }

    fn generer_grow(coord: Coord, parent_id: Id) -> impl Iterator<Item = Grow> {
        iproduct!(
            [
                OrganeType::Basic,
                OrganeType::Harvester,
                OrganeType::Tentacle,
                OrganeType::Sporer
            ],
            Direction::all()
        )
        .map(move |(organe_type, direction)| Grow {
            parent_id,
            coord,
            organe_type,
            direction,
        })
    }

    pub fn juger(state: Rc<dyn State>) -> WeightedState {
        let planification = state.planification().take_content();
        let planifiaction_len = planification.len();
        let wait_le_plus_tard_possible: usize = state
            .planification()
            .take_content()
            .into_iter()
            .enumerate()
            .flat_map(|(i, v)| v.into_iter().map(move |d| (i, d)))
            .map(|(i, d)| match d {
                Decision::Wait if i > 5 => 0,
                Decision::Wait => i.abs_diff(planifiaction_len) * 5,
                _ => 0,
            })
            .sum();

        let nb_harvesting = state.harvesting().len();
        let note_nb_harvesting = nb_harvesting.min(3) * 4;

        let resources = state.ressource_ami();
        let note_resources = Protein::all()
            .into_iter()
            .filter(|p| resources.get(p) != 0)
            .count();
        let nb_ami = state.organes_amis().len() * 3;
        let nb_ennemi = state.organe_ennemy_location().len() * 3;

        let weight = u32::try_from(1 + note_nb_harvesting + note_resources + nb_ami)
            .unwrap_or(u32::MAX)
            .saturating_sub(u32::try_from(nb_ennemi).unwrap_or(u32::MAX))
            .saturating_add(5)
            .saturating_sub(u32::try_from(wait_le_plus_tard_possible).unwrap_or(u32::MAX))
            .clamp(0, u32::MAX);
        WeightedState { state, weight }
    }

    #[derive(Debug, Clone)]
    pub struct WeightedState {
        pub state: Rc<dyn State>,
        pub weight: u32,
    }

    pub struct Managing {
        debut: Instant,
        permier_tour_duree: Duration,
        tours_suivant_duree: Duration,
        end_offset: Duration,
        nb_max_iteration: u8,
        tour_nb: u8,
        rng: StdRng,
        nb_to_choose: usize,
    }

    impl Managing {
        pub fn new() -> Self {
            Managing {
                debut: Instant::now(),
                permier_tour_duree: Duration::from_millis(1000),
                tours_suivant_duree: Duration::from_millis(50),
                end_offset: Duration::from_millis(5),
                nb_max_iteration: 100,
                tour_nb: 0,
                rng: rand::rngs::StdRng::seed_from_u64(81),
                nb_to_choose: 150,
            }
        }

        pub fn with_rng(mut self, rng: StdRng) -> Self {
            self.rng = rng;
            self
        }

        pub fn with_nb_max_iteration(mut self, nb: u8) -> Self {
            self.nb_max_iteration = nb;
            self
        }

        pub fn with_nb_to_choose(mut self, nb: usize) -> Self {
            self.nb_to_choose = nb;
            self
        }

        pub fn restart(&mut self) {
            self.debut = Instant::now();
        }

        pub fn iterations(&self) -> Range<u8> {
            0..(self.nb_max_iteration)
        }

        pub fn is_finished(&self) -> bool {
            let duree_max = match self.tour_nb {
                0 => self.permier_tour_duree,
                _ => self.tours_suivant_duree,
            }
            .sub(self.end_offset);
            let delta = self.debut.elapsed();
            delta > duree_max
        }

        pub fn rng(&mut self) -> &mut StdRng {
            &mut self.rng
        }

        pub fn nb_to_choose(&self) -> usize {
            self.nb_to_choose
        }

        pub fn next_turn(&mut self) {
            self.tour_nb = self.tour_nb.saturating_add(1);
        }
    }
}

pub mod state {
    use std::{
        collections::{HashMap, HashSet},
        fmt::{Debug, Display},
        rc::Rc,
    };

    use itertools::iproduct;

    use super::{atome::*, decision::*, molecule::*};

    pub trait State: Debug {
        fn planification(&self) -> Planification;
        fn action_count(&self) -> ActionCount;
        fn max_ami_id(&self) -> Id;
        fn ressource_ami(&self) -> Ressource;
        fn get_coord(&self, coord: Coord) -> Option<Cell>;

        fn attacking(&self) -> CoordMap<Attacking>;
        fn harvesting(&self) -> CoordMap<Harvesting>;
        fn empty_cell(&self) -> CoordMap<EmptyCell>;
        fn protein(&self) -> CoordMap<Protein>;
        fn organes_amis(&self) -> CoordMap<OrganeAmi>;

        fn organe_ennemy_childs(&self) -> IdMap<HashSet<Id>>;
        fn organe_ennemy_location(&self) -> IdMap<Coord>;

        fn get_neighbour(&self, coor: Coord) -> [Option<Cell>; 4] {
            Direction::all()
                .map(|direction| coor.decaler(direction.clone()))
                .map(|co| self.get_coord(co?))
        }

        fn en_face(&self, coord: Coord, direction: Direction) -> Option<Cell> {
            coord.decaler(direction).and_then(|c| self.get_coord(c))
        }

        fn fertile_coords(&self) -> CoordMap<Fertilite> {
            let empty_cell: HashSet<Coord> = self.empty_cell().keys().cloned().collect();
            let prot: HashSet<Coord> = self.protein().keys().cloned().collect();
            let empty_or_prot: HashSet<Coord> = empty_cell.union(&prot).cloned().collect();
            self.organes_amis()
                .into_iter()
                .flat_map(|(coord, org_ami)| {
                    self.get_neighbour(coord)
                        .into_iter()
                        .filter_map(|c| c.clone())
                        .filter(|c| empty_or_prot.contains(&c.coord))
                        .map(move |c| {
                            (
                                c.coord,
                                Fertilite {
                                    parent_id: org_ami.id,
                                },
                            )
                        })
                        .collect::<CoordMap<Fertilite>>()
                })
                .collect()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InitState {
        dimension: Dimension,
        ressources_ami: Ressource,
        ressources_ennemy: Ressource,
        action_count: ActionCount,
        max_ami_id: Id,

        coord_cells: CoordMap<Cell>,
        empty_cells: CoordMap<EmptyCell>,
        prot_cells: CoordMap<Protein>,
        organe_ami_cells: CoordMap<OrganeAmi>,
        harvesting_cells: CoordMap<Harvesting>,

        organe_ennemy_relation_map: IdMap<HashSet<Id>>,
        organe_ennemy_location_by_id_map: IdMap<Coord>,
    }

    impl InitState {
        pub fn new(
            dimension: Dimension,
            ressources_ami: Ressource,
            ressources_ennemy: Ressource,
            action_count: ActionCount,
            cells: Vec<Cell>,
        ) -> Self {
            let mut max_ami_id = Id::default();
            let coord_cells: CoordMap<Cell> = cells.iter().map(|c| (c.coord, c.clone())).collect();
            let mut prot_cells: CoordMap<Protein> = HashMap::new();
            let mut empty_cells: CoordMap<EmptyCell> =
                iproduct!(0..dimension.width, 0..dimension.height)
                    .map(|(x, y)| (Coord { x, y }, EmptyCell))
                    .collect();
            let mut organe_ami_cells: CoordMap<OrganeAmi> = HashMap::new();
            let mut harvesting_cells: CoordMap<Harvesting> = HashMap::new();
            let mut organe_ennemy_relation_map: IdMap<HashSet<Id>> = HashMap::new();
            let mut organe_ennemy_location_by_id_map: IdMap<Coord> = HashMap::new();

            for cell in cells.into_iter() {
                let Cell { coord, entity } = cell.clone();
                empty_cells.remove(&coord);
                match entity {
                    Entity::Void => {}
                    Entity::Wall => {}
                    Entity::Protein(prot) => {
                        prot_cells.insert(coord, prot);
                    }
                    Entity::Organe(org) if org.owner == Owner::Ennemy => {
                        if org.id != org.parent_id {
                            organe_ennemy_relation_map
                                .entry(org.parent_id)
                                .or_default()
                                .insert(org.id);
                        };
                        organe_ennemy_location_by_id_map.insert(org.id, coord);
                    }
                    Entity::Organe(org) => {
                        max_ami_id = max_ami_id.max(org.id);
                        organe_ami_cells.insert(coord, OrganeAmi { id: org.id });

                        if org.organe_type == OrganeType::Harvester {
                            let en_face =
                                coord.decaler(org.dir).and_then(|coo| coord_cells.get(&coo));
                            if let Some(Cell {
                                coord: coord_prot,
                                entity: Entity::Protein(prot),
                            }) = en_face
                            {
                                harvesting_cells.insert(
                                    coord_prot.clone(),
                                    Harvesting {
                                        protein: prot.clone(),
                                        direction: org.dir,
                                        harvester_coord: coord,
                                    },
                                );
                            };
                        }
                    }
                };
            }
            Self {
                dimension,
                ressources_ami,
                ressources_ennemy,
                action_count,
                coord_cells,
                max_ami_id,
                empty_cells,
                prot_cells,
                organe_ami_cells,
                harvesting_cells,
                organe_ennemy_relation_map,
                organe_ennemy_location_by_id_map,
            }
        }
    }

    impl Display for InitState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Debug::fmt(&self, f)
        }
    }

    impl State for InitState {
        fn planification(&self) -> Planification {
            Planification::default()
        }
        fn action_count(&self) -> ActionCount {
            self.action_count
        }
        fn max_ami_id(&self) -> Id {
            self.max_ami_id
        }
        fn ressource_ami(&self) -> Ressource {
            self.ressources_ami
        }
        fn get_coord(&self, coord: Coord) -> Option<Cell> {
            match self.coord_cells.get(&coord) {
                Some(cell) => Some(cell.clone()),
                None => match self.empty_cells.get(&coord) {
                    Some(_) => Some(Cell {
                        coord,
                        entity: Entity::Void,
                    }),
                    None => None,
                },
            }
        }

        fn harvesting(&self) -> CoordMap<Harvesting> {
            self.harvesting_cells.clone()
        }

        fn empty_cell(&self) -> CoordMap<EmptyCell> {
            self.empty_cells.clone()
        }

        fn protein(&self) -> CoordMap<Protein> {
            self.prot_cells.clone()
        }

        fn organes_amis(&self) -> CoordMap<OrganeAmi> {
            self.organe_ami_cells.clone()
        }

        fn attacking(&self) -> CoordMap<Attacking> {
            CoordMap::new()
        }

        fn organe_ennemy_childs(&self) -> IdMap<HashSet<Id>> {
            self.organe_ennemy_relation_map.clone()
        }

        fn organe_ennemy_location(&self) -> IdMap<Coord> {
            self.organe_ennemy_location_by_id_map.clone()
        }
    }

    #[derive(Debug)]
    pub struct WaitStep {
        previous: Rc<dyn State>,
    }

    impl WaitStep {
        pub fn new(previous: Rc<dyn State>) -> Self {
            Self { previous }
        }
    }

    impl State for WaitStep {
        fn planification(&self) -> Planification {
            self.previous.planification().add_decision(Decision::Wait)
        }

        fn action_count(&self) -> ActionCount {
            self.previous.action_count().decrement()
        }

        fn max_ami_id(&self) -> Id {
            self.previous.max_ami_id()
        }

        fn ressource_ami(&self) -> Ressource {
            self.previous.ressource_ami()
        }

        fn get_coord(&self, coord: Coord) -> Option<Cell> {
            self.previous.get_coord(coord)
        }

        fn harvesting(&self) -> CoordMap<Harvesting> {
            self.previous.harvesting()
        }

        fn empty_cell(&self) -> CoordMap<EmptyCell> {
            self.previous.empty_cell()
        }

        fn protein(&self) -> CoordMap<Protein> {
            self.previous.protein()
        }

        fn organes_amis(&self) -> CoordMap<OrganeAmi> {
            self.previous.organes_amis()
        }

        fn attacking(&self) -> CoordMap<Attacking> {
            self.previous.attacking()
        }

        fn organe_ennemy_childs(&self) -> IdMap<HashSet<Id>> {
            self.previous.organe_ennemy_childs()
        }

        fn organe_ennemy_location(&self) -> IdMap<Coord> {
            self.previous.organe_ennemy_location()
        }
    }

    #[derive(Debug)]
    pub struct GrowStep {
        previous: Rc<dyn State>,
        current: Grow,
        ami_ressource: Ressource,
    }

    impl GrowStep {
        pub fn try_new(previous: Rc<dyn State>, current: Grow) -> Option<Self> {
            let mut ami_ressource = previous
                .ressource_ami()
                .checked_sub(current.organe_type.prix())?;
            if let Some(prot) = previous.protein().get(&current.coord) {
                ami_ressource = ami_ressource.ajout(prot.clone());
            };
            Some(Self {
                previous,
                current,
                ami_ressource,
            })
        }
    }

    impl State for GrowStep {
        fn planification(&self) -> Planification {
            self.previous
                .planification()
                .add_decision(Decision::Grow(self.current))
        }

        fn action_count(&self) -> ActionCount {
            self.previous.action_count().decrement()
        }

        fn max_ami_id(&self) -> Id {
            self.previous.max_ami_id().increment()
        }

        fn ressource_ami(&self) -> Ressource {
            self.ami_ressource
        }

        fn get_coord(&self, coord: Coord) -> Option<Cell> {
            if self.current.coord == coord {
                Some(Cell {
                    coord: self.current.coord,
                    entity: Entity::Organe(Organe {
                        id: self.max_ami_id(),
                        parent_id: self.current.parent_id,
                        root_id: Id::default(),
                        organe_type: self.current.organe_type,
                        dir: self.current.direction,
                        owner: Owner::Me,
                    }),
                })
            } else {
                self.previous.get_coord(coord)
            }
        }

        fn harvesting(&self) -> CoordMap<Harvesting> {
            let mut retour = self.previous.harvesting();
            retour.remove(&self.current.coord);
            if self.current.organe_type != OrganeType::Harvester {
                return retour;
            }
            let en_face = self.en_face(self.current.coord, self.current.direction);

            if let Some(Cell {
                coord: coord_prot,
                entity: Entity::Protein(prot),
            }) = en_face
            {
                retour.insert(
                    coord_prot.clone(),
                    Harvesting {
                        protein: prot.clone(),
                        direction: self.current.direction,
                        harvester_coord: self.current.coord,
                    },
                );
            };
            retour
        }

        fn empty_cell(&self) -> CoordMap<EmptyCell> {
            let mut retour = self.previous.empty_cell();
            retour.remove(&self.current.coord);
            retour
        }

        fn protein(&self) -> CoordMap<Protein> {
            let mut retour = self.previous.protein();
            retour.remove(&self.current.coord);
            retour
        }

        fn organes_amis(&self) -> CoordMap<OrganeAmi> {
            let mut retour = self.previous.organes_amis();
            retour.insert(
                self.current.coord,
                OrganeAmi {
                    id: self.max_ami_id(),
                },
            );
            retour
        }

        fn attacking(&self) -> CoordMap<Attacking> {
            let mut previous_attacking = self.previous.attacking();
            if self.current.organe_type == OrganeType::Tentacle {
                let Some(en_face) = self.en_face(self.current.coord, self.current.direction) else {
                    return previous_attacking;
                };
                let Entity::Organe(org) = en_face.entity else {
                    return previous_attacking;
                };
                if org.owner == Owner::Ennemy {
                    previous_attacking.insert(
                        self.current.coord,
                        Attacking {
                            target_coord: en_face.coord,
                            target_id: org.id,
                        },
                    );
                };
            };
            previous_attacking
        }

        fn organe_ennemy_childs(&self) -> IdMap<HashSet<Id>> {
            self.previous.organe_ennemy_childs()
        }

        fn organe_ennemy_location(&self) -> IdMap<Coord> {
            self.previous.organe_ennemy_location()
        }
    }

    #[derive(Debug)]
    pub struct EndTurn {
        previous: Rc<dyn State>,
        detruit_coord: HashSet<Coord>,
        detruit_id: HashSet<Id>,
    }

    fn get_childs_recursive(
        organe_ennemy_childs: &IdMap<HashSet<Id>>,
        organe_ennemy_location: &IdMap<Coord>,
        parent_id: Id,
    ) -> (HashSet<Id>, HashSet<Coord>) {
        let mut coords: HashSet<Coord> = HashSet::new();
        let mut ids: HashSet<Id> = HashSet::new();
        for child in organe_ennemy_childs
            .get(&parent_id)
            .cloned()
            .unwrap_or_default()
        {
            let (next_ids, next_coords) =
                get_childs_recursive(organe_ennemy_childs, organe_ennemy_location, child);
            ids.insert(child);
            if let Some(coord) = organe_ennemy_location.get(&child).cloned() {
                coords.insert(coord);
            }
            ids = ids.union(&next_ids).cloned().collect();
            coords = coords.union(&next_coords).cloned().collect();
        }
        (ids, coords)
    }

    impl EndTurn {
        pub fn new(state: Rc<dyn State>) -> Self {
            let mut detruit_coord: HashSet<Coord> = HashSet::new();
            let mut detruit_id: HashSet<Id> = HashSet::new();
            let organe_ennemy_childs_map = state.organe_ennemy_childs();
            let organe_ennemy_location_map = state.organe_ennemy_location();
            for attacked in state.attacking().into_values() {
                let (ids, coords) = get_childs_recursive(
                    &organe_ennemy_childs_map,
                    &organe_ennemy_location_map,
                    attacked.target_id,
                );
                detruit_coord = detruit_coord.union(&coords).cloned().collect();
                detruit_id = detruit_id.union(&ids).cloned().collect();
            }
            Self {
                previous: state,
                detruit_coord,
                detruit_id,
            }
        }
    }

    impl State for EndTurn {
        fn planification(&self) -> Planification {
            self.previous.planification().new_turn()
        }

        fn get_coord(&self, coord: Coord) -> Option<Cell> {
            if self.detruit_coord.contains(&coord) {
                return Some(Cell {
                    coord,
                    entity: Entity::Void,
                });
            }
            self.previous.get_coord(coord)
        }

        fn action_count(&self) -> ActionCount {
            ActionCount::default()
        }

        fn max_ami_id(&self) -> Id {
            self.previous.max_ami_id()
        }

        fn ressource_ami(&self) -> Ressource {
            self.previous
                .harvesting()
                .into_iter()
                .fold(self.previous.ressource_ami(), |res, (_c, harvesting)| {
                    res.ajout(harvesting.protein)
                })
        }

        fn harvesting(&self) -> CoordMap<Harvesting> {
            self.previous.harvesting()
        }

        fn empty_cell(&self) -> CoordMap<EmptyCell> {
            let mut empty_cells = self.previous.empty_cell();
            empty_cells.extend(self.detruit_coord.iter().map(|c| (c.clone(), EmptyCell)));
            empty_cells
        }

        fn protein(&self) -> CoordMap<Protein> {
            self.previous.protein()
        }

        fn organes_amis(&self) -> CoordMap<OrganeAmi> {
            self.previous.organes_amis()
        }

        fn attacking(&self) -> CoordMap<Attacking> {
            CoordMap::new()
        }

        fn organe_ennemy_childs(&self) -> IdMap<HashSet<Id>> {
            let mut childs = self.previous.organe_ennemy_childs();
            childs.retain(|key, value| {
                *value = value.difference(&self.detruit_id).cloned().collect();
                !self.detruit_id.contains(&key)
            });
            childs
        }

        fn organe_ennemy_location(&self) -> IdMap<Coord> {
            let mut locations = self.previous.organe_ennemy_location();
            locations.retain(|_key, value| !self.detruit_coord.contains(value));
            locations
        }
    }
}

pub mod decision {
    use super::atome::*;

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
    pub enum Decision {
        #[default]
        Wait,
        Grow(Grow),
    }

    impl ToString for Decision {
        fn to_string(&self) -> String {
            match self {
                Decision::Wait => "WAIT".to_string(),
                Decision::Grow(grow) => grow.to_string(),
            }
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Grow {
        pub parent_id: Id,
        pub coord: Coord,
        pub organe_type: OrganeType,
        pub direction: Direction,
    }

    impl ToString for Grow {
        fn to_string(&self) -> String {
            format!(
                "GROW {} {} {} {}",
                self.parent_id.to_string(),
                self.coord.to_string(),
                self.organe_type.to_string(),
                self.direction.to_string()
            )
        }
    }
}

pub mod molecule {
    use super::atome::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Attacking {
        pub target_coord: Coord,
        pub target_id: Id,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Harvesting {
        pub protein: Protein,
        pub direction: Direction,
        pub harvester_coord: Coord,
    }

    #[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
    pub struct OrganeAmi {
        pub id: Id,
    }

    pub struct Fertilite {
        pub parent_id: Id,
    }

    #[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct EmptyCell;
}

pub mod atome {
    use std::collections::HashMap;

    use super::decision::Decision;

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub enum Protein {
        A,
        B,
        C,
        D,
    }

    impl Protein {
        pub fn all() -> [Self; 4] {
            [Self::A, Self::B, Self::C, Self::D]
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub enum Direction {
        N,
        E,
        S,
        W,
    }

    impl Direction {
        pub fn all() -> [Direction; 4] {
            [Direction::N, Direction::S, Direction::E, Direction::W]
        }
    }

    impl ToString for Direction {
        fn to_string(&self) -> String {
            match self {
                Direction::N => "N",
                Direction::E => "E",
                Direction::S => "S",
                Direction::W => "W",
            }
            .into()
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub enum OrganeType {
        Root,
        Basic,
        Harvester,
        Tentacle,
        Sporer,
    }
    impl OrganeType {
        pub fn prix(&self) -> Ressource {
            match self {
                OrganeType::Root => Ressource::default(),
                OrganeType::Basic => Ressource::new(1, 0, 0, 0),
                OrganeType::Harvester => Ressource::new(0, 0, 1, 1),
                OrganeType::Tentacle => Ressource::new(0, 1, 1, 0),
                OrganeType::Sporer => Ressource::new(0, 1, 0, 1),
            }
        }
    }
    impl ToString for OrganeType {
        fn to_string(&self) -> String {
            match self {
                OrganeType::Basic => "BASIC",
                OrganeType::Root => "ROOT",
                OrganeType::Harvester => "HARVESTER",
                OrganeType::Tentacle => "TENTACLE",
                OrganeType::Sporer => "SPORER",
            }
            .into()
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
    pub struct Id(u8);
    impl Id {
        pub fn new(id: u8) -> Self {
            Id(id)
        }
        pub fn increment(self) -> Self {
            Self(self.0 + 1)
        }
    }
    impl ToString for Id {
        fn to_string(&self) -> String {
            self.0.to_string()
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Organe {
        pub id: Id,
        pub parent_id: Id,
        pub root_id: Id,
        pub organe_type: OrganeType,
        pub dir: Direction,
        pub owner: Owner,
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub enum Entity {
        Void,
        Wall,
        Protein(Protein),
        Organe(Organe),
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Cell {
        pub coord: Coord,
        pub entity: Entity,
    }

    impl Cell {
        pub fn can_grow(&self) -> bool {
            match self.entity {
                Entity::Void => true,
                Entity::Protein(_) => true,
                _ => false,
            }
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub enum Owner {
        Me,
        Ennemy,
    }

    pub type CoordMap<T> = HashMap<Coord, T>;
    pub type IdMap<T> = HashMap<Id, T>;
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Coord {
        pub x: u8,
        pub y: u8,
    }

    impl Coord {
        pub fn decaler(self, direction: Direction) -> Option<Coord> {
            let Coord { mut x, mut y } = self;
            match direction {
                Direction::N => y = y.checked_sub(1)?,
                Direction::S => y = y.checked_add(1)?,
                Direction::W => x = x.checked_sub(1)?,
                Direction::E => x = x.checked_add(1)?,
            };
            Some(Coord { x, y })
        }
    }

    impl ToString for Coord {
        fn to_string(&self) -> String {
            format!("{} {}", self.x, self.y)
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Dimension {
        pub height: u8,
        pub width: u8,
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
    pub struct Ressource {
        a: u8,
        b: u8,
        c: u8,
        d: u8,
    }

    impl Ressource {
        pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
            Self { a, b, c, d }
        }

        pub fn checked_sub(self, rhs: Self) -> Option<Self> {
            Some(Self {
                a: self.a.checked_sub(rhs.a)?,
                b: self.b.checked_sub(rhs.b)?,
                c: self.c.checked_sub(rhs.c)?,
                d: self.d.checked_sub(rhs.d)?,
            })
        }

        pub fn get(&self, prot: &Protein) -> u8 {
            match prot {
                Protein::A => self.a,
                Protein::B => self.b,
                Protein::C => self.c,
                Protein::D => self.d,
            }
        }

        pub fn ajout(mut self, prot: Protein) -> Self {
            match prot {
                Protein::A => {
                    self.a += 1;
                }
                Protein::B => {
                    self.b += 1;
                }
                Protein::C => {
                    self.c += 1;
                }
                Protein::D => {
                    self.d += 1;
                }
            };
            self
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct ActionCount(u32);
    impl ActionCount {
        pub fn new(count: u32) -> Self {
            Self(count)
        }

        pub fn decrement(self) -> Self {
            Self(self.0.saturating_sub(1))
        }

        pub fn is_null(&self) -> bool {
            self.0.eq(&0)
        }
    }

    impl Default for ActionCount {
        fn default() -> Self {
            Self(1)
        }
    }

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    pub struct Planification {
        content: Vec<Vec<Decision>>,
    }

    impl Planification {
        pub fn new_turn(mut self) -> Self {
            self.content.push(Vec::new());
            self
        }

        pub fn add_decision(mut self, decision: Decision) -> Self {
            if let Some(turn) = self.content.last_mut() {
                turn.push(decision);
            };
            self
        }

        pub fn take_first_turn(self) -> Vec<Decision> {
            self.content.into_iter().next().unwrap_or_default()
        }

        pub fn take_content(self) -> Vec<Vec<Decision>> {
            self.content
        }
    }

    impl Default for Planification {
        fn default() -> Self {
            Self {
                content: vec![Vec::new()],
            }
        }
    }
}

mod parsing {
    use std::io;

    use super::{atome::*, state::InitState};

    pub fn parser_dimension() -> Dimension {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        let mut inputs = buf.split(" ").map(str::trim).map(str::parse);
        Dimension {
            width: inputs.next().expect("pas de width").unwrap_or(u8::MAX),
            height: inputs.next().expect("pas de height").unwrap_or(u8::MAX),
        }
    }

    fn parser_count() -> u32 {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        let mut inputs = buf.split(" ").map(str::trim).map(str::parse);
        inputs
            .next()
            .expect("pas de entitycount")
            .expect("entityCount pas un nombre")
    }

    fn parser_entity() -> Cell {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        let mut inputs = buf.split(" ").map(str::trim);
        let x = inputs
            .next()
            .expect("pas de x")
            .parse()
            .expect("x pas un nombre");
        let y = inputs
            .next()
            .expect("pas de y")
            .parse()
            .expect("y pas un nombre");
        let type_str: &str = inputs.next().expect("pas de type");
        let owner: Option<Owner> = match inputs.next().expect("pas d'owner") {
            "1" => Some(Owner::Me),
            "0" => Some(Owner::Ennemy),
            _ => None,
        };
        let organe_id = inputs
            .next()
            .expect("pas d'organ_id")
            .parse()
            .expect("organ_id pas un nombre");
        let organe_dir: Option<Direction> = match inputs.next().expect("pas de direction") {
            "N" => Some(Direction::N),
            "E" => Some(Direction::E),
            "S" => Some(Direction::S),
            "W" => Some(Direction::W),
            _ => None,
        };
        let organe_parent_id = inputs
            .next()
            .expect("pas d'organ_parent_id")
            .parse()
            .expect("organ_parent_id pas un nombre");
        let organe_root_id = inputs
            .next()
            .expect("pas d'organ_root_id")
            .parse()
            .expect("organ_root_id pas un nombre");
        let entity: Entity = match type_str {
            "WALL" => Entity::Wall,
            "A" => Entity::Protein(Protein::A),
            "B" => Entity::Protein(Protein::B),
            "C" => Entity::Protein(Protein::C),
            "D" => Entity::Protein(Protein::D),
            organ_type_str => {
                let organe_type: OrganeType = match organ_type_str {
                    "ROOT" => OrganeType::Root,
                    "BASIC" => OrganeType::Basic,
                    "HARVESTER" => OrganeType::Harvester,
                    "TENTACLE" => OrganeType::Tentacle,
                    "SPORER" => OrganeType::Sporer,
                    _ => panic!("pas d'organe type valide: {organ_type_str}"),
                };
                Entity::Organe(Organe {
                    organe_type,
                    dir: organe_dir.expect("pas de dir"),
                    owner: owner.expect("pas d'owner"),
                    id: Id::new(organe_id),
                    parent_id: Id::new(organe_parent_id),
                    root_id: Id::new(organe_root_id),
                })
            }
        };
        Cell {
            coord: Coord { x, y },
            entity,
        }
    }

    fn parser_resource() -> Ressource {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        let mut inputs = buf.split(" ").map(str::trim).map(str::parse);
        let a = inputs.next().expect("pas de proteine A").unwrap_or(u8::MAX);

        let b = inputs.next().expect("pas de proteine B").unwrap_or(u8::MAX);
        let c = inputs.next().expect("pas de proteine C").unwrap_or(u8::MAX);
        let d = inputs.next().expect("pas de proteine D").unwrap_or(u8::MAX);
        Ressource::new(a, b, c, d)
    }

    pub fn parser_tour(dimension: Dimension) -> InitState {
        let entity_count = parser_count();
        let cells: Vec<Cell> = (0u32..entity_count).map(|_| parser_entity()).collect();
        let ressources_ami = parser_resource();
        let ressources_ennemy = parser_resource();
        let action_count = ActionCount::new(parser_count());
        InitState::new(
            dimension,
            ressources_ami,
            ressources_ennemy,
            action_count,
            cells,
        )
    }
}
