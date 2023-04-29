
use chess::Board;
use chess::MoveGen;
use chess::ChessMove;
use chess::Square;
use std::fmt::Debug;
use std::time::{SystemTime};


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

pub fn get_default_cfg(depth: u32) -> Cfg {
    Cfg { depth_left: depth }
}

// TOdo cfg
pub fn get_best_move(board : &Board, cfg: &Cfg) -> ChessMove {   
    return iterative_deepening(board, &cfg).0;
}



fn iterative_deepening(board : &Board, cfg : &Cfg) -> (ChessMove, i32) {
    let mut best_move : ChessMove = DUMMY_MOVE;
    let mut eval = 0;

    for depth in 1..=cfg.depth_left {
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
        
        let result = alphabeta(board, &alpha_beta_info, &mut pv_line);
        
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
        println!("info depth {depth} score cp {eval} time {duration_millis} nodes {nodes} pv {pv_string}");
    }
    (best_move, eval)
}

fn alphabeta(board : &Board, alpha_beta_info : &AlphabetaInfo, pv_line : &mut Line) -> SearchResult {
    // Init variables
    let mut alpha = alpha_beta_info.alpha;
    let mut nodes = 0;

    let depth = alpha_beta_info.depth_left;
    let beta = alpha_beta_info.beta;

    let mut line: Line = Line {
        cmove : 0,
        chess_move : [DUMMY_MOVE; 100],
    };

    // Depth 0, quiesce
    if depth <= 0 {
        pv_line.cmove = 0;
        
        return SearchResult {
            eval: quiesce(board, alpha_beta_info),
            nodes: 1
        }
    }

    let moves = MoveGen::new_legal(&board);
    
    // Go through each move, internal alphabeta
    for chess_move in moves {
        let inner_ab_info: AlphabetaInfo = AlphabetaInfo {
            alpha : -beta,
            beta : -alpha,
            depth_left : depth - 1,
            ply : alpha_beta_info.ply + 1,
        };

        // make move
        let new_board: Board = board.make_move_new(chess_move);
        let inner_result = alphabeta(&new_board, &inner_ab_info, &mut line);
        let score = -inner_result.eval;
        nodes += inner_result.nodes;

        // Score >= beta means refutation was found (i.e we know we worst case eval is -200. this move gives eval of > that)
        if score >= beta {
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
        }
    }

    SearchResult{
        eval : alpha,
        nodes : nodes,
    }
}


fn quiesce(board : &Board, alpha_beta_info : &AlphabetaInfo) -> i32 {
    // Do our initial eval and check cutoffs
    let mut alpha = alpha_beta_info.alpha;
    let eval = crate::evaluation::eval(board, alpha_beta_info.ply);
    if eval >= alpha_beta_info.beta {
        return alpha_beta_info.beta
    }

    if eval < alpha {
        alpha = eval;
    }

    // If we have any captures, redo quiesce search


    alpha
}
