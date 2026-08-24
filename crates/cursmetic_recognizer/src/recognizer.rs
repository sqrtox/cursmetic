use std::path::Path;

use crate::Cursor;
use crate::eval::{Evaluator, Scoreboard};

#[derive(Debug)]
pub struct CursorMatch {
    pub cursor: Cursor,
    pub score: f64,
}

pub struct CursorRecognizer {
    evaluators: Vec<Box<dyn Evaluator>>,
}

impl CursorRecognizer {
    pub fn new() -> Self {
        Self {
            evaluators: Vec::new(),
        }
    }

    pub fn evaluator<E>(mut self, evaluator: E) -> Self
    where
        E: Evaluator + 'static,
    {
        self.evaluators.push(Box::new(evaluator));

        self
    }

    pub fn recognize<P>(&self, path: P) -> Vec<CursorMatch>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let mut scoreboard = Scoreboard::default();

        for evaluator in &self.evaluators {
            if let Some(sc) = evaluator.evaluate(path) {
                scoreboard.merge_best(sc);
            }
        }

        let mut scores = scoreboard
            .scores
            .into_iter()
            .map(|(k, v)| (k, v.normalized()))
            .collect::<Vec<_>>();

        scores.sort_by(|(_, a), (_, b)| b.total_cmp(&a));

        scores
            .into_iter()
            .map(|(cursor, score)| CursorMatch { cursor, score })
            .collect()
    }
}
