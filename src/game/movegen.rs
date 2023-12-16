use super::{MoveResult, State};

pub struct MoveGenerator;

impl MoveGenerator {
    pub fn compute(&self, state: &State) -> Vec<MoveResult> {
        let mut moves = Vec::new();
        self.compute_into(state, &mut moves);
        moves
    }

    pub fn compute_into(&self, _state: &State, moves: &mut Vec<MoveResult>) {
        moves.clear();
        todo!()
    }
}
