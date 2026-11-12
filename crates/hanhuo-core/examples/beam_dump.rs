//! 役N tick21: 束池 dump（对拍 py beam_dump_py.py——平局分叉层定位:
//! 池层一致 → 分叉在 face/kbest 层; 池层不一致 → 束内层）
use hanhuo_core::{Engine, default_bin};

fn main() {
    let e = Engine::open(&default_bin()).unwrap();
    for q in std::env::args().skip(1) {
        let pool = e.beam_pool_dbg(&q);
        println!("== {q} pool={}", pool.len());
        for (i, ent) in pool.iter().enumerate() {
            println!(
                "{i:3} letters={} syls={} spens={:?} frd={} hk={} nj={} miss={}",
                ent.2,
                ent.3.join(","),
                ent.4,
                ent.5,
                ent.6 as char,
                ent.7,
                ent.8
            );
        }
    }
}
