
use chess::Board;
use chess::MoveGen;
use chess::ChessMove;
use chess::{Square, Color, EMPTY};
use std::alloc::System;
use std::time::{SystemTime, Duration};
use vampirc_uci::{UciTimeControl, UciSearchControl};
use crate::evaluation;
use crate::move_ordering;
use crate::move_ordering::MoveOrdering;
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
    time_control : Option<UciTimeControl>,
    search_control : Option<UciSearchControl>,
    end_time : SystemTime,
    is_following_pv : bool,
    in_null_move_prune : bool,
}
impl Search {
    pub fn new() -> Search {
        Search {
            cfg : Cfg{
                depth_left : 0,
            },
            end_time : SystemTime::now() + Duration::new(i32::MAX as u64, 0),
            time_control : None,
            search_control : None,
            is_following_pv : false,
            in_null_move_prune : false,
        }
    }

    fn calculate_expected_ply(&self, board : &Board) -> u32 {
        100
    }

    fn calculate_end_time(&mut self, board : &Board) {
        if self.cfg.depth_left != 0 {
            return
        }
        if self.time_control.is_some() {
            let now = SystemTime::now();
            let mut end_time = now + Duration::new(i32::MAX as u64, 0);
            match self.time_control.as_ref().unwrap() {
                UciTimeControl::MoveTime (duration) => {
                    end_time = SystemTime::now() + duration.to_std().expect("Out of range");
                },
                UciTimeControl::Infinite => (),
                UciTimeControl::TimeLeft { white_time, black_time, 
                    white_increment, black_increment, moves_to_go } =>
                    {
                        let mut time_left = Duration::new(0, 0);
                        match board.side_to_move() {
                            Color::White => {
                                if white_time.is_some() {
                                    time_left = white_time.unwrap().to_std().expect("Out of range");
                                }
                            },
                            Color::Black => {
                                if black_time.is_some() {
                                    time_left = black_time.unwrap().to_std().expect("Out of range");
                                }
                            },
                        }
                        let num_moves = self.calculate_expected_ply(board);
                        let ply_num = 0;
                        let moves_to_go = (num_moves - ply_num) / 2;
                        let move_time = time_left/moves_to_go;
                        println!("Calculated {} {moves_to_go}", move_time.as_millis());
                        end_time = now + move_time;
                   }
                _ => ()
            }
            self.end_time = end_time;
            self.cfg.depth_left = u32::MAX;
        }
    }

    pub fn set_time_controls(&mut self, time_controls : UciTimeControl) {
        self.time_control = Some(time_controls);
    }
    
    pub fn set_search_controls(&mut self, search_controls : Option<UciSearchControl>) {
        self.search_control = search_controls;
    }

    pub fn set_cfg_depth(&mut self, depth: u32){
        self.cfg.depth_left = depth;
    }

    pub fn get_best_move(&mut self, board : &Board, tt : &mut TranspoTable) -> ChessMove {  
        self.calculate_end_time(board);
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
            
            // At the start we follow the pv so we can display the whole thing 
            self.is_following_pv = true;
            let result = self.alphabeta(board, &alpha_beta_info, &mut pv_line, tt);
            if SystemTime::now() > self.end_time {
                break;
            }
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
            // As soon as we hit a leaf node, we can't be following the pv any more.
            self.is_following_pv = false;
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

        // TODO try to avoid calling this on every single search
        if SystemTime::now() > self.end_time {
            return SearchResult {
                eval : i32::MIN+10,
                nodes : 0
            }
        }

        // Try out null move pruning
        if !self.in_null_move_prune {
            if evaluation::total_material_eval(board) > 1000 { // Endgames can lead to zugzwang
                if (*board.checkers()) == EMPTY {
                    if depth > 3 {
                        self.in_null_move_prune = true;
                        let board_copy = board.clone();
                        board_copy.null_move();
                        let reduction_depth: i32 = depth / 4 + 3;
                        let inner_ab_info: AlphabetaInfo = AlphabetaInfo {
                            alpha : -beta,
                            beta : -alpha,
                            depth_left : depth - reduction_depth,
                            ply : alpha_beta_info.ply + 1,
                        };
                        let mut null_move_line: Line = Line {
                            cmove : 0,
                            chess_move : [DUMMY_MOVE; 100],
                        };
                        let result = self.alphabeta(&board_copy, &inner_ab_info, &mut null_move_line, tt);
                        if -result.eval >= beta {
                            return SearchResult {
                                eval : beta,
                                nodes : result.nodes
                            }
                        }
                    }
                }
            }
        }
        self.in_null_move_prune = false;

        // Create our inner line, generate the moves
        let mut line: Line = Line {
            cmove : 0,
            chess_move : [DUMMY_MOVE; 100],
        };

        // TODO Gen psuedo moves, check len(pseudo)
        let mut moves = MoveGen::new_legal(&board);
        let mut move_ordering = MoveOrdering::from_moves(&mut moves);

        // If there were no moves, that means it's draw or mate
        if move_ordering.len() == 0 {
            pv_line.cmove = 0;
            alpha = evaluation::eval(board, alpha_beta_info.ply);
        }


        

        let mut num_alpha_hits = 0;
        // Go through each move, internal alphabeta
        for i in 0..move_ordering.len() {
            let chess_move = move_ordering.get_next_best_move(i, board, tt);
            // let chess_move = moves
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