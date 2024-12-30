use ai::{make_decision, Managing};
use atome::ToCommand;
use parsing::{parser_dimension, parser_tour};
use rand::prelude::*;

pub fn main() {
    let dimension = parser_dimension();
    let mut managing = Managing::new().with_rng(rand::rngs::StdRng::seed_from_u64(33));
    loop {
        managing.restart();
        let game_state = parser_tour(dimension);
        eprintln!("Game State: {}", &game_state);
        let decisions = make_decision(game_state, &mut managing);
        for decision in decisions {
            println!("{}", decision.to_command());
        }
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

    pub fn make_decision(state: InitState, managing: &mut Managing) -> Vec<Decision> {
        let state_pointer = Rc::new(state);
        let mut planification_iter = planifier(state_pointer.clone(), managing)
            .take_first_turn()
            .into_iter();
        let mut decisions: Vec<Decision> = Vec::new();
        for _ in state_pointer.get_action_count() {
            decisions.push(planification_iter.next().unwrap_or(Decision::Wait));
        }
        decisions
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
                .filter_map(|s| {
                    Some((
                        s.clone(),
                        s.action_set().get(Owner::Me).iter().next().cloned()?,
                    ))
                })
                .flat_map(|(s, root_id)| generer_step(s, root_id))
                .partition(|s| s.action_set().get(Owner::Me).is_empty());
            process = encore;
            retour.extend(finis.into_iter().map(EndTurn::new));
        }
        retour
    }

    fn generer_step(state: Rc<dyn State>, root_id: Id) -> impl Iterator<Item = Rc<dyn State>> {
        let wait_step = Rc::new(WaitStep::new(state.clone(), Wait { root_id })) as Rc<dyn State>;
        state
            .grow_candidate(root_id)
            .into_iter()
            .flat_map(|(coord, GrowCandidate { parent_id })| generer_grow(coord, parent_id))
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

        let resources = state.ressource();
        let note_resources = Protein::all()
            .into_iter()
            .filter(|p| resources.get(Owner::Me).get(p) != 0)
            .count();
        let nb_ami = state.nb_organe(Owner::Me) * 3;
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
        fn action_set(&self) -> OwnerMap<HashSet<Id>>;
        fn max_id(&self) -> OwnerMap<Id>;
        fn ressource(&self) -> OwnerMap<Ressource>;
        fn get_by_coord(&self, coord: Coord) -> Option<Cell>;
        fn get_by_id(&self, id: Id) -> Option<OrgWithCoord>;
        fn organes_by_root(&self, root_id: Id) -> HashSet<OrgWithCoord>;

        fn roots(&self) -> OwnerMap<HashSet<Id>>;
        fn attacking(&self) -> CoordMap<Attacking>;
        fn harvesting(&self) -> CoordMap<Harvesting>;
        fn empty_cell(&self) -> CoordMap<EmptyCell>;
        fn protein(&self) -> CoordMap<Protein>;

        fn child_map(&self) -> IdMap<HashSet<Id>>;
        fn organe_ennemy_location(&self) -> IdMap<Coord>;

        fn nb_organe(&self, owner: Owner) -> usize {
            self.roots().get(owner).into_iter().map(|root_id|self.organes_by_root(root_id.clone()).len()).sum()
        }

        fn get_neighbour(&self, coor: Coord) -> [Option<Cell>; 4] {
            Direction::all()
                .map(|direction| coor.decaler(direction.clone()))
                .map(|co| self.get_by_coord(co?))
        }

        fn en_face(&self, coord: Coord, direction: Direction) -> Option<Cell> {
            coord.decaler(direction).and_then(|c| self.get_by_coord(c))
        }

        fn fertile_cell(&self) -> CoordMap<Fertile> {
            self.protein()
                .into_iter()
                .map(|(c, p)| (c, Some(p)))
                .chain(self.empty_cell().keys().map(|c| (c.clone(), None)))
                .map(|(coord, protein)| (coord, Fertile { coord, protein }))
                .collect()
        }

        fn grow_candidate(&self, root_id: Id) -> CoordMap<GrowCandidate> {
            let empty_or_prot: HashSet<Coord> = self.fertile_cell().keys().cloned().collect();
            self.organes_by_root(root_id)
                .into_iter()
                .flat_map(|org| {
                    self.get_neighbour(org.coord)
                        .into_iter()
                        .filter_map(|c| c.clone())
                        .filter(|c| empty_or_prot.contains(&c.coord))
                        .map(move |c| (c.coord, GrowCandidate { parent_id: org.id }))
                        .collect::<CoordMap<GrowCandidate>>()
                })
                .collect()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InitState {
        dimension: Dimension,
        ressources_map: OwnerMap<Ressource>,
        ressources_ennemy: Ressource,
        root_set: OwnerMap<HashSet<Id>>,
        action_count: ActionCount,
        max_id: OwnerMap<Id>,

        coord_cells: CoordMap<Cell>,
        id_map: IdMap<OrgWithCoord>,
        empty_cells: CoordMap<EmptyCell>,
        prot_cells: CoordMap<Protein>,
        organe_ami_cells: CoordMap<OrganeAmi>,
        harvesting_cells: CoordMap<Harvesting>,

        organes_by_root: IdMap<HashSet<OrgWithCoord>>,
        child_map: IdMap<HashSet<Id>>,
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
            let ressources_map: OwnerMap<Ressource> =
                OwnerMap::new(ressources_ami, ressources_ennemy);
            let mut max_id: OwnerMap<Id> = OwnerMap::default();
            let mut root_set: OwnerMap<HashSet<Id>> = OwnerMap::default();
            let coord_cells: CoordMap<Cell> = cells.iter().map(|c| (c.coord, c.clone())).collect();
            let mut id_map: IdMap<OrgWithCoord> = HashMap::new();
            let mut prot_cells: CoordMap<Protein> = HashMap::new();
            let mut empty_cells: CoordMap<EmptyCell> =
                iproduct!(0..dimension.width, 0..dimension.height)
                    .map(|(x, y)| (Coord { x, y }, EmptyCell))
                    .collect();
            let mut organe_ami_cells: CoordMap<OrganeAmi> = HashMap::new();
            let mut harvesting_cells: CoordMap<Harvesting> = HashMap::new();
            let mut organes_by_root: IdMap<HashSet<OrgWithCoord>> = HashMap::new();
            let mut child_map: IdMap<HashSet<Id>> = HashMap::new();
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
                    Entity::Organe(org) => {
                        max_id = max_id.insert_max(org.id);
                        let org_with_coord = OrgWithCoord {
                            coord,
                            id: org.id,
                            parent_id: org.parent_id,
                            root_id: org.root_id,
                            org_type: org.organe_type,
                        };
                        id_map.insert(org.id, org_with_coord);
                        organes_by_root
                            .entry(org.root_id)
                            .or_default()
                            .insert(org_with_coord);
                        if org.id != org.parent_id {
                            child_map.entry(org.parent_id).or_default().insert(org.id);
                        };

                        if org.organe_type == OrganeType::Root {
                            root_set = root_set.update(org.owner, |mut ids| {
                                ids.insert(org.id);
                                ids
                            })
                        }

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
                                        harvester_id: org.id,
                                    },
                                );
                            };
                        }

                        match org.owner {
                            Owner::Ennemy => {
                                organe_ennemy_location_by_id_map.insert(org.id, coord);
                            }
                            Owner::Me => {
                                organe_ami_cells.insert(coord, OrganeAmi { id: org.id });
                            }
                        }
                    }
                };
            }
            Self {
                dimension,
                ressources_map,
                ressources_ennemy,
                root_set,
                coord_cells,
                id_map,
                max_id,
                action_count,
                empty_cells,
                prot_cells,
                organe_ami_cells,
                harvesting_cells,
                child_map,
                organe_ennemy_location_by_id_map,
                organes_by_root,
            }
        }

        pub fn get_action_count(&self) -> ActionCount {
            self.action_count
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

        fn max_id(&self) -> OwnerMap<Id> {
            self.max_id
        }
        fn ressource(&self) -> OwnerMap<Ressource> {
            self.ressources_map
        }
        fn get_by_coord(&self, coord: Coord) -> Option<Cell> {
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

        fn attacking(&self) -> CoordMap<Attacking> {
            CoordMap::new()
        }

        fn child_map(&self) -> IdMap<HashSet<Id>> {
            self.child_map.clone()
        }

        fn organe_ennemy_location(&self) -> IdMap<Coord> {
            self.organe_ennemy_location_by_id_map.clone()
        }

        fn action_set(&self) -> OwnerMap<HashSet<Id>> {
            self.root_set.clone()
        }

        fn get_by_id(&self, id: Id) -> Option<OrgWithCoord> {
            self.id_map.get(&id).copied()
        }

        fn roots(&self) -> OwnerMap<HashSet<Id>> {
            self.root_set.clone()
        }

        fn organes_by_root(&self, root_id: Id) -> HashSet<OrgWithCoord> {
            let retour = self
                .organes_by_root
                .get(&root_id)
                .cloned()
                .unwrap_or_default();
            retour
        }
    }

    #[derive(Debug)]
    pub struct WaitStep {
        previous: Rc<dyn State>,
        decision: Wait,
    }

    impl WaitStep {
        pub fn new(previous: Rc<dyn State>, decision: Wait) -> Self {
            Self { previous, decision }
        }
    }

    impl HaveRoot for WaitStep {
        fn get_root_id(&self) -> Id {
            self.decision.root_id
        }
    }

    impl State for WaitStep {
        fn planification(&self) -> Planification {
            self.previous.planification().add_decision(Decision::Wait)
        }

        fn max_id(&self) -> OwnerMap<Id> {
            self.previous.max_id()
        }

        fn ressource(&self) -> OwnerMap<Ressource> {
            self.previous.ressource()
        }

        fn get_by_coord(&self, coord: Coord) -> Option<Cell> {
            self.previous.get_by_coord(coord)
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

        fn attacking(&self) -> CoordMap<Attacking> {
            self.previous.attacking()
        }

        fn child_map(&self) -> IdMap<HashSet<Id>> {
            self.previous.child_map()
        }

        fn organe_ennemy_location(&self) -> IdMap<Coord> {
            self.previous.organe_ennemy_location()
        }

        fn action_set(&self) -> OwnerMap<HashSet<Id>> {
            self.previous
                .action_set()
                .update(self.get_owner(), |mut ids| {
                    ids.remove(&self.get_root_id());
                    ids
                })
        }

        fn get_by_id(&self, id: Id) -> Option<OrgWithCoord> {
            self.previous.get_by_id(id)
        }

        fn roots(&self) -> OwnerMap<HashSet<Id>> {
            self.previous.roots()
        }

        fn organes_by_root(&self, root_id: Id) -> HashSet<OrgWithCoord> {
            self.previous.organes_by_root(root_id)
        }
    }

    #[derive(Debug)]
    pub struct GrowStep {
        previous: Rc<dyn State>,
        decision: Grow,
        ressource_map: OwnerMap<Ressource>,
    }

    impl GrowStep {
        pub fn try_new(previous: Rc<dyn State>, decision: Grow) -> Option<Self> {
            let mut new_ressource = previous
                .ressource()
                .get(decision.parent_id.get_owner())
                .checked_sub(decision.organe_type.prix())?;
            let fertilite = previous.fertile_cell().get(&decision.coord).cloned()?;
            if let Some(prot) = fertilite.protein {
                new_ressource = new_ressource.ajout_3(prot.clone());
            };
            let ressource_map = previous
                .ressource()
                .update(decision.parent_id.get_owner(), |_| new_ressource);
            Some(Self {
                previous,
                decision,
                ressource_map,
            })
        }

        fn get_last_id(&self) -> Id {
            self.max_id().get(self.get_owner()).clone()
        }
    }

    impl HaveRoot for GrowStep {
        fn get_root_id(&self) -> Id {
            self.previous
                .get_by_id(self.decision.parent_id)
                .map(|org| org.root_id)
                .expect("pas de parent existant")
        }
    }

    impl State for GrowStep {
        fn planification(&self) -> Planification {
            self.previous
                .planification()
                .add_decision(Decision::Grow(self.decision))
        }

        fn max_id(&self) -> OwnerMap<Id> {
            self.previous.max_id().increment(self.get_owner())
        }

        fn ressource(&self) -> OwnerMap<Ressource> {
            self.ressource_map
        }

        fn get_by_coord(&self, coord: Coord) -> Option<Cell> {
            if self.decision.coord == coord {
                Some(Cell {
                    coord: self.decision.coord,
                    entity: Entity::Organe(Organe {
                        id: self.get_last_id(),
                        parent_id: self.decision.parent_id,
                        root_id: self.get_root_id(),
                        organe_type: self.decision.organe_type,
                        dir: self.decision.direction,
                        owner: self.get_owner(),
                    }),
                })
            } else {
                self.previous.get_by_coord(coord)
            }
        }

        fn get_by_id(&self, id: Id) -> Option<OrgWithCoord> {
            if self.get_last_id() == id {
                Some(OrgWithCoord {
                    coord: self.decision.coord,
                    id,
                    parent_id: self.decision.parent_id,
                    root_id: self.get_root_id(),
                    org_type: self.decision.organe_type,
                })
            } else {
                self.previous.get_by_id(id)
            }
        }

        fn harvesting(&self) -> CoordMap<Harvesting> {
            let mut retour = self.previous.harvesting();
            retour.remove(&self.decision.coord);
            if self.decision.organe_type != OrganeType::Harvester {
                return retour;
            }
            let en_face = self.en_face(self.decision.coord, self.decision.direction);

            if let Some(Cell {
                coord: coord_prot,
                entity: Entity::Protein(prot),
            }) = en_face
            {
                retour.insert(
                    coord_prot.clone(),
                    Harvesting {
                        protein: prot.clone(),
                        direction: self.decision.direction,
                        harvester_coord: self.decision.coord,
                        harvester_id: self.get_last_id(),
                    },
                );
            };
            retour
        }

        fn empty_cell(&self) -> CoordMap<EmptyCell> {
            let mut retour = self.previous.empty_cell();
            retour.remove(&self.decision.coord);
            retour
        }

        fn protein(&self) -> CoordMap<Protein> {
            let mut retour = self.previous.protein();
            retour.remove(&self.decision.coord);
            retour
        }

        fn attacking(&self) -> CoordMap<Attacking> {
            let mut previous_attacking = self.previous.attacking();
            if self.decision.organe_type == OrganeType::Tentacle {
                let Some(en_face) = self.en_face(self.decision.coord, self.decision.direction)
                else {
                    return previous_attacking;
                };
                let Entity::Organe(org) = en_face.entity else {
                    return previous_attacking;
                };
                if self.get_owner().is_ennemy(org.owner) {
                    previous_attacking.insert(
                        self.decision.coord,
                        Attacking {
                            target_coord: en_face.coord,
                            target_id: org.id,
                        },
                    );
                };
            };
            previous_attacking
        }

        fn child_map(&self) -> IdMap<HashSet<Id>> {
            let mut retour = self.previous.child_map();
            retour
                .entry(self.decision.parent_id)
                .or_default()
                .insert(self.get_last_id());
            retour
        }

        fn organe_ennemy_location(&self) -> IdMap<Coord> {
            self.previous.organe_ennemy_location()
        }

        fn action_set(&self) -> OwnerMap<HashSet<Id>> {
            self.previous
                .action_set()
                .update(self.decision.parent_id.get_owner(), |mut ids| {
                    ids.remove(&self.get_root_id());
                    ids
                })
        }

        fn roots(&self) -> OwnerMap<HashSet<Id>> {
            self.previous.roots()
        }

        fn organes_by_root(&self, root_id: Id) -> HashSet<OrgWithCoord> {
            let mut retour = self.previous.organes_by_root(root_id);
            if self.get_root_id() == root_id {
                if let Some(org) = self.get_by_id(self.get_last_id()) {
                    retour.insert(org);
                };
            };
            retour
        }
    }

    #[derive(Debug)]
    pub struct SporeStep {
        previous: Rc<dyn State>,
        decision: Spore,
        ressource_map: OwnerMap<Ressource>,
    }

    impl SporeStep {
        fn try_new(previous: Rc<dyn State>, decision: Spore) -> Option<Self> {
            let mut new_ressource = previous
                .ressource()
                .get(decision.parent_id.get_owner())
                .checked_sub(Ressource::new(1, 1, 1, 1))?;
            let fertilite = previous.fertile_cell().get(&decision.coord).cloned()?;
            if let Some(prot) = fertilite.protein {
                new_ressource = new_ressource.ajout_3(prot.clone());
            };
            let ressource_map = previous
                .ressource()
                .update(decision.parent_id.get_owner(), |_| new_ressource);

            let coor_spore = previous.get_by_id(decision.parent_id)?.coord;
            let dir = previous
                .get_by_coord(coor_spore)
                .and_then(|cell| match cell.entity {
                    Entity::Organe(organe) => {
                        Some(organe.dir)
                    }
                    _ => None,
                })?;
            let mut curr_coor = coor_spore.clone();
            for _ in 0..100 {
                let Some(new_coor) = curr_coor.decaler(dir) else {
                    return None;
                };
                curr_coor = new_coor;
                if decision.coord == curr_coor {
                    break;
                }
                if !previous
                    .get_by_coord(curr_coor)
                    .map(|c| c.can_grow())
                    .unwrap_or(false)
                {
                    return None;
                }
            }
            Some(Self {
                decision,
                previous,
                ressource_map,
            })
        }

        fn get_new_root_id(&self) -> Id {
            self.max_id().get(self.get_owner()).clone()
        }
    }

    impl HaveRoot for SporeStep {
        fn get_root_id(&self) -> Id {
            self.previous
                .get_by_id(self.decision.parent_id)
                .map(|org| org.root_id)
                .expect("pas de parent existant pour SporeStep")
        }
    }

    impl State for SporeStep {
        fn planification(&self) -> Planification {
            self.previous
                .planification()
                .add_decision(Decision::Spore(self.decision))
        }

        fn max_id(&self) -> OwnerMap<Id> {
            self.previous.max_id().increment(self.get_owner())
        }

        fn ressource(&self) -> OwnerMap<Ressource> {
            self.ressource_map
        }

        fn get_by_coord(&self, coord: Coord) -> Option<Cell> {
            if coord == self.decision.coord {
                Some(Cell {
                    coord,
                    entity: Entity::Organe(Organe {
                        id: self.get_new_root_id(),
                        parent_id: Id::zero(self.get_owner()),
                        root_id: self.get_new_root_id(),
                        organe_type: OrganeType::Root,
                        dir: Direction::N,
                        owner: self.get_owner(),
                    }),
                })
            } else {
                self.previous.get_by_coord(coord)
            }
        }

        fn get_by_id(&self, id: Id) -> Option<OrgWithCoord> {
            if self.get_new_root_id() == id {
                Some(OrgWithCoord {
                    coord: self.decision.coord,
                    id,
                    parent_id: Id::zero(self.get_owner()),
                    root_id: id,
                    org_type: OrganeType::Root,
                })
            } else {
                self.previous.get_by_id(id)
            }
        }

        fn attacking(&self) -> CoordMap<Attacking> {
            self.previous.attacking()
        }

        fn harvesting(&self) -> CoordMap<Harvesting> {
            self.previous.harvesting()
        }

        fn empty_cell(&self) -> CoordMap<EmptyCell> {
            let mut retour = self.previous.empty_cell();
            retour.remove(&self.decision.coord);
            retour
        }

        fn protein(&self) -> CoordMap<Protein> {
            let mut retour = self.previous.protein();
            retour.remove(&self.decision.coord);
            retour
        }

        fn child_map(&self) -> IdMap<HashSet<Id>> {
            let mut retour = self.previous.child_map();
            retour.insert(self.get_new_root_id(), Default::default());
            retour
        }

        fn organe_ennemy_location(&self) -> IdMap<Coord> {
            self.previous.organe_ennemy_location()
        }

        fn action_set(&self) -> OwnerMap<HashSet<Id>> {
            self.previous.action_set().update(Owner::Me, |mut ids| {
                ids.remove(&self.get_root_id());
                ids
            })
        }

        fn roots(&self) -> OwnerMap<HashSet<Id>> {
            self.previous
                .roots()
                .update(self.decision.parent_id.get_owner(), |mut ids| {
                    ids.insert(self.get_new_root_id());
                    ids
                })
        }

        fn organes_by_root(&self, root_id: Id) -> HashSet<OrgWithCoord> {
            let mut retour = self.previous.organes_by_root(root_id);
            if self.get_new_root_id() == root_id {
                if let Some(org) = self.get_by_id(self.get_new_root_id()) {
                    retour.insert(org);
                };
            };
            retour
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
            let organe_ennemy_childs_map = state.child_map();
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

        fn get_by_coord(&self, coord: Coord) -> Option<Cell> {
            if self.detruit_coord.contains(&coord) {
                Some(Cell {
                    coord,
                    entity: Entity::Void,
                })
            } else {
                self.previous.get_by_coord(coord)
            }
        }

        fn get_by_id(&self, id: Id) -> Option<OrgWithCoord> {
            if self.detruit_id.contains(&id) {
                None
            } else {
                self.previous.get_by_id(id)
            }
        }

        fn max_id(&self) -> OwnerMap<Id> {
            self.previous.max_id()
        }

        fn ressource(&self) -> OwnerMap<Ressource> {
            self.previous.harvesting().into_iter().fold(
                self.previous.ressource(),
                |res, (_c, harvesting)| {
                    res.update(harvesting.harvester_id.get_owner(), |r| {
                        r.ajout_1(harvesting.protein)
                    })
                },
            )
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


        fn attacking(&self) -> CoordMap<Attacking> {
            CoordMap::new()
        }

        fn child_map(&self) -> IdMap<HashSet<Id>> {
            let mut childs = self.previous.child_map();
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

        fn action_set(&self) -> OwnerMap<HashSet<Id>> {
            self.previous.roots()
        }

        fn roots(&self) -> OwnerMap<HashSet<Id>> {
            self.previous.roots()
        }

        fn organes_by_root(&self, root_id: Id) -> HashSet<OrgWithCoord> {
            if self.detruit_id.contains(&root_id) {
                HashSet::new()
            } else {
                self.previous.organes_by_root(root_id)
            }
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
        Spore(Spore),
    }

    impl ToCommand for Decision {
        fn to_command(&self) -> String {
            match self {
                Decision::Wait => "WAIT".to_string(),
                Decision::Grow(grow) => grow.to_command(),
                Decision::Spore(spore) => spore.to_command(),
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

    impl ToCommand for Grow {
        fn to_command(&self) -> String {
            format!(
                "GROW {} {} {} {}",
                self.parent_id.to_command(),
                self.coord.to_command(),
                self.organe_type.to_command(),
                self.direction.to_command()
            )
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Spore {
        pub parent_id: Id,
        pub coord: Coord,
    }

    impl ToCommand for Spore {
        fn to_command(&self) -> String {
            format!(
                "SPORE {} {}",
                self.parent_id.to_command(),
                self.coord.to_command()
            )
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Wait {
        pub root_id: Id,
    }
}

pub mod molecule {
    use std::fmt::Debug;

    use super::atome::*;

    #[derive(PartialEq, Eq, Debug, Clone, Copy, Default)]
    pub struct OwnerMap<T> {
        friend: T,
        ennemy: T,
    }

    impl<T> OwnerMap<T> {
        pub fn new(friend: T, ennemy: T) -> Self {
            OwnerMap { friend, ennemy }
        }

        pub fn update<F>(mut self, owner: Owner, func: F) -> Self
        where
            F: FnOnce(T) -> T,
        {
            match owner {
                Owner::Me => {
                    self.friend = func(self.friend);
                }
                Owner::Ennemy => {
                    self.ennemy = func(self.ennemy);
                }
            };
            self
        }

        pub fn get(&self, owner: Owner) -> &T {
            match owner {
                Owner::Me => &self.friend,
                Owner::Ennemy => &self.ennemy,
            }
        }
    }

    impl OwnerMap<Id> {
        pub fn increment(self, owner: Owner) -> Self {
            self.update(owner, |id| id.increment())
        }

        pub fn insert_max(self, new_id: Id) -> Self {
            self.update(new_id.get_owner(), |id| {
                if id.get_num() < new_id.get_num() {
                    new_id
                } else {
                    id
                }
            })
        }
    }

    impl Default for OwnerMap<Id> {
        fn default() -> Self {
            Self {
                friend: Id::zero(Owner::Me),
                ennemy: Id::zero(Owner::Ennemy),
            }
        }
    }

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
        pub harvester_id: Id,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct OrganeAmi {
        pub id: Id,
    }

    #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
    pub struct OrgWithCoord {
        pub coord: Coord,
        pub id: Id,
        pub parent_id: Id,
        pub root_id: Id,
        pub org_type: OrganeType,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ProteinWithCoord {
        pub coord: Coord,
        pub protein: Protein,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Fertile {
        pub coord: Coord,
        pub protein: Option<Protein>,
    }

    pub struct GrowCandidate {
        pub parent_id: Id,
    }

    #[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct EmptyCell;
}

pub mod atome {
    use std::{collections::HashMap, ops::Range};

    use super::decision::Decision;

    pub trait ToCommand {
        fn to_command(&self) -> String;
    }

    pub trait HaveRoot {
        fn get_root_id(&self) -> Id;
    }

    pub trait HaveOwner {
        fn get_owner(&self) -> Owner;
    }

    impl<T: HaveRoot> HaveOwner for T {
        fn get_owner(&self) -> Owner {
            self.get_root_id().get_owner()
        }
    }

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

    impl ToCommand for Direction {
        fn to_command(&self) -> String {
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
    impl ToCommand for OrganeType {
        fn to_command(&self) -> String {
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

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Id {
        owner: Owner,
        id: u8,
    }

    impl Id {
        pub fn new(owner: Owner, id: u8) -> Self {
            Id { owner, id }
        }
        pub fn zero(owner: Owner) -> Self {
            Id { owner, id: 0 }
        }
        pub fn increment(self) -> Self {
            Self {
                owner: self.owner,
                id: self.id.saturating_add(1),
            }
        }

        pub fn get_num(&self) -> u8 {
            self.id
        }
    }
    impl HaveOwner for Id {
        fn get_owner(&self) -> Owner {
            self.owner
        }
    }
    impl ToCommand for Id {
        fn to_command(&self) -> String {
            self.id.to_string()
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

    impl Owner {
        pub fn switch_side(self) -> Self {
            match self {
                Owner::Me => Owner::Ennemy,
                Owner::Ennemy => Owner::Me,
            }
        }

        pub fn is_ennemy(self, other: Self) -> bool {
            self == other.switch_side()
        }
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

    impl ToCommand for Coord {
        fn to_command(&self) -> String {
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

        fn ajout(mut self, prot: Protein, nb: u8) -> Self {
            match prot {
                Protein::A => {
                    self.a = self.a.saturating_add(nb);
                }
                Protein::B => {
                    self.b = self.b.saturating_add(nb);
                }
                Protein::C => {
                    self.c = self.c.saturating_add(nb);
                }
                Protein::D => {
                    self.d = self.d.saturating_add(nb);
                }
            };
            self
        }

        pub fn ajout_1(self, prot: Protein) -> Self {
            self.ajout(prot, 1)
        }

        pub fn ajout_3(self, prot: Protein) -> Self {
            self.ajout(prot, 3)
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

    impl IntoIterator for ActionCount {
        type Item = u32;

        type IntoIter = Range<u32>;

        fn into_iter(self) -> Self::IntoIter {
            0..self.0
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
        let owner_opt: Option<Owner> = match inputs.next().expect("pas d'owner") {
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
                let owner = owner_opt.expect("pas d'owner");
                Entity::Organe(Organe {
                    organe_type,
                    dir: organe_dir.expect("pas de dir"),
                    owner,
                    id: Id::new(owner, organe_id),
                    parent_id: Id::new(owner, organe_parent_id),
                    root_id: Id::new(owner, organe_root_id),
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
