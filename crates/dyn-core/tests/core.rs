//! Integration tests for dyn-core (parser / fixed points / stability / integration).

use dyn_core::{Expr, Model, Params, Stability, classify, find_roots, fixed_points, integrate};

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

/// Array of parameters p, q, r (PARAM_NAMES order).
fn pqr(p: f64, q: f64, r: f64) -> [f64; 3] {
    [p, q, r]
}

/// Evaluate with only r given (p=q=0).
fn ev(src: &str, x: f64, r: f64) -> f64 {
    Expr::compile(src)
        .expect("compile")
        .eval(x, &pqr(0.0, 0.0, r))
}

#[test]
fn parser_basic_arithmetic() {
    assert!(approx(ev("x*(1-x)", 0.5, 0.0), 0.25, 1e-12));
    assert!(approx(ev("x - x^3", 2.0, 0.0), 2.0 - 8.0, 1e-12));
    assert!(approx(ev("abs(-3) + 1", 0.0, 0.0), 4.0, 1e-12));
    assert!(approx(ev("pow(2,10)", 0.0, 0.0), 1024.0, 1e-9));
}

#[test]
fn parser_unary_and_pow_precedence() {
    // -x^2 = -(x^2)
    assert!(approx(ev("-x^2", 3.0, 0.0), -9.0, 1e-12));
    // 2^-2 = 2^(-2) = 0.25
    assert!(approx(ev("2^-2", 0.0, 0.0), 0.25, 1e-12));
    // -(-x) = x
    assert!(approx(ev("-(-x)", 4.0, 0.0), 4.0, 1e-12));
    // 2*-3 = -6
    assert!(approx(ev("2*-3", 0.0, 0.0), -6.0, 1e-12));
}

#[test]
fn parser_implicit_multiplication() {
    assert!(approx(ev("2x + 1", 3.0, 0.0), 7.0, 1e-12));
    assert!(approx(
        ev("x(1-x)(x-0.3)", 0.5, 0.0),
        0.5 * 0.5 * 0.2,
        1e-12
    ));
    assert!(approx(
        ev("3pi", 0.0, 0.0),
        3.0 * std::f64::consts::PI,
        1e-12
    ));
}

#[test]
fn parser_functions_and_constants() {
    assert!(approx(
        ev("sin(x)", std::f64::consts::FRAC_PI_2, 0.0),
        1.0,
        1e-12
    ));
    assert!(approx(ev("exp(0)", 5.0, 0.0), 1.0, 1e-12));
    assert!(approx(ev("sqrt(x)", 9.0, 0.0), 3.0, 1e-12));
    assert!(approx(ev("log(e)", 0.0, 0.0), 1.0, 1e-12));
    assert!(approx(ev("pi", 0.0, 0.0), std::f64::consts::PI, 1e-12));
}

#[test]
fn parser_used_params() {
    // PARAM_NAMES = [p, q, r] -> index 0,1,2
    let none = Expr::compile("x*(1-x)").unwrap();
    assert_eq!(none.used_params(), [false, false, false]);
    let r = Expr::compile("r - x^2").unwrap();
    assert_eq!(r.used_params(), [false, false, true]);
    let pqr = Expr::compile("p*x + q*sin(x) - r").unwrap();
    assert_eq!(pqr.used_params(), [true, true, true]);
    assert!(approx(ev("r - x^2", 1.0, 0.5), 0.5 - 1.0, 1e-12));
}

#[test]
fn parser_three_params() {
    // f = p + q*x + r*x^2,  p=1,q=2,r=3, x=2 -> 1 + 4 + 12 = 17
    let e = Expr::compile("p + q*x + r*x^2").unwrap();
    assert!(approx(e.eval(2.0, &pqr(1.0, 2.0, 3.0)), 17.0, 1e-12));
    // 'p' must not be confused with 'pi' / 'q'
    assert!(approx(
        Expr::compile("pi").unwrap().eval(0.0, &pqr(9.0, 9.0, 9.0)),
        std::f64::consts::PI,
        1e-12
    ));
}

#[test]
fn parser_errors() {
    assert!(Expr::compile("(x").is_err()); // mismatched parentheses
    assert!(Expr::compile("foo(x)").is_err()); // unknown function -> treated as Name -> unknown variable
    assert!(Expr::compile("").is_err()); // empty
    assert!(Expr::compile("y + 1").is_err()); // unknown variable y
}

