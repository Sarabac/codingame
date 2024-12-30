
use crate::ligue1::{atome::*, state::*};

#[derive(Debug, Clone)]
pub struct StateBuilder {
    dimension: Dimension,
    ressources_ami: Ressource,
    ressources_ennemy: Ressource,
    action_count: ActionCount,
    cells: Vec<Cell>,
}

impl StateBuilder {
    pub fn build(self) -> InitState {
        let StateBuilder {
            dimension,
            ressources_ami,
            ressources_ennemy,
            action_count,
            cells,
        } = self;
        InitState::new(
            dimension,
            ressources_ami,
            ressources_ennemy,
            action_count,
            cells,
        )
    }

    pub fn add_cell(mut self, cell: Cell) -> Self {
        self.cells.push(cell);
        self
    }

    pub fn add_cells(mut self, mut cell: Vec<Cell>) -> Self {
        self.cells.append(&mut cell);
        self
    }

    pub fn with_ressources_ami(mut self, ressource: Ressource) -> Self {
        self.ressources_ami = ressource;
        self
    }
}

impl StateBuilder {
    pub fn new_carre_vide_3() -> Self {
        Self {
            dimension: Dimension {
                height: 3,
                width: 3,
            },
            ressources_ami: Ressource::new(1, 1, 1, 1),
            ressources_ennemy: Ressource::new(1, 1, 1, 1),
            action_count: ActionCount::default(),
            cells: Vec::default(),
        }
    }

    pub fn new_ligne_de_3_root_a_gauche() -> Self {
        Self {
            dimension: Dimension {
                height: 1,
                width: 3,
            },
            ressources_ami: Ressource::new(1, 1, 1, 1),
            ressources_ennemy: Ressource::new(1, 1, 1, 1),
            action_count: ActionCount::default(),
            cells: vec![Cell {
                coord: Coord { x: 0, y: 0 },
                ..Self::build_root()
            }],
        }
    }

    /**
     * -----------
     * |.|.|.|.|.|
     * -----------
     * |.|.|.|.|.|
     * -----------
     * |.|.|r|.|.|
     * -----------
     * |.|.|.|.|.|
     * -----------
     * |.|.|.|.|.|
     * -----------
     */
    pub fn new_au_milieu() -> Self {
        Self {
            dimension: Dimension {
                height: 5,
                width: 5,
            },
            ressources_ami: Ressource::new(50, 1, 1, 1),
            ressources_ennemy: Ressource::new(50, 50, 50, 50),
            action_count: ActionCount::default(),
            cells: vec![Cell {
                coord: Coord { x: 2, y: 2 },
                ..Self::build_root()
            }],
        }
    }

    /**
     *
     * |.|.|.|.|.|
     * -----------
     * |.|.|.|.|.|
     * -----------
     * |r|.|.|.|A|
     * -----------
     * |.|.|.|.|.|
     * -----------
     * |.|.|.|.|.|
     *
     */
    pub fn new_a_gauche_prot_a_a_droite() -> Self {
        Self {
            cells: vec![
                Cell {
                    coord: Coord { x: 0, y: 2 },
                    ..Self::build_root()
                },
                Cell {
                    coord: Coord { x: 4, y: 2 },
                    entity: Entity::Protein(Protein::A),
                },
            ],
            ..Self::new_au_milieu()
        }
    }

    pub fn build_root() -> Cell {
        Cell {
            coord: Coord { x: 0, y: 0 },
            entity: Entity::Organe(Organe {
                dir: Direction::N,
                id: Id::zero(Owner::Me),
                parent_id: Id::zero(Owner::Me),
                root_id: Id::zero(Owner::Me),
                organe_type: OrganeType::Root,
                owner: Owner::Me,
            }),
        }
    }
}

#[derive(Debug, Default)]
pub struct OrganismBuilder {
    decalages: Vec<Direction>,
}

impl OrganismBuilder {
    pub fn add_basic(mut self, decale: Direction) -> Self {
        self.decalages.push(decale);
        self
    }

    pub fn build(&self, owner: Owner, depart: Coord) -> Vec<Cell> {
        let mut curr_coord = depart;
        let mut curr_id = Id::zero(owner);
        let root_id = Id::zero(owner);
        let current_cell = Cell {
            coord: curr_coord.clone(),
            entity: Entity::Organe(Organe {
                id: curr_id,
                parent_id: curr_id,
                root_id,
                organe_type: OrganeType::Root,
                dir: Direction::N,
                owner,
            }),
        };
        let mut resultat = vec![current_cell];
        for dir in self.decalages.iter() {
            curr_coord = curr_coord.decaler(dir.clone()).expect("mauvaise direction");
            let parent_id = curr_id;
            curr_id = curr_id.increment();
            resultat.push(Cell {
                coord: curr_coord,
                entity: Entity::Organe(Organe {
                    id: curr_id,
                    parent_id,
                    root_id,
                    organe_type: OrganeType::Basic,
                    dir: Direction::N,
                    owner,
                }),
            });
        }
        resultat
    }
}
