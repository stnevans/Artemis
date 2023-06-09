use chess::Board;
use chess::MoveGen;
use chess::ChessMove;
use chess::{Square, Color, EMPTY};
use std::time::{SystemTime, Duration};
use vampirc_uci::{UciTimeControl, UciSearchControl};
use crate::bb_utils::BitBoardUtils;
use crate::evaluation::{Evaluator, NAIVE_PIECE_VAL};
use crate::move_ordering::{MoveOrderer, MoveOrdering};
use crate::transpo::{TranspoTable, EntryFlags};

const MIN_ALPHA : i32 = i32::MIN + 500;
const MAX_BETA : i32 = i32::MAX - 500;
pub const MAX_DEPTH : u32 = 200;
pub const DUMMY_MOVE : ChessMove = ChessMove {
        source: Square::A1,
        dest: Square::A1,
        promotion: None
};
const FUTILITY_PRUNE_DEPTH : i32 = 3;
const FUTILITY_VALUES : [i32; (FUTILITY_PRUNE_DEPTH+1) as usize] = [0, 200, 300, 500];


const NULL_MOVE_MIN_REDUCTION : i32 = 3;
const DELTA_PRUNE_MAX : i32 = 900;
const DELTA_PRUNE_MATERIAL_CUTOFF : i32 = 1600;
pub struct Cfg {
    depth_left : u32,
}