#[test]
fn tangent_fixed_point() {
    // -x^2 : f touches 0 at x=0 (no sign change) -> detect one semi-stable fixed point
    let e = Expr::compile("-x^2").unwrap();
    let fps = fixed_points(&e, &pqr(0.0, 0.0, 0.0), -2.0, 2.0);
    assert_eq!(
        fps.len(),
        1,
        "should detect exactly one tangent fixed point"
    );
    assert!(approx(fps[0].x, 0.0, 1e-4));
    assert_eq!(fps[0].stability, Stability::SemiStable);

    // a local minimum whose bottom is not 0 (does not touch) is not a fixed point
    let g = Expr::compile("-x^2 - 1").unwrap();
    assert!(fixed_points(&g, &pqr(0.0, 0.0, 0.0), -2.0, 2.0).is_empty());
}

#[test]
fn fixed_points_bistable() {
    // x - x^3 : -1 (stable), 0 (unstable), 1 (stable)
    let e = Expr::compile("x - x^3").unwrap();
    let fps = fixed_points(&e, &pqr(0.0, 0.0, 0.0), -1.8, 1.8);
    assert_eq!(fps.len(), 3);
    assert!(approx(fps[0].x, -1.0, 1e-6));
    assert!(approx(fps[1].x, 0.0, 1e-6));
    assert!(approx(fps[2].x, 1.0, 1e-6));
    assert_eq!(fps[0].stability, Stability::Stable);
    assert_eq!(fps[1].stability, Stability::Unstable);
    assert_eq!(fps[2].stability, Stability::Stable);
}

#[test]
fn fixed_points_logistic() {
    // x*(1-x) : 0 (unstable), 1 (stable)
    let e = Expr::compile("x*(1-x)").unwrap();
    let fps = fixed_points(&e, &pqr(0.0, 0.0, 0.0), -0.5, 1.6);
    assert_eq!(fps.len(), 2);
    assert_eq!(fps[0].stability, Stability::Unstable);
    assert_eq!(fps[1].stability, Stability::Stable);
}

#[test]
fn saddle_node_bifurcation() {
    // r - x^2 : 2 points for r>0, none for r<0 (saddle-node bifurcation)
    let e = Expr::compile("r - x^2").unwrap();
    assert_eq!(
        find_roots(&e, &pqr(0.0, 0.0, 1.0), -2.0, 2.0, 2000).len(),
        2
    );
    assert_eq!(
        find_roots(&e, &pqr(0.0, 0.0, -1.0), -2.0, 2.0, 2000).len(),
        0
    );
    let fps = fixed_points(&e, &pqr(0.0, 0.0, 1.0), -2.0, 2.0);
    assert_eq!(fps[0].stability, Stability::Unstable); // x* = -1
    assert_eq!(fps[1].stability, Stability::Stable); //  x* = +1
}

#[test]
fn classify_slope_sign() {
    let e = Expr::compile("-x").unwrap();
    let fp = classify(&e, &pqr(0.0, 0.0, 0.0), 0.0);
    assert!(approx(fp.slope, -1.0, 1e-3));
    assert_eq!(fp.stability, Stability::Stable);
}

#[test]
fn integrate_matches_analytic_decay() {
    // dx/dt = -x, x0 = 1 -> x(t) = e^{-t}
    let e = Expr::compile("-x").unwrap();
    let dt = 0.01;
    let xs = integrate(&e, &pqr(0.0, 0.0, 0.0), 1.0, 5.0, dt, -10.0, 10.0);
    let idx = (2.0 / dt).round() as usize;
    assert!(approx(xs[idx], (-2.0f64).exp(), 1e-4));
}

#[test]
fn integrate_clamps_divergence() {
    // dx/dt = x, x0 = 1 -> diverges but is clamped at hi=10, no NaN/Inf
    let e = Expr::compile("x").unwrap();
    let xs = integrate(&e, &pqr(0.0, 0.0, 0.0), 1.0, 10.0, 0.01, -10.0, 10.0);
    assert!(xs.iter().all(|v| v.is_finite()));
    assert!(approx(*xs.last().unwrap(), 10.0, 1e-9));
}

#[test]
fn model_build_logistic() {
    let m = Model::build("x*(1 - x)", Params::default()).unwrap();
    assert_eq!(m.fixed_points.len(), 2);
    assert_eq!(m.trajectories.len(), Params::default().n_traj);
    // all trajectories have the same length
    let len0 = m.trajectories[0].len();
    assert!(m.trajectories.iter().all(|t| t.len() == len0));
    // flow intervals: split by 0 and 1 -> signs for 3 intervals
    assert!(!m.flow_segments().is_empty());
}
