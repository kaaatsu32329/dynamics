//! CLI that draws the first-order system dx/dt = f(x) from an equation.
//!
//! Usage:
//!   dyn-cli "x - x^3"
//!   dyn-cli "r - x^2" --r 1 --out saddle.png
//!   dyn-cli "x*(1-x)" --gif --out logistic.gif --frames 120
//!
//! Options:
//!   -o, --out <PATH>     output file (.png / .gif)           [default: first_order.png]
//!       --xmin/--xmax    displayed x range                   [default: -2 / 2]
//!       --tmax <T>       integration time                    [default: 8]
//!       --n <N>          number of trajectories (ICs)        [default: 15]
//!       --p/--q/--r <V>  parameter values                    [default: 0]
//!       --size <WxH>     image size                          [default: 1300x460]
//!       --gif            output an animated GIF
//!       --frames <N>     number of GIF frames                [default: 120]

mod figure;
mod font;

use dyn_core::{Model, Params};

struct Cli {
    eq: String,
    out: String,
    params: Params,
    size: (u32, u32),
    gif: bool,
    frames: usize,
}

fn parse_f64(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<f64, String> {
    it.next()
        .ok_or_else(|| format!("{flag} に値が必要です"))?
        .parse()
        .map_err(|_| format!("{flag} の値が数値ではありません"))
}

fn parse_args() -> Result<Cli, String> {
    let mut eq: Option<String> = None;
    let mut out = String::from("first_order.png");
    let mut p = Params::default();
    let mut size = (1300u32, 460u32);
    let mut gif = false;
    let mut frames = 120usize;

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{}", HELP);
                std::process::exit(0);
            }
            "-o" | "--out" => out = it.next().ok_or("--out に値が必要です")?,
            "--xmin" => p.xmin = parse_f64(&mut it, "--xmin")?,
            "--xmax" => p.xmax = parse_f64(&mut it, "--xmax")?,
            "--tmax" => p.tmax = parse_f64(&mut it, "--tmax")?,
            "--p" => p.vals[0] = parse_f64(&mut it, "--p")?,
            "--q" => p.vals[1] = parse_f64(&mut it, "--q")?,
            "--r" => p.vals[2] = parse_f64(&mut it, "--r")?,
            "--n" => p.n_traj = parse_f64(&mut it, "--n")?.max(1.0) as usize,
            "--frames" => frames = parse_f64(&mut it, "--frames")?.max(2.0) as usize,
            "--size" => {
                let s = it.next().ok_or("--size に WxH が必要です")?;
                let (w, h) = s
                    .split_once(['x', 'X', ','])
                    .ok_or("--size は WxH 形式 (例 1300x460)")?;
                size = (
                    w.trim().parse().map_err(|_| "幅が不正")?,
                    h.trim().parse().map_err(|_| "高さが不正")?,
                );
            }
            "--gif" => gif = true,
            other if other.starts_with('-') => return Err(format!("不明なオプション: {other}")),
            other => eq = Some(other.to_string()),
        }
    }

    if p.xmax <= p.xmin {
        return Err("x 範囲が不正です (--xmin < --xmax)".into());
    }
    Ok(Cli {
        eq: eq.unwrap_or_else(|| "x - x^3".to_string()),
        out,
        params: p,
        size,
        gif,
        frames,
    })
}

const HELP: &str = "\
dyn-cli — 一次の力学系 dx/dt = f(x) を図示する

USAGE:
  dyn-cli [EQUATION] [OPTIONS]

例:
  dyn-cli \"x - x^3\"
  dyn-cli \"r - x^2\" --r 1 -o saddle.png
  dyn-cli \"x*(1-x)\" --gif -o logistic.gif

OPTIONS:
  -o, --out <PATH>   出力 (.png / .gif)        [default: first_order.png]
      --xmin <V>     x 下限                    [default: -2]
      --xmax <V>     x 上限                    [default: 2]
      --tmax <T>     積分時間                  [default: 8]
      --n <N>        軌道本数                  [default: 15]
      --p/--q/--r    パラメータ p, q, r        [default: 0]
      --size <WxH>   画像サイズ                [default: 1300x460]
      --gif          アニメーション GIF を出力
      --frames <N>   GIF フレーム数            [default: 120]
  -h, --help         このヘルプ";

fn main() {
    let cli = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("エラー: {e}\n\n{HELP}");
            std::process::exit(2);
        }
    };

    let model = match Model::build(&cli.eq, cli.params) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("式を解釈できません: {e}");
            std::process::exit(1);
        }
    };

    // Print a summary of the fixed points to stdout.
    println!("dx/dt = {}", cli.eq);
    let used = model.expr.used_params();
    let shown: Vec<String> = dyn_core::PARAM_NAMES
        .iter()
        .enumerate()
        .filter(|(i, _)| used[*i])
        .map(|(i, name)| format!("{name} = {}", cli.params.vals[i]))
        .collect();
    if !shown.is_empty() {
        println!("  ({})", shown.join(", "));
    }
    if model.fixed_points.is_empty() {
        println!("  固定点なし (この範囲では流れは一方向)");
    } else {
        for fp in &model.fixed_points {
            let k = match fp.stability {
                dyn_core::Stability::Stable => "安定",
                dyn_core::Stability::Unstable => "不安定",
                dyn_core::Stability::SemiStable => "半安定",
            };
            println!("  x* = {:+.4}  [{k}]  f'(x*) = {:+.3}", fp.x, fp.slope);
        }
    }

    if let Err(e) = font::register() {
        eprintln!("フォント登録エラー: {e}");
        std::process::exit(1);
    }

    let res = if cli.gif {
        figure::render_gif(&model, &cli.out, cli.size, cli.frames)
    } else {
        figure::render_png(&model, &cli.out, cli.size)
    };

    match res {
        Ok(()) => println!("=> {}", cli.out),
        Err(e) => {
            eprintln!("描画エラー: {e}");
            std::process::exit(1);
        }
    }
}