struct AlphabetaInfo {
    alpha : i32,
    beta : i32,
    depth_left : i32,
    ply : u32,
    last_move : ChessMove,
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
    _search_control : Option<UciSearchControl>,
    end_time : SystemTime,
    is_following_pv : bool,
    in_null_move_prune : bool,
    nodes_evaled : u32,
    past_end_time : bool,
    move_orderer : MoveOrderer,
    evaluator : Evaluator,
    bb_utils : BitBoardUtils,
}
impl Search {
    pub fn new() -> Search {
        Search {
            cfg : Cfg{
                depth_left : 0,
            },
            end_time : SystemTime::now() + Duration::new(i32::MAX as u64, 0),
            time_control : None,
            _search_control : None,
            is_following_pv : false,
            in_null_move_prune : false,
            nodes_evaled : 0,
            past_end_time : false,
            move_orderer : MoveOrderer::new(),
            evaluator : Evaluator::new(),
            bb_utils : BitBoardUtils::new(),
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
                    white_increment, black_increment, .. } =>
                    {
                        let mut time_left = Duration::new(0, 0);
                        let mut increment = Duration::new(0, 0);
                        match board.side_to_move() {
                            Color::White => {
                                if white_time.is_some() {
                                    time_left = white_time.unwrap().to_std().expect("Out of range");
                                }
                                if white_increment.is_some() {
                                    increment = white_increment.unwrap().to_std().expect("Bad");
                                }
                            },
                            Color::Black => {
                                if black_time.is_some() {
                                    time_left = black_time.unwrap().to_std().expect("Out of range");
                                }
                                if white_increment.is_some() {
                                    increment = black_increment.unwrap().to_std().expect("Bad");
                                }
                            },
                        }
                        let num_moves = self.calculate_expected_ply(board);
                        let ply_num = 0;
                        let moves_to_go = (num_moves - ply_num) / 2;
                        let move_time = time_left/moves_to_go + increment;
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
    
    pub fn _set_search_controls(&mut self, search_controls : Option<UciSearchControl>) {
        self._search_control = search_controls;
    }

    pub fn set_cfg_depth(&mut self, depth: u32){
        self.cfg.depth_left = depth;
    }

    fn should_null_move_prune(&self, board : &Board, depth : i32) -> bool {
        if !self.in_null_move_prune {
            if self.evaluator.total_material_eval(board) > 1000 { // Endgames can lead to zugzwang
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
            last_move : DUMMY_MOVE,
        };
        let windowed_eval = self.alphabeta(board, &aspirated_ab_info, pv_line, tt).eval;
        let eval;
        // Check if the search actually ran into one of our bounds. If so, re-search
        if (windowed_eval <= last_eval - window_radius) || (windowed_eval >= last_eval + window_radius) {
            // println!("Re searching for {depth} {windowed_eval} {last_eval}");
            let full_alpha_beta_range = AlphabetaInfo {
                alpha : MIN_ALPHA,
                beta : MAX_BETA,
                depth_left : depth as i32,
                ply : 0,
                last_move : DUMMY_MOVE,
            };
            eval = self.alphabeta(board, &full_alpha_beta_range, pv_line, tt).eval;
        } else {
            eval = windowed_eval;
        }
        eval
    }

    fn iterative_deepening(&mut self, board : &Board, tt : &mut TranspoTable) -> (ChessMove, i32) {
        self.nodes_evaled = 0;
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
                last_move : DUMMY_MOVE, 
            };
            
            // At the start we follow the pv so we can display the whole thing 
            self.is_following_pv = true;

            // Aspiration window here
            if depth > 1 {
                eval = self.aspirated_search(board, eval, depth as i32, &mut pv_line, tt);
            } else {
                let result = self.alphabeta(board, &full_alpha_beta_range, &mut pv_line, tt);
                eval = result.eval;
            }
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

            print!("info depth {depth} score ");
            if self.evaluator.eval_is_mate(eval) {
                print!("mate {} ", self.evaluator.eval_distance_to_mate(eval));
            } else {
                print!("cp {eval} ");
            }
            let nodes = self.nodes_evaled;
            let duration_millis = u128::max(search_duration.as_millis(), 1);
            

            let nps = (nodes as f64 / (duration_millis as f64 / 1000.0)) as u64;

            println!("time {duration_millis} nodes {nodes} pv {pv_string} nps {nps}");
            
        }
        self.past_end_time = false;
        self.move_orderer.reset_history();
        (best_move, eval)
    }

    fn is_capture(&self, board : &Board, chess_move : ChessMove) -> bool {
        let enemy_pieces = board.color_combined(!board.side_to_move());
        let sq_mask = self.bb_utils.square_mask[chess_move.dest.to_index()];
        sq_mask & enemy_pieces != EMPTY
    }

    fn should_futility_prune_position(&self, board : &Board, depth : i32, ply : u32, beta : i32, eval : i32) -> bool {
        if depth <= FUTILITY_PRUNE_DEPTH && ply > 1 {
            if (*board.checkers()) == EMPTY {
                return eval - FUTILITY_VALUES[depth as usize] > beta; 
            }
        }
        false
    }

    fn should_futility_prune_move(&self, board : &Board, is_move_check : bool, chess_move : ChessMove, index : usize, 
        depth : i32, ply : u32, alpha : i32, position_eval : i32) -> bool {
        if depth > FUTILITY_PRUNE_DEPTH  || ply < 1 || (*board.checkers()) == EMPTY {
            return false
        }

        if index == 0  {
            return false
        }

        if is_move_check {
            return false
        }

        // TODO
        // Don't futility move in endgame for now. This is because I'm worried about promos
        if self.evaluator.total_material_eval(board) < 1000 {
            return false
        }

        // TODO see
        if self.is_capture(board, chess_move) {
            return false
        }
        position_eval + FUTILITY_VALUES[depth as usize] < alpha        
    }

    fn should_delta_prune(&self, board : &Board, eval : i32, total_material : i32, capture : ChessMove, alpha : i32) -> bool {
        if total_material > DELTA_PRUNE_MATERIAL_CUTOFF {
            let captured_piece = board.piece_on(capture.get_dest()).unwrap();
            if eval + NAIVE_PIECE_VAL[captured_piece.to_index()] < alpha {
                return true
            }
        }
        false
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

            let eval = self.quiesce(board, alpha_beta_info, tt);
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
                if self.evaluator.eval_is_mate(eval) {
                    if eval < 0 {
                        eval += alpha_beta_info.ply as i32;
                    } else {
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

        if self.nodes_evaled % 1000 == 0 {
            if SystemTime::now() > self.end_time {
                self.past_end_time = true;
                return SearchResult {
                    eval : i32::MIN+10,
                }
            }
        }

        // Try out null move pruning
        if self.should_null_move_prune(board, depth) {
            self.in_null_move_prune = true;
            let board_copy = board.null_move().unwrap();
            let reduction_depth: i32 = depth / 4 + NULL_MOVE_MIN_REDUCTION;
            let inner_ab_info: AlphabetaInfo = AlphabetaInfo {
                alpha : -beta,
                beta : -alpha,
                depth_left : depth - reduction_depth,
                ply : alpha_beta_info.ply + 1,
                last_move : DUMMY_MOVE,
            };
            let mut null_move_line: Line = Line {
                cmove : 0,
                chess_move : [DUMMY_MOVE; 100],
            };

            let result = self.alphabeta(&board_copy, &inner_ab_info, &mut null_move_line, tt);
            let eval = -result.eval;
            if eval >= beta {
                return SearchResult {
                    eval : beta,
                }
            }
        }
        self.in_null_move_prune = false;


        // Try to futility prune based on the position
        let position_eval = self.evaluator.eval(board, alpha_beta_info.ply);
        if self.should_futility_prune_position(board, depth, alpha_beta_info.ply, beta, position_eval) {
            return SearchResult {
                eval : beta
            }
        }

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
            alpha = self.evaluator.eval(board, alpha_beta_info.ply);
        }

        let mut num_alpha_hits = 0;
        // Go through each move, internal alphabeta
        for i in 0..move_ordering.len() {            
            let chess_move = move_ordering.get_next_best_move(i, board, depth as usize, tt, &self.move_orderer, alpha_beta_info.last_move);

            let inner_ab_info: AlphabetaInfo = AlphabetaInfo {
                alpha : -beta,
                beta : -alpha,
                depth_left : depth - 1,
                ply : alpha_beta_info.ply + 1,
                last_move : chess_move,
            };


            // test a move
            let new_board: Board = board.make_move_new(chess_move);
            let is_move_check = (*new_board.checkers()) != EMPTY;
            if self.should_futility_prune_move(board, is_move_check, chess_move, i, depth, alpha_beta_info.ply, alpha, position_eval) {
                continue;
            }

            let inner_result = self.alphabeta(&new_board, &inner_ab_info, &mut line, tt);
            let score = -inner_result.eval;
            // Score >= beta means refutation was found (i.e we know we worst case eval is -200. this move gives eval of > that)
            if score >= beta {
                if !self.past_end_time {
                    tt.save(board.get_hash(), beta, EntryFlags::Beta, chess_move, depth as u8, alpha_beta_info.ply as u8);
                    self.move_orderer.update_killer_move(depth as usize, chess_move);
                    
                    if !self.is_capture(board, chess_move) {
                        self.move_orderer.update_history(depth, chess_move, board.side_to_move());
                        self.move_orderer.update_counter_move(alpha_beta_info.last_move, chess_move);
                    }
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


    fn quiesce(&mut self, board : &Board, alpha_beta_info : &AlphabetaInfo, tt : &TranspoTable) -> i32 {
        // Do our initial eval and check cutoffs
        let mut alpha = alpha_beta_info.alpha;
        let beta = alpha_beta_info.beta;
        let initial_eval = self.evaluator.eval(board, alpha_beta_info.ply);
        if initial_eval >= beta {
            return alpha_beta_info.beta
        }

        // Do initial delta pruning
        if initial_eval + DELTA_PRUNE_MAX < alpha {
            return alpha;
        }

        if initial_eval > alpha {        
            alpha = initial_eval;
        }

        // Do our captures and keep searching
        // TODO BUG NO ENPASSANT IN THIS
        let mut moves = MoveGen::new_legal(&board);
        let targets = board.color_combined(!board.side_to_move());
        moves.set_iterator_mask(*targets);

        let mut move_ordering = MoveOrdering::from_moves(&mut moves);
        let total_material = self.evaluator.total_material_eval(board);

        for i in 0..move_ordering.len() {
            let capture = move_ordering.get_next_best_move(i, board, 0, tt, &self.move_orderer, alpha_beta_info.last_move);

            if self.should_delta_prune(board, initial_eval, total_material, capture, alpha) {
                continue;
            }
            let inner_ab_info: AlphabetaInfo = AlphabetaInfo {
                alpha : -beta,
                beta : -alpha,
                depth_left : 0,
                ply : alpha_beta_info.ply + 1,
                last_move : capture,
            };


            let new_board: Board = board.make_move_new(capture);
            let score = -self.quiesce(&new_board, &inner_ab_info, tt);

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