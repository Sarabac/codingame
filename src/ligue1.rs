
use ai::make_decision;
use parsing::{parser_dimension, parser_tour};

fn main() {
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

    pub fn make_decision(_state: State) -> Decision {
        Decision::Wait
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct State {
        dimension: Dimension,
        ressources: Ressource,
        ressources_ennemy: Ressource,
        action_count: ActionCount,
        coords: HashMap<Coord, Rc<Cell>>,
    }

    impl State {
        pub fn new(
            dimension: Dimension,
            ressources_ami: Ressource,
            ressources_ennemy: Ressource,
            action_count: ActionCount,
            cells: Vec<Cell>,
        ) -> Self {
            let coords: HashMap<Coord, Rc<Cell>> = cells
                .into_iter()
                .map(|cell| (cell.coord.clone(), Rc::new(cell)))
                .collect();
            Self {
                dimension,
                ressources: ressources_ami,
                ressources_ennemy,
                action_count,
                coords,
            }
        }

        pub fn action_count(&self) -> ActionCount {
            self.action_count
        }
    }

    impl Display for State {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Debug::fmt(&self, f)
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

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Id(i32);
    impl Id {
        pub fn new(id: i32) -> Self {
            Id(id)
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
        pub x: i32,
        pub y: i32,
    }

    impl ToString for Coord {
        fn to_string(&self) -> String {
            format!("{} {}", self.x, self.y)
        }
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Dimension {
        pub height: i32,
        pub width: i32,
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Ressource([i32; 4]);
    impl Ressource {
        pub fn new(a: i32, b: i32, c: i32, d: i32) -> Self {
            Ressource([a, b, c, d])
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

    use super::{base_objects::*, main_objects::State};

    pub fn parser_dimension() -> Dimension {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        let mut inputs = buf.split(" ").map(str::trim).map(str::parse::<i32>);
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
            .parse::<i32>()
            .expect("x pas un nombre");
        let y = inputs
            .next()
            .expect("pas de y")
            .parse::<i32>()
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
            .parse::<i32>()
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
            .parse::<i32>()
            .expect("organ_parent_id pas un nombre");
        let organe_root_id = inputs
            .next()
            .expect("pas d'organ_root_id")
            .parse::<i32>()
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
        let mut inputs = buf.split(" ").map(str::trim).map(str::parse::<i32>);
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

    pub fn parser_tour(dimension: Dimension) -> State {
        let entity_count = parser_count();
        let cells: Vec<Cell> = (0i32..entity_count).map(|_| parser_entity()).collect();
        let ressources_ami = parser_resource();
        let ressources_ennemy = parser_resource();
        let action_count = ActionCount::new(parser_count());
        State::new(
            dimension,
            ressources_ami,
            ressources_ennemy,
            action_count,
            cells,
        )
    }
}
