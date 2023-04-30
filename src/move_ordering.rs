

use chess::{ChessMove, Square, Board, MoveGen};

use crate::{transpo::TranspoTable, search::MAX_DEPTH};
const DUMMY_MOVE : ChessMove = ChessMove {
    source: Square::A1,
    dest: Square::A1,
    promotion: None
};
const MAX_MOVES : usize = 255;
const NUM_KILLER_MOVES : usize = 2;
const KILLER_MOVE_VALUE : i32 = 400;

pub struct MoveOrderer {
    killer_moves : [[ChessMove; NUM_KILLER_MOVES]; MAX_DEPTH as usize],

}

impl MoveOrderer {
    pub fn new() -> MoveOrderer {
        MoveOrderer {
            killer_moves : [[DUMMY_MOVE; NUM_KILLER_MOVES]; MAX_DEPTH as usize],
        }
    }

    pub fn update_killer_move(&mut self, depth : usize, killer_move : ChessMove) {
        for i in (1..NUM_KILLER_MOVES).rev() {
            self.killer_moves[depth][i] = self.killer_moves[depth][i-1];
        }
        self.killer_moves[depth][0] = killer_move;
    }
}

pub struct MoveOrdering {
    moves : [ChessMove; MAX_MOVES],
    move_scores : [i32; MAX_MOVES],
    num_moves : usize,
}


impl MoveOrdering {
    pub fn new () -> MoveOrdering {
        MoveOrdering {
            moves : [DUMMY_MOVE; MAX_MOVES],
            move_scores : [0; MAX_MOVES],
            num_moves : 0,
        }
    }

    pub fn from_moves(gen : &mut MoveGen) -> MoveOrdering {
        let mut moves = [DUMMY_MOVE; MAX_MOVES];
        let mut num_moves = 0;
        for chess_move in gen {
            moves[num_moves] = chess_move;
            num_moves += 1;
        }
        MoveOrdering {
            moves : moves,
            move_scores : [0; MAX_MOVES],
            num_moves : num_moves
        }
    }

    pub fn add_move(&mut self, chess_move : ChessMove) {
        self.moves[self.num_moves] = chess_move;
        self.move_scores[self.num_moves] = 0;
        self.num_moves += 1;
    }

    fn calculate_score(&self, chess_move : ChessMove, board : &Board, depth : usize, move_orderer : &MoveOrderer) -> i32{
        for i in 0..NUM_KILLER_MOVES {
            if move_orderer.killer_moves[depth][i] == chess_move {
                return KILLER_MOVE_VALUE
            }
        }
        1
    }

    pub fn get(&self, idx : usize) -> ChessMove {
        self.moves[idx]
    }

    fn swap_moves(&mut self, i : usize, j : usize) {
        let temp = self.moves[i];
        self.moves[i] = self.moves[j];
        self.moves[j] = temp;
    }

    pub fn get_next_best_move(&mut self, moves_processed : usize, board : &Board, depth : usize, tt: &TranspoTable, move_orderer : &MoveOrderer) -> ChessMove {
        // If it's the first move for this board, check if we have it in the transpo table
        // If so, we want to return the best move we found before
        if moves_processed == 0 {
            let entry = tt.probe(board.get_hash());
            if entry.hash == board.get_hash() {
                let entry_move = entry.best_move;
                for i in 0..self.num_moves {
                    if entry_move == self.moves[i] {
                        self.swap_moves(0, i);
                        return entry_move
                    }
                }
            }
        }


        // Find the best move
        let mut max_score = i32::MIN;
        let mut best_idx = moves_processed;
        for i in moves_processed..self.num_moves {
            let score = self.calculate_score(self.moves[i], board, depth, move_orderer);
            self.move_scores[i] = score;
            if score > max_score {
                max_score = score;
                best_idx = i;
            }
        }
        

        // Make it so best move is at idx moves_processed
        self.swap_moves(moves_processed, best_idx);
        self.moves[moves_processed]
    }

    pub fn len(&self) -> usize {
        self.num_moves
    }
}