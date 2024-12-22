use ai::make_decision;
use parsing::{parser_dimension, parser_tour};

pub fn main() {
    let dimension = parser_dimension();
    loop {
        let game_state = parser_tour(dimension);
        eprintln!("Game State: {}", &game_state);
        let decision = make_decision(game_state);
        println!("{}", decision.to_string());
    }
}

pub mod ai {
    use std::rc::Rc;

    use itertools::iproduct;

    use super::{base_objects::*, decision::*, state::*};

    pub fn make_decision(state: InitState) -> Decision {
        generer(Rc::new(state))
    }

    pub fn generer(state: Rc<dyn State>) -> Decision {
        state
            .fertile_coords()
            .into_iter()
            .flat_map(|(parent_id, coord)| generer_grow(parent_id, coord))
            .filter_map(|decision| GrowStep::try_new(state.clone(), decision))
            .max_by(|a, b| juger(a).cmp(&juger(b)))
            .and_then(|game_step| game_step.first_decision())
            .unwrap_or(Decision::Wait)
    }

    fn generer_grow(parent_id: Id, coord: Coord) -> impl Iterator<Item = Grow> {
        iproduct!([OrganeType::Basic, OrganeType::Harvester], Direction::all()).map(
            move |(organe_type, direction)| Grow {
                parent_id,
                coord,
                organe_type,
                direction,
            },
        )
    }

    pub fn juger(state: &impl State) -> usize {
        let nb_harvesting = state.harvesting().len();
        let note_nb_harvesting = nb_harvesting.min(3) * 4;

        let resources = state.get_ami_ressource();
        let note_resources = Protein::all()
            .into_iter()
            .filter(|p| resources.get(p) != 0)
            .count();

        return 1 + note_nb_harvesting + note_resources;
    }
}

pub mod state {
    use itertools::Itertools;
    use std::{
        collections::HashMap,
        fmt::{Debug, Display},
        rc::Rc,
    };

    use super::{base_objects::*, decision::*};

    pub trait State {
        fn first_decision(&self) -> Option<Decision>;
        fn get_max_ami_id(&self) -> Id;
        fn get_ami_ressource(&self) -> Ressource;
        fn get_coord(&self, coord: Coord) -> Option<&Cell>;

        fn iter_values(&self) -> Vec<&Cell>;

        fn harvesting(&self) -> Vec<Protein> {
            self.iter_values()
                .into_iter()
                .filter_map(|cell| {
                    let Entity::Organe(organe) = cell.entity else {
                        return None;
                    };
                    if OrganeType::Harvester != organe.organe_type || organe.owner != Owner::Me {
                        return None;
                    }
                    let en_face = cell.coord.decaler(organe.dir)?;
                    let cell_en_face = self.get_coord(en_face)?;
                    match cell_en_face.entity {
                        Entity::Protein(prot) => Some(prot),
                        _ => None,
                    }
                })
                .collect()
        }

        fn get_neighbour(&self, coor: Coord) -> Vec<&Cell> {
            Direction::all()
                .into_iter()
                .filter_map(|direction| coor.decaler(direction.clone()))
                .filter_map(|co| self.get_coord(co))
                .collect()
        }

        fn organes_amis(&self) -> Vec<(Coord, Id)> {
            self.iter_values()
                .into_iter()
                .filter_map(|cell| match cell.entity {
                    Entity::Organe(organe) if organe.owner == Owner::Me => {
                        Some((cell.coord, organe.id))
                    }
                    _ => None,
                })
                .collect()
        }

        fn fertile_coords(&self) -> Vec<(Id, Coord)> {
            self.organes_amis()
                .into_iter()
                .flat_map(|(parent_coord, parent_id)| {
                    self.get_neighbour(parent_coord)
                        .into_iter()
                        .filter(|cell| cell.can_grow())
                        .map(move |cell| (parent_id, cell.coord))
                })
                .unique_by(|(_parent_id, coord)| coord.clone())
                .collect()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InitState {
        dimension: Dimension,
        ressources: Ressource,
        ressources_ennemy: Ressource,
        action_count: ActionCount,
        coords: HashMap<Coord, Cell>,
        max_ami_id: Id,
    }

    impl InitState {
        pub fn new(
            dimension: Dimension,
            ressources_ami: Ressource,
            ressources_ennemy: Ressource,
            action_count: ActionCount,
            cells: Vec<Cell>,
        ) -> Self {
            let max_ami_id = cells
                .iter()
                .filter_map(|cell| match cell.entity {
                    Entity::Organe(Organe { id, owner, .. }) if owner == Owner::Me => Some(id),
                    _ => None,
                })
                .max()
                .unwrap_or_default();
            let mut coords: HashMap<Coord, Cell> =
                cells.into_iter().map(|cell| (cell.coord, cell)).collect();
            for x in 0..(dimension.width) {
                for y in 0..(dimension.height) {
                    let coord = Coord { x, y };
                    coords.entry(coord).or_insert(Cell {
                        coord,
                        entity: Entity::Void,
                    });
                }
            }
            Self {
                dimension,
                ressources: ressources_ami,
                ressources_ennemy,
                action_count,
                coords,
                max_ami_id,
            }
        }
    }

    impl Display for InitState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Debug::fmt(&self, f)
        }
    }

    impl State for InitState {
        fn first_decision(&self) -> Option<Decision> {
            None
        }

        fn get_max_ami_id(&self) -> Id {
            self.max_ami_id
        }
        fn get_ami_ressource(&self) -> Ressource {
            self.ressources
        }

        fn get_coord(&self, coord: Coord) -> Option<&Cell> {
            self.coords.get(&coord)
        }

