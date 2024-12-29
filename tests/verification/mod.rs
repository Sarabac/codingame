use std::fmt::Debug;
pub mod random_testing;
use codingame::ligue1::{
    atome::{Coord, Direction, OrganeType, Planification},
    decision::{Decision, Grow},
};
use itertools::Itertools;

#[derive(Default)]
pub struct PlanificationChecker {
    assertions: Vec<Box<dyn Fn(Option<Decision>) -> Option<DecisionFailType>>>,
}

impl PlanificationChecker {
    pub fn verify(&self, planification: Planification) -> Result<(), Box<dyn Debug>> {
        let decision_iter = planification
            .take_content()
            .into_iter()
            .flat_map(|t| t.into_iter());
        let fails: Vec<DecisionFail> = self
            .assertions
            .iter()
            .zip_longest(decision_iter)
            .enumerate()
            .filter_map(|(indice, either_or_both)| {
                let (f, decision) = either_or_both.left_and_right();
                f?(decision).map(|err| DecisionFail {
                    indice,
                    actual: decision,
                    fail_type: err,
                })
            })
            .collect();
        match fails.is_empty() {
            true => Ok(()),
            false => Err(Box::new(fails)),
        }
    }

    pub fn then_finis(mut self) -> Self {
        self.assertions.push(Box::new(|decision| match decision {
            None => None,
            Some(_sinon) => Some(DecisionFailType::PasFinis),
        }));
        self
    }

    pub fn then_wait(mut self) -> Self {
        self.assertions.push(Box::new(|decision| match decision {
            Some(d) => match d {
                Decision::Wait => None,
                _sinon => Some(DecisionFailType::WaitFail),
            },
            None => Some(DecisionFailType::NoMoreDecision),
        }));
        self
    }

    pub fn then_grow(mut self, checker: GrowChecker) -> Self {
        self.assertions
            .push(Box::new(move |decision| match decision {
                Some(Decision::Grow(grow)) => match checker.check(grow) {
                    errors if errors.is_empty() => None,
                    errors => Some(DecisionFailType::GrowFail(errors)),
                },
                Some(_sinon) => Some(DecisionFailType::PasDeTypeGrow),
                None => Some(DecisionFailType::NoMoreDecision),
            }));
        self
    }
}

#[derive(Default)]
pub struct GrowChecker {
    assertions: Vec<Box<dyn Fn(Grow) -> Option<GrowFail>>>,
}

impl GrowChecker {
    pub fn check(&self, grow: Grow) -> Vec<GrowFail> {
        self.assertions.iter().filter_map(|f| f(grow)).collect()
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.assertions.push(Box::new(move |grow| {
            if grow.direction == direction {
                None
            } else {
                Some(GrowFail::BadDirection { actual: direction })
            }
        }));
        self
    }

    pub fn organ_type(mut self, organ_type: OrganeType) -> Self {
        self.assertions.push(Box::new(move |grow| {
            if grow.organe_type == organ_type {
                None
            } else {
                Some(GrowFail::BadOrganType { actual: organ_type })
            }
        }));
        self
    }

    pub fn coord(mut self, coord: Coord) -> Self {
        self.assertions.push(Box::new(move |grow| {
            if grow.coord == coord {
                None
            } else {
                Some(GrowFail::BadCoord { actual: coord })
            }
        }));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionFail {
    indice: usize,
    actual: Option<Decision>,
    fail_type: DecisionFailType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionFailType {
    NoMoreDecision,
    PasFinis,
    WaitFail,
    GrowFail(Vec<GrowFail>),
    PasDeTypeGrow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrowFail {
    BadDirection { actual: Direction },
    BadOrganType { actual: OrganeType },
    BadCoord { actual: Coord },
}
