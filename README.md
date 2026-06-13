# 一次の力学系ビジュアライザ (Rust)

一次（1 次元）の自励系

```
dx/dt = f(x)
```

の挙動を可視化する Rust 製ワークスペース。任意の関数 `f(x)` を文字列で与えると、
**f(x) のグラフ・相直線（流れの向き）・固定点の安定性・時系列 x(t)** を描画する。

1 次元系では、解の挙動はすべて相直線上で理解できる:

- `f(x) = 0` の点が **固定点**（平衡点）。
- `f(x) > 0` の区間では x は増加（→）、`f(x) < 0` では減少（←）。
- 固定点での傾きが安定性を決める: `f'(x*) < 0` なら **安定**（●）、`f'(x*) > 0` なら **不安定**（○）。

![bistable](gallery/bistable.png)

## ワークスペース構成

| クレート | 種別 | 役割 | 依存 |
|----------|------|------|------|
| [`dyn-core`](crates/dyn-core) | lib | 数式パーサ・固定点検出・安定性判定・RK4 積分 | **依存ゼロ** |
| [`dyn-wasm`](crates/dyn-wasm) | cdylib | `dyn-core` を WebAssembly に公開（**ブラウザ版の計算エンジン**） | `wasm-bindgen` |
| [`dyn-cli`](crates/dyn-cli)   | bin | 方程式 → PNG / アニメーション GIF | `plotters` |

`dyn-core` は標準ライブラリのみに依存し、`no_std` 化や組込み用途への転用も視野に入る薄い数値コア。
ブラウザ版・CLI はいずれもこの同じコアを消費する薄い presentation 層。
**ブラウザ版が主役**で、数式パース・固定点検出・RK4 積分という本質ロジックは Rust(WASM) が実行し、
描画（canvas）と UI（HTML/CSS）のみが JavaScript。

## 使い方

### ブラウザ版（推奨）

```sh
./run.sh
```

これ一つで完結する:

1. wasm ターゲットと `wasm-bindgen-cli`（`Cargo.lock` と同版）が無ければ自動インストール
2. 未ビルド / ソースが更新されていれば WebAssembly を再ビルド
3. ローカル HTTP サーバを起動し、**既定ブラウザでページを開く**（Ctrl-C で停止＆後始末）

ポートは自動選択（`./run.sh 8080` で指定可）。方程式を入力（または例ボタン）すると即座に
再描画。パラメータ `p` `q` `r` を含む式（例 `r - x^2`、`r*x*(1 - x/q)`、`p + r*x - x^3`）では
含むものだけスライダーが専用行に現れ、分岐（固定点の生成・消滅）を動かして観察できる。
パラメータを含む式では **分岐図**（固定点 x* を掃引パラメータに対して描き、安定＝緑/不安定＝赤、
現在値を縦線で表示）も自動で表示される（複数パラメータ時は掃引対象を選択可）。
`▶ アニメーション再生` で粒子が相直線上を流れ安定固定点へ収束する
様子を表示。計算（パース・固定点・積分）はすべて Rust(WASM)、描画は canvas。

ビルドだけ・配信だけしたい場合は `./build-web.sh`（`--serve` で 8000 番配信）。

> `file://` で直接開くと ES モジュール/WASM の取得が CORS で失敗するため、HTTP 配信が必要。
> `web/pkg/` は `build-web.sh`（= `cargo build --target wasm32-unknown-unknown` + `wasm-bindgen`）が生成する。

### CLI（静止画 / GIF）

```sh
# 静止画
cargo run -p dyn-cli -- "x - x^3"

# パラメータ付き（サドルノード分岐）
cargo run -p dyn-cli -- "r - x^2" --r 1 -o saddle.png
cargo run -p dyn-cli -- "r*x*(1 - x/q)" --r 1.2 --q 0.8   # 複数パラメータ p/q/r

# アニメーション GIF
cargo run -p dyn-cli -- "x*(1-x)" --gif -o logistic.gif --frames 120
```

主なオプション: `--xmin/--xmax`（x 範囲）, `--tmax`（積分時間）, `--n`（軌道本数）,
`--p`/`--q`/`--r`（パラメータ）, `--size WxH`, `--gif`, `--frames`。`-h` で一覧。

## デプロイ（GitHub Pages）

`.github/workflows/deploy.yml` が、`main` への push ごとに

1. `dyn-core` をテスト
2. `dyn-wasm` を `wasm32-unknown-unknown` でビルド
3. `Cargo.lock` と同じ版の `wasm-bindgen-cli` で JS グルーを生成（`web/pkg`）
4. `web/` を **GitHub Pages** に公開

します。初回 push 時に Pages を自動で有効化（`configure-pages` の `enablement`）。
HTML/WASM はすべて相対パスなので、プロジェクトページ
`https://<ユーザー名>.github.io/<リポジトリ名>/` でそのまま動きます。

初回手順:

```sh
git add -A                 # Cargo.lock と .github/ と web/ を必ず含める
git commit -m "first-order dynamics visualizer"
git branch -M main
git remote add origin https://github.com/<ユーザー名>/<リポジトリ名>.git
git push -u origin main
```

push 後、リポジトリの **Actions** タブで進行を確認できます。完了すると Pages の URL に公開されます
（Settings → Pages にも表示）。`enablement` が組織設定で弾かれる場合だけ、Settings → Pages →
Source を「GitHub Actions」に手動設定してください。

## 数式の記法

- 演算: `+ - * /`、べき乗 `^`、括弧、**暗黙の掛け算** `2x` / `x(1-x)`
- 関数: `sin cos tan asin acos atan sinh cosh tanh exp log(=ln) log10 log2 sqrt cbrt abs sign floor ceil round pow(a,b) atan2 min max mod`
- 定数: `pi e tau`、変数: `x`、パラメータ: `p` `q` `r`（式に現れたものだけ調整可能）

## テスト

```sh
cargo test --workspace
```

- `dyn-core`: パーサ（単項マイナス・べき乗の優先順位・暗黙の掛け算・複数パラメータ）、
  固定点検出、サドルノード分岐、RK4 が解析解 `e^{-t}` に一致、などを検証。

## ギャラリー

| | |
|---|---|
| ロジスティック `x(1-x)` | 双安定 `x - x³` |
| サドルノード `r - x²` (r=1) | `sin(x)` |

`gallery/` 配下に出力例（PNG / GIF）。

## メモ（ツールチェーン）

- stable Rust でビルド可（CI は stable、ローカル開発は nightly でも可）。
- plotters は既定の `ttf`/font-kit が最新 nightly で壊れる（`pathfinder_simd`）ため、
  純 Rust の `ab_glyph` フォントバックエンドを使用し、システム TTF を実行時に登録している
  （`crates/dyn-cli/src/font.rs`）。
- ブラウザ版は `wasm-bindgen` を使用。`wasm-bindgen` **クレートと CLI のバージョンは一致必須**
  （本リポジトリは `0.2.125`）。CLI 不要の `trunk`/`wasm-pack` は使わず、`wasm-bindgen` を直接呼ぶ
  最小構成（`build-web.sh`）。描画と UI を HTML/CSS+canvas にしているのは、日本語表示が
  そのまま出せて軽量なため（重い計算はすべて Rust/WASM）。