        fn iter_values(&self) -> Vec<&Cell> {
            self.coords.values().collect_vec()
        }
    }

    pub struct GrowStep {
        previous: Rc<dyn State>,
        current: Grow,
        cell_change: Cell,
        ami_ressource: Ressource,
    }

    impl GrowStep {
        pub fn try_new(previous: Rc<dyn State>, current: Grow) -> Option<Self> {
            let cell_change: Cell = Cell {
                coord: current.coord,
                entity: Entity::Organe(Organe {
                    id: previous.get_max_ami_id().increment(),
                    parent_id: current.parent_id,
                    root_id: Id::default(),
                    organe_type: current.organe_type,
                    dir: current.direction,
                    owner: Owner::Me,
                }),
            };
            let ami_ressource = previous
                .get_ami_ressource()
                .checked_sub(current.organe_type.prix())?;
            Some(Self {
                previous,
                current,
                cell_change,
                ami_ressource,
            })
        }
    }

    impl State for GrowStep {
        fn first_decision(&self) -> Option<Decision> {
            self.previous
                .first_decision()
                .or(Some(Decision::Grow(self.current)))
        }

        fn get_max_ami_id(&self) -> Id {
            match self.cell_change.entity {
                Entity::Organe(organe) if organe.owner == Owner::Me => organe.id,
                _ => self.previous.get_max_ami_id(),
            }
        }

        fn get_ami_ressource(&self) -> Ressource {
            self.ami_ressource
        }

        fn get_coord(&self, coord: Coord) -> Option<&Cell> {
            if self.cell_change.coord == coord {
                Some(&self.cell_change)
            } else {
                self.previous.get_coord(coord)
            }
        }

        fn iter_values(&self) -> Vec<&Cell> {
            self.previous
                .iter_values()
                .into_iter()
                .map(|cell| {
                    if self.cell_change.coord == cell.coord {
                        &self.cell_change
                    } else {
                        cell
                    }
                })
                .collect()
        }
    }

    pub struct EndTurn {
        previous: Rc<dyn State>,
    }

    impl State for EndTurn {
        fn get_coord(&self, coord: Coord) -> Option<&Cell> {
            self.previous.get_coord(coord)
        }

        fn iter_values(&self) -> Vec<&Cell> {
            self.previous.iter_values()
        }

        fn first_decision(&self) -> Option<Decision> {
            self.previous.first_decision()
        }

        fn get_max_ami_id(&self) -> Id {
            self.previous.get_max_ami_id()
        }

        fn get_ami_ressource(&self) -> Ressource {
            self.harvesting()
                .into_iter()
                .fold(self.previous.get_ami_ressource(), |res, prot| {
                    res.ajout(prot)
                })
        }
    }
}

pub mod decision {
    use super::base_objects::*;

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub enum Decision {
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
pub mod base_objects {
    use std::ops::Range;

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
    }
    impl OrganeType {
        pub fn prix(&self) -> Ressource {
            match self {
                OrganeType::Root => Ressource::default(),
                OrganeType::Basic => Ressource::new(1, 0, 0, 0),
                OrganeType::Harvester => Ressource::new(0, 0, 1, 1),
            }
        }
    }
    impl ToString for OrganeType {
        fn to_string(&self) -> String {
            match self {
                OrganeType::Basic => "BASIC",
                OrganeType::Root => "ROOT",
                OrganeType::Harvester => "HARVESTER",
            }
            .into()
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
    pub struct Id(u32);
    impl Id {
        pub fn new(id: u32) -> Self {
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

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Coord {
        pub x: u16,
        pub y: u16,
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
        pub height: u16,
        pub width: u16,
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
    pub struct Ressource {
        a: u16,
        b: u16,
        c: u16,
        d: u16,
    }

    impl Ressource {
        pub fn new(a: u16, b: u16, c: u16, d: u16) -> Self {
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

        pub fn get(&self, prot: &Protein) -> u16 {
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
    pub struct ActionCount(i32);
    impl ActionCount {
        pub fn new(count: i32) -> Self {
            Self(count)
        }
    }

    impl IntoIterator for ActionCount {
        type IntoIter = Range<i32>;
        type Item = i32;
        fn into_iter(self) -> Self::IntoIter {
            0..self.0
        }
    }
}

mod parsing {
    use std::io;

    use super::{base_objects::*, state::InitState};

    pub fn parser_dimension() -> Dimension {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        let mut inputs = buf.split(" ").map(str::trim).map(str::parse::<u16>);
        Dimension {
            width: inputs
                .next()
                .expect("pas de width")
                .expect("width pas un nombre"),
            height: inputs
                .next()
                .expect("pas de height")
                .expect("height pas un nombre"),
        }
    }

    fn parser_count() -> i32 {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        let mut inputs = buf.split(" ").map(str::trim).map(str::parse::<i32>);
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
                    _ => panic!("pas d'organe type valide {organ_type_str}"),
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
        let mut inputs = buf.split(" ").map(str::trim).map(str::parse::<u16>);
        let a = inputs
            .next()
            .expect("pas de proteine A")
            .expect("proteine A pas un nombre");

        let b = inputs
            .next()
            .expect("pas de proteine B")
            .expect("proteine B pas un nombre");
        let c = inputs
            .next()
            .expect("pas de proteine C")
            .expect("proteine C pas un nombre");
        let d = inputs
            .next()
            .expect("pas de proteine D")
            .expect("proteine D pas un nombre");
        Ressource::new(a, b, c, d)
    }

    pub fn parser_tour(dimension: Dimension) -> InitState {
        let entity_count = parser_count();
        let cells: Vec<Cell> = (0i32..entity_count).map(|_| parser_entity()).collect();
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
