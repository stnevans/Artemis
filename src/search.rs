
use chess::Board;
use chess::MoveGen;
use chess::ChessMove;
use chess::{Square, Color, EMPTY};
use std::time::{SystemTime, Duration};
use vampirc_uci::{UciTimeControl, UciSearchControl};
use crate::evaluation;
use crate::move_ordering::{MoveOrderer, MoveOrdering};
use crate::transpo::{TranspoTable, EntryFlags};

const MIN_ALPHA : i32 = i32::MIN + 500;
const MAX_BETA : i32 = i32::MAX - 500;
pub const MAX_DEPTH : u32 = 200;
const DUMMY_MOVE : ChessMove = ChessMove {
        source: Square::A1,
        dest: Square::A1,
        promotion: None
};

const NULL_MOVE_MIN_REDUCTION : i32 = 3;
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
    nodes_evaled : u32,
    past_end_time : bool,
    move_orderer : MoveOrderer,
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
            nodes_evaled : 0,
            past_end_time : false,
            move_orderer : MoveOrderer::new(),
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

    fn should_null_move_prune(&self, board : &Board, depth : i32) -> bool {
        if !self.in_null_move_prune {
            if evaluation::total_material_eval(board) > 1000 { // Endgames can lead to zugzwang
                if (*board.checkers()) == EMPTY {
                    if depth > NULL_MOVE_MIN_REDUCTION {
                        return true
                    }
                }
            }
        }
        false
    }

    pub fn get_best_move(&mut self, board : &Board, tt : &mut TranspoTable) -> ChessMove {  
        self.calculate_end_time(board);
        return self.iterative_deepening(board, tt).0;
    }

    fn aspirated_search(&mut self, board : &Board, last_eval : i32, depth : i32, pv_line : &mut Line ,tt : &mut TranspoTable) -> i32{
        // For some reason this is not working at all :(
        let window_radius = 50;
        let aspirated_ab_info = AlphabetaInfo{
            alpha : last_eval - window_radius,
            beta : last_eval + window_radius,
            depth_left : depth as i32,
            ply : 0,
        };
        let windowed_eval = self.alphabeta(board, &aspirated_ab_info, pv_line, tt).eval;
        let mut eval;
        // Check if the search actually ran into one of our bounds. If so, re-search
        if (windowed_eval <= last_eval - window_radius) || (windowed_eval >= last_eval + window_radius) {
            println!("Re searching for {depth} {windowed_eval} {last_eval}");
            let full_alpha_beta_range = AlphabetaInfo {
                alpha : MIN_ALPHA,
                beta : MAX_BETA,
                depth_left : depth as i32,
                ply : 0,
            };
            eval = self.alphabeta(board, &full_alpha_beta_range, pv_line, tt).eval;
        } else {
            eval = windowed_eval;
        }
        eval
    }

    fn iterative_deepening(&mut self, board : &Board, tt : &mut TranspoTable) -> (ChessMove, i32) {
        let mut best_move : ChessMove = DUMMY_MOVE;
        let mut eval: i32 = 0;
        let search_start_time: SystemTime = SystemTime::now();

        for depth in 1..=self.cfg.depth_left {
            if depth > MAX_DEPTH {
                break;
            }
            let mut pv_line = Line {
                cmove : 0,
                chess_move : [DUMMY_MOVE; 100],
            };

            let full_alpha_beta_range = AlphabetaInfo {
                alpha : MIN_ALPHA,
                beta : MAX_BETA,
                depth_left : depth as i32,
                ply : 0,
            };
            
            // At the start we follow the pv so we can display the whole thing 
            self.is_following_pv = true;

            // Aspiration window here
            // if depth > 1 {
            //     eval = self.aspirated_search(board, eval, depth as i32, &mut pv_line, tt);
            // } else {
            let result = self.alphabeta(board, &full_alpha_beta_range, &mut pv_line, tt);
            eval = result.eval;
            // }
            if SystemTime::now() > self.end_time {
                break;
            }

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
                print!("mate {} cp {eval}", evaluation::eval_distance_to_mate(eval));
            } else {
                print!("cp {eval} ");
            }
            let nodes = self.nodes_evaled;
            println!("time {duration_millis} nodes {nodes} pv {pv_string}");
            
        }
        self.past_end_time = false;
        (best_move, eval)
    }

    fn alphabeta(&mut self, board : &Board, alpha_beta_info : &AlphabetaInfo, pv_line : &mut Line, tt : &mut TranspoTable) -> SearchResult {
        // Init variables
        let mut alpha = alpha_beta_info.alpha;

        let depth = alpha_beta_info.depth_left;
        let beta = alpha_beta_info.beta;

        // Depth 0, quiesce
        if depth <= 0 {
            // As soon as we hit a leaf node, we can't be following the pv any more.
            self.is_following_pv = false;
            pv_line.cmove = 0;
            let eval = self.quiesce(board, alpha_beta_info);
            self.nodes_evaled += 1;
            return SearchResult {
                eval: eval,
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
                        }
                    },
                    EntryFlags::Beta => {
                        // In the past, this node caused a beta cutoff. Check if it would do the same here
                        if eval >= beta {
                            pv_line.cmove = 1;
                            pv_line.chess_move[0] = entry.best_move;
                            return SearchResult {
                                eval : beta
                            }
                        }
                    },
                    EntryFlags::Alpha => {
                        // In the past, we returned alpha for this node, meaning we couldn't beat the lower bound we used to have
                        // Check if it is worse than our lower bound still
                        if eval <= alpha {
                            pv_line.cmove = 1;
                            pv_line.chess_move[0] = entry.best_move;
                            return SearchResult {
                                eval : alpha
                            }
                        }
                    },
                }
            }
        }

        // TODO try to avoid calling this on every single search
        if SystemTime::now() > self.end_time {
            self.past_end_time = true;
            return SearchResult {
                eval : i32::MIN+10,
            }
        }

        // Try out null move pruning
        if self.should_null_move_prune(board, depth) {
            self.in_null_move_prune = true;
            let board_copy = board.clone();
            board_copy.null_move();
            let reduction_depth: i32 = depth / 4 + NULL_MOVE_MIN_REDUCTION;
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
            let chess_move = move_ordering.get_next_best_move(i, board, depth as usize, tt, &self.move_orderer);
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


            // Score >= beta means refutation was found (i.e we know we worst case eval is -200. this move gives eval of > that)
            if score >= beta {
                if !self.past_end_time {
                    tt.save(board.get_hash(), beta, EntryFlags::Beta, chess_move, depth as u8, alpha_beta_info.ply as u8);
                    self.move_orderer.update_killer_move(depth as usize, chess_move);
                }
                return SearchResult {
                    eval : beta,
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
            // We got an alpha lower bound. This means none of the moves were better than the lower bound.
            // Call the pv move the best
            tt.save(board.get_hash(), alpha, EntryFlags::Alpha, move_ordering.get(0), depth as u8, alpha_beta_info.ply as u8);
        }

        SearchResult{
            eval : alpha,
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