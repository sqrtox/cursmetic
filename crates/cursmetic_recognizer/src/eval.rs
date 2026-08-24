mod word;

use crate::Cursor;
use std::collections::HashMap;
use std::ops::AddAssign;
use std::path::Path;

pub use word::*;

#[derive(Default)]
pub struct Evaluation {
    total: f64,
    max: f64,
}

impl std::fmt::Debug for Evaluation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Evaluation({} / {} = {})",
            self.total,
            self.max,
            self.normalized()
        )
    }
}

impl Evaluation {
    pub fn normalized(&self) -> f64 {
        if self.max == 0.0 {
            return 0.0;
        }

        self.total / self.max
    }

    pub fn merge(&mut self, other: &Self) {
        self.total += other.total;
        self.max += other.max;
    }
}

#[derive(Debug, Default)]
pub struct Scoreboard {
    pub scores: HashMap<Cursor, Evaluation>,
}

impl Scoreboard {
    pub fn add(&mut self, cursor: Cursor, evaluation: &Evaluation) {
        self.scores.entry(cursor).or_default().merge(evaluation);
    }

    pub fn merge_best(&mut self, other: Self) {
        for (cursor, evaluation) in other.scores.into_iter() {
            match self.scores.get_mut(&cursor) {
                Some(current) if evaluation.normalized() > current.normalized() => {
                    *current = evaluation;
                }
                None => {
                    self.scores.insert(cursor.clone(), evaluation);
                }
                _ => {}
            }
        }
    }
}

impl AddAssign for Evaluation {
    fn add_assign(&mut self, rhs: Self) {
        self.total += rhs.total;
        self.max += rhs.max;
    }
}

pub trait Evaluator {
    fn evaluate(&self, path: &Path) -> Option<Scoreboard>;
}
