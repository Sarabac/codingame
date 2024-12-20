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

mod ai {
    use super::main_objects::*;

    pub fn make_decision(_state: GameState) -> Decision {
        Decision::Wait
    }

    fn generer_decisions(state: &impl State) -> impl Iterator<Item = Decision> {

    }

    fn juger(state: &impl State) -> f32 {
        let nb_harvesting: u16 = state.harvesting().len().try_into().expect("trop d'harvesting");
        let note_nb_harvesting = f32::from(nb_harvesting.min(3)) * 4f32;

        return 1f32 + note_nb_harvesting;
    }
}

mod main_objects {
    use std::{
        collections::HashMap,
        fmt::{Debug, Display},
        rc::Rc,
    };

    use super::base_objects::*;

    pub enum Decision {
        Wait,
        Grow(Id, Coord, OrganeType, Direction),
    }

    impl ToString for Decision {
        fn to_string(&self) -> String {
            match self {
                Decision::Wait => "WAIT".to_string(),
                Decision::Grow(id, coord, organe_type, direction) => format!(
                    "GROW {} {} {} {}",
                    id.to_string(),
                    coord.to_string(),
                    organe_type.to_string(),
                    direction.to_string()
                ),
            }
        }
    }

    pub trait State {
        fn get_max_ami_id(&self) -> Id;
        fn get_ami_ressource(&self) -> Ressource;
        fn next_ami_ressource(&self) -> Ressource {
            self.harvesting().into_iter().fold(self.get_ami_ressource(), |res, prot| res.ajout(prot))
        }
        fn get_coord(&self, coord: Coord) -> Option<&Cell>;

        fn iter_values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Cell> + 'a>;

        fn harvesting(&self) -> Vec<Protein> {
            self.iter_values()
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
            [Direction::N, Direction::S, Direction::E, Direction::W]
                .into_iter()
                .filter_map(|direction| coor.decaler(direction))
                .filter_map(|co| self.get_coord(co))
                .collect()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GameState {
        dimension: Dimension,
        ressources: Ressource,
        ressources_ennemy: Ressource,
        action_count: ActionCount,
        coords: HashMap<Coord, Cell>,
        max_ami_id: Id,
    }

    impl GameState {
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
            let coords: HashMap<Coord, Cell> = cells
                .into_iter()
                .map(|cell| (cell.coord.clone(), cell))
                .collect();
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

    impl Display for GameState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Debug::fmt(&self, f)
        }
    }

    impl State for GameState {
        fn get_max_ami_id(&self) -> Id {
            self.max_ami_id
        }
        fn get_ami_ressource(&self) -> Ressource {
            self.ressources
        }

        fn get_coord(&self, coord: Coord) -> Option<&Cell> {
            self.coords.get(&coord)
        }

        fn iter_values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Cell> + 'a> {
            Box::new(self.coords.values().into_iter())
        }
    }

    pub struct GameStep {
        previous: Rc<dyn State>,
        current: Decision,
        cell_change: Option<Cell>,
        new_ami_ressource: Ressource,
    }

    impl GameStep {
        pub fn try_new(previous: Rc<dyn State>, current: Decision) -> Option<Self> {
            let cell_change: Option<Cell>;
            let prix_change: Ressource;
            match current {
                Decision::Wait => {
                    cell_change = None;
                    prix_change = Ressource::default()
                }
                Decision::Grow(parent_id, coo, organe_type, dir) => {
                    cell_change = Some(Cell {
                        coord: coo,
                        entity: Entity::Organe(Organe {
                            id: previous.get_max_ami_id().increment(),
                            parent_id,
                            root_id: Id::default(),
                            organe_type,
                            dir,
                            owner: Owner::Me,
                        }),
                    });
                    prix_change = organe_type.prix();
                }
            };
            let new_ami_ressource = previous.next_ami_ressource().checked_sub(prix_change)?;
            Some(Self {
                previous,
                current,
                cell_change,
                new_ami_ressource,
            })
        }
    }

    impl State for GameStep {
        fn get_max_ami_id(&self) -> Id {
            self.cell_change
                .and_then(|val| match val.entity {
                    Entity::Organe(organe) if organe.owner == Owner::Me => Some(organe.id),
                    _ => None,
                })
                .unwrap_or_else(|| self.previous.get_max_ami_id())
        }

        fn get_ami_ressource(&self) -> Ressource {
            self.new_ami_ressource
        }

        fn get_coord(&self, coord: Coord) -> Option<&Cell> {
            self.cell_change
                .as_ref()
                .filter(|cell| cell.coord == coord)
                .or_else(|| self.previous.get_coord(coord))
        }

        fn iter_values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Cell> + 'a> {
            Box::new(self.previous.iter_values().map(|cell| {
                self.cell_change
                    .as_ref()
                    .filter(|change| change.coord == cell.coord)
                    .unwrap_or(cell)
            }))
        }
    }
}

mod base_objects {
    use std::ops::Range;

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub enum Protein {
        A,
        B,
        C,
        D,
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub enum Direction {
        N,
        E,
        S,
        W,
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
    pub struct Ressource([u16; 4]);
    impl Ressource {
        pub fn new(a: u16, b: u16, c: u16, d: u16) -> Self {
            Ressource([a, b, c, d])
        }

        pub fn checked_sub(self, rhs: Self) -> Option<Self> {
            let Ressource([a1, b1, c1, d1]) = self;
            let Ressource([a2, b2, c2, d2]) = rhs;
            Some(Ressource([
                a1.checked_sub(a2)?,
                b1.checked_sub(b2)?,
                c1.checked_sub(c2)?,
                d1.checked_sub(d2)?,
            ]))
        }

        pub fn ajout(self, prot: Protein) -> Self {
            let Ressource([mut a, mut b, mut c, mut d]) = self;
            match prot {
                Protein::A => {a += 1;}, 
                Protein::B => {b += 1;}, 
                Protein::C => {c += 1;}, 
                Protein::D => {d += 1;}, 
            };
            Self([a, b, c, d])
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

    use super::{base_objects::*, main_objects::GameState};

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

    pub fn parser_tour(dimension: Dimension) -> GameState {
        let entity_count = parser_count();
        let cells: Vec<Cell> = (0i32..entity_count).map(|_| parser_entity()).collect();
        let ressources_ami = parser_resource();
        let ressources_ennemy = parser_resource();
        let action_count = ActionCount::new(parser_count());
        GameState::new(
            dimension,
            ressources_ami,
            ressources_ennemy,
            action_count,
            cells,
        )
    }
}
