
use chess::Board;
use chess::MoveGen;
use chess::ChessMove;
use chess::Square;
use std::time::{SystemTime};

use crate::evaluation;
use crate::transpo::{TranspoTable, EntryFlags};

const MIN_ALPHA : i32 = i32::MIN + 500;
const MIN_BETA : i32 = i32::MAX - 500;
const DUMMY_MOVE : ChessMove = ChessMove {
        source: Square::A1,
        dest: Square::A1,
        promotion: None
};

pub struct Cfg {
    depth_left : u32,
}

struct AlphabetaInfo {
    alpha : i32,
    beta : i32,
    depth_left : i32,
    ply : u32,
}

struct SearchResult {
    nodes : u32,
    eval : i32,
}

struct Line {
    cmove : u32,
    chess_move : [ChessMove; 100], //todo unsafe uninit
}

pub struct Search {
    cfg : Cfg,
}
impl Search {
    pub fn new() -> Search {
        Search {
            cfg : Cfg{
                depth_left : 1,
            }
        }
    }

    pub fn set_cfg_depth(&mut self, depth: u32){
        self.cfg.depth_left = depth;
    }

    pub fn get_best_move(&mut self, board : &Board, tt : &mut TranspoTable) -> ChessMove {   
        return self.iterative_deepening(board, tt).0;
    }

    fn iterative_deepening(&mut self, board : &Board, tt : &mut TranspoTable) -> (ChessMove, i32) {
        let mut best_move : ChessMove = DUMMY_MOVE;
        let mut eval = 0;

        for depth in 1..=self.cfg.depth_left {
            let mut pv_line = Line {
                cmove : 0,
                chess_move : [DUMMY_MOVE; 100],
            };

            let alpha_beta_info = AlphabetaInfo {
                alpha : MIN_ALPHA,
                beta : MIN_BETA,
                depth_left : depth as i32,
                ply : 0,
            };
            let search_start_time = SystemTime::now();
            
            let result = self.alphabeta(board, &alpha_beta_info, &mut pv_line, tt);
            
            let nodes = result.nodes;
            eval = result.eval;

            let search_duration = search_start_time.elapsed().unwrap();
            best_move = pv_line.chess_move[0];
            
            let mut pv_string = String::new();
            
            for i in 0..pv_line.cmove {
                pv_string.push_str(&pv_line.chess_move[i as usize].to_string());
                pv_string.push_str(" ");
            }

            let duration_millis = search_duration.as_millis();
            print!("info depth {depth} score ");
            if evaluation::eval_is_mate(eval) {
                print!("mate {} ", evaluation::eval_distance_to_mate(eval));
            } else {
                print!("cp {eval} ");
            }
            println!("time {duration_millis} nodes {nodes} pv {pv_string}");
        }
        (best_move, eval)
    }

    fn alphabeta(&mut self, board : &Board, alpha_beta_info : &AlphabetaInfo, pv_line : &mut Line, tt : &mut TranspoTable) -> SearchResult {
        // Init variables
        let mut alpha = alpha_beta_info.alpha;
        let mut nodes = 0;

        let depth = alpha_beta_info.depth_left;
        let beta = alpha_beta_info.beta;

        // Depth 0, quiesce
        if depth <= 0 {
            pv_line.cmove = 0;
            let eval = self.quiesce(board, alpha_beta_info);
            return SearchResult {
                eval: eval,
                nodes: 1
            }
        }

        // Check our transpo table
        let entry = tt.probe(board.get_hash());
        if entry.hash == board.get_hash() {
            if entry.depth as i32 >= depth {
                let mut eval = entry.eval;
                if evaluation::eval_is_mate(eval) {
                    if eval < 0 {
                        eval += alpha_beta_info.ply as i32;
                    } else{
                        eval -= alpha_beta_info.ply as i32;
                    }
                }
                match entry.flags {
                    EntryFlags::Exact => {
                        // We know the exact eval.
                        pv_line.cmove = 1;
                        pv_line.chess_move[0] = entry.best_move;
                        return SearchResult {
                            eval : eval,
                            nodes : 1
                        }
                    },
                    EntryFlags::Beta => (),
                    EntryFlags::Alpha => (),
                }
            }
        }

        // Create our inner line, generate the moves
        let mut line: Line = Line {
            cmove : 0,
            chess_move : [DUMMY_MOVE; 100],
        };

        // TODO Gen psuedo moves, check len(pseudo)
        let moves = MoveGen::new_legal(&board);

        // If there were no moves, that means it's draw or mate
        if moves.len() == 0 {
            pv_line.cmove = 0;
            alpha = evaluation::eval(board, alpha_beta_info.ply);
        }

        let mut num_alpha_hits = 0;
        // Go through each move, internal alphabeta
        for chess_move in moves {
            let inner_ab_info: AlphabetaInfo = AlphabetaInfo {
                alpha : -beta,
                beta : -alpha,
                depth_left : depth - 1,
                ply : alpha_beta_info.ply + 1,
            };


            // test a move
            let new_board: Board = board.make_move_new(chess_move);
            let inner_result = self.alphabeta(&new_board, &inner_ab_info, &mut line, tt);
            let score = -inner_result.eval;
            nodes += inner_result.nodes;


            // Score >= beta means refutation was found (i.e we know we worst case eval is -200. this move gives eval of > that)
            if score >= beta {
                tt.save(board.get_hash(), beta, EntryFlags::Beta, chess_move, depth as u8, alpha_beta_info.ply as u8);
                return SearchResult {
                    eval : beta,
                    nodes: nodes
                }
            }

            // Score > alpha means we have a new best move
            if score > alpha {
                pv_line.chess_move[0] = chess_move;
                for i in 1..100 {
                    pv_line.chess_move[i] = line.chess_move[i - 1];
                }
                pv_line.cmove = line.cmove + 1;
                alpha = score;

                num_alpha_hits += 1;
            }
        }
        
        if num_alpha_hits != 0 {
            // We got the exact eval for the position, not just an alpha lower bound
        
            tt.save(board.get_hash(), alpha, EntryFlags::Exact, pv_line.chess_move[0], depth as u8, alpha_beta_info.ply as u8);
        } else {
            // We got an alpha lower bound. This means every child had a beta cutoff
            // So the pv_line is not updated
            // tt.save(board.get_hash(), alpha, EntryFlags::Alpha, pv_line.chess_move[0], depth as u8, alpha_beta_info.ply as u8);
        }

        SearchResult{
            eval : alpha,
            nodes : nodes,
        }
    }


    fn quiesce(&mut self, board : &Board, alpha_beta_info : &AlphabetaInfo) -> i32 {
        // Do our initial eval and check cutoffs
        let mut alpha = alpha_beta_info.alpha;
        let beta = alpha_beta_info.beta;
        let eval = crate::evaluation::eval(board, alpha_beta_info.ply);
        if eval >= beta {
            return alpha_beta_info.beta
        }

        if eval > alpha {        
            alpha = eval;
        }

        // Do our captures and keep searching
        // TODO BUG NO ENPASSANT IN THIS
        let mut moves = MoveGen::new_legal(&board);
        let targets = board.color_combined(!board.side_to_move());
        moves.set_iterator_mask(*targets);
        for capture in &mut moves {

            let inner_ab_info: AlphabetaInfo = AlphabetaInfo {
                alpha : -beta,
                beta : -alpha,
                depth_left : 0,
                ply : alpha_beta_info.ply + 1,
            };


            let new_board: Board = board.make_move_new(capture);
            let score = -self.quiesce(&new_board, &inner_ab_info);

            if score >= beta {
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }


        alpha
    }

}