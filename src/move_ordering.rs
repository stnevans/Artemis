

use chess::{ChessMove, Square, Board, MoveGen, NUM_SQUARES, NUM_COLORS, Color, EMPTY, NUM_PIECES};

use crate::{transpo::TranspoTable, search::MAX_DEPTH, bb_utils::BitBoardUtils};
const DUMMY_MOVE : ChessMove = ChessMove {
    source: Square::A1,
    dest: Square::A1,
    promotion: None
};
const MAX_MOVES : usize = 255;
const NUM_KILLER_MOVES : usize = 2;
const KILLER_MOVE_VALUE : i32 = 200000;
const COUNTER_MOVE_BONUS : i32 = 200;
// [captured][capturing]
const MVV_LVA_VALUES : [[i32; NUM_PIECES]; NUM_PIECES] = [
    [100000, 110000, 120000, 130000, 140000, 150000],
    [200000, 210000, 220000, 230000, 240000, 250000],
    [300000, 310000, 320000, 330000, 340000, 350000],
    [400000, 410000, 420000, 430000, 440000, 450000],
    [510000, 520000, 530000, 540000, 550000, 560000],
    [0; 6],
];

pub struct MoveOrderer {
    killer_moves : [[ChessMove; NUM_KILLER_MOVES]; MAX_DEPTH as usize],
    history_info : [[[i32; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
    counter_move : [[ChessMove; NUM_SQUARES]; NUM_SQUARES],
    bb_utils : BitBoardUtils,
}

impl MoveOrderer {
    pub fn new() -> MoveOrderer {
        MoveOrderer {
            killer_moves : [[DUMMY_MOVE; NUM_KILLER_MOVES]; MAX_DEPTH as usize],
            history_info : [[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS],
            counter_move : [[DUMMY_MOVE; NUM_SQUARES]; NUM_SQUARES],
            bb_utils : BitBoardUtils::new(),
        }
    }

    pub fn update_killer_move(&mut self, depth : usize, killer_move : ChessMove) {
        for i in (1..NUM_KILLER_MOVES).rev() {
            self.killer_moves[depth][i] = self.killer_moves[depth][i-1];
        }
        self.killer_moves[depth][0] = killer_move;
    }

    pub fn update_history(&mut self, depth : i32, chess_move : ChessMove, color : Color) {
        let from = chess_move.get_source().to_index();
        let to = chess_move.get_dest().to_index();
        self.history_info[color.to_index()][from][to] += depth * depth;

        // TODO derate history
    }

    pub fn update_counter_move(&mut self, last_move : ChessMove, chess_move : ChessMove) {
        let from = last_move.get_source().to_index();
        let to = last_move.get_dest().to_index();

        self.counter_move[from][to] = chess_move;
    }

    pub fn reset_history(&mut self) {
        self.history_info = [[[0; NUM_SQUARES]; NUM_SQUARES]; NUM_COLORS];
    }

    fn is_capture(&self, board : &Board, chess_move : ChessMove) -> bool {
        let enemy_pieces = board.color_combined(!board.side_to_move());
        let sq_mask = self.bb_utils.square_mask[chess_move.dest.to_index()];
        sq_mask & enemy_pieces != EMPTY
    }

    fn get_history(&self, board : &Board, chess_move : ChessMove) -> i32 {
        let from = chess_move.get_source().to_index();
        let to = chess_move.get_dest().to_index();

        self.history_info[board.side_to_move().to_index()][from][to]
    }
    
}

pub struct MoveOrdering {
    moves : [ChessMove; MAX_MOVES],
    move_scores : [i32; MAX_MOVES],
    num_moves : usize,
}


impl MoveOrdering {
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
    
    fn calculate_score(&self, chess_move : ChessMove, board : &Board, depth : usize, move_orderer : &MoveOrderer, last_move : ChessMove) -> i32{
        for i in 0..NUM_KILLER_MOVES {
            if move_orderer.killer_moves[depth][i] == chess_move {
                return KILLER_MOVE_VALUE
            }
        }

        let mut score = 0;
        if !move_orderer.is_capture(board, chess_move) {
            score += move_orderer.get_history(board, chess_move);            
            let last_from = last_move.get_source().to_index();
            let last_to = last_move.get_dest().to_index();
            if chess_move == move_orderer.counter_move[last_from][last_to] {
                score += COUNTER_MOVE_BONUS;
            }

        } else{

            let captured_piece = board.piece_on(chess_move.get_dest()).unwrap();
            let capturing_piece = board.piece_on(chess_move.get_source()).unwrap();
            score += MVV_LVA_VALUES[captured_piece.to_index()][capturing_piece.to_index()];
        }

        score
    }

    pub fn get(&self, idx : usize) -> ChessMove {
        self.moves[idx]
    }

    fn swap_moves(&mut self, i : usize, j : usize) {
        let temp = self.moves[i];
        self.moves[i] = self.moves[j];
        self.moves[j] = temp;
    }

    pub fn get_next_best_move(&mut self, moves_processed : usize, board : &Board, depth : usize, tt: &TranspoTable, move_orderer : &MoveOrderer, last_move : ChessMove) -> ChessMove {
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
            let score = self.calculate_score(self.moves[i], board, depth, move_orderer, last_move);
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