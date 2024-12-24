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
                id: Id::new(0),
                parent_id: Id::new(0),
                root_id: Id::new(0),
                organe_type: OrganeType::Root,
                owner: Owner::Me,
            }),
        }
    }
}
