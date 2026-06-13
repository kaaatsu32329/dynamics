//! Numerical analysis of the first-order system `dx/dt = f(x)`:
//! fixed-point detection, stability classification, and RK4 time integration.

use crate::parser::{Expr, N_PARAMS};

/// Stability of a fixed point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stability {
    /// f'(x*) < 0 : attracts nearby solutions (sink).
    Stable,
    /// f'(x*) > 0 : repels nearby solutions (source).
    Unstable,
    /// f'(x*) ≈ 0 : non-hyperbolic (e.g. semi-stable).
    SemiStable,
}

/// A fixed point and its properties.
#[derive(Clone, Copy, Debug)]
pub struct FixedPoint {
    /// Position x*.
    pub x: f64,
    /// Linearization coefficient f'(x*).
    pub slope: f64,
    /// Stability.
    pub stability: Stability,
}

/// Find fixed points in `[xmin, xmax]`. Samples with `n` subdivisions and detects
/// both sign-change (simple) roots via bisection and tangent (double) roots
/// that touch zero without changing sign.
pub fn find_roots(
    expr: &Expr,
    params: &[f64; N_PARAMS],
    xmin: f64,
    xmax: f64,
    n: usize,
) -> Vec<f64> {
    let n = n.max(2);
    let h = (xmax - xmin) / n as f64;
    let xs: Vec<f64> = (0..=n).map(|i| xmin + i as f64 * h).collect();
    let fs: Vec<f64> = xs.iter().map(|&x| expr.eval(x, params)).collect();
    let mut pts: Vec<f64> = Vec::new();

    // Absolute tolerance for accepting a tangent (double) root, scaled by |f|.
    let scale = fs
        .iter()
        .filter(|v| v.is_finite())
        .fold(0.0_f64, |m, &v| m.max(v.abs()))
        .max(1.0);
    let tangent_tol = scale * 1e-7;

    // 1) Simple roots where the sign changes (refined by bisection).
    for i in 0..n {
        let (fa, fb) = (fs[i], fs[i + 1]);
        if !(fa.is_finite() && fb.is_finite()) {
            continue;
        }
        if fa == 0.0 {
            pts.push(xs[i]);
        } else if fa * fb < 0.0 {
            let (mut a, mut b, mut faa) = (xs[i], xs[i + 1], fa);
            for _ in 0..80 {
                let m = 0.5 * (a + b);
                let fm = expr.eval(m, params);
                if fm == 0.0 || (b - a).abs() < 1e-13 {
                    a = m;
                    b = m;
                    break;
                }
                if faa * fm < 0.0 {
                    b = m;
                } else {
                    a = m;
                    faa = fm;
                }
            }
            pts.push(0.5 * (a + b));
        }
    }
    if fs[n] == 0.0 {
        pts.push(xs[n]);
    }

    // 2) Tangent fixed points: refine an interior local minimum of |f| that does
    //    not change sign via ternary search; if the bottom is ~0 it is a fixed
    //    point (e.g. x=0 of -x^2, the saddle-node turning point at r=0).
    for i in 1..n {
        let (fl, fm, fr) = (fs[i - 1], fs[i], fs[i + 1]);
        if !(fl.is_finite() && fm.is_finite() && fr.is_finite()) {
            continue;
        }
        // Only refine local minima whose bottom could plausibly reach 0 (small
        // relative to scale). Avoids wasteful ternary search on e.g. constant f.
        let is_dip = fm.abs() <= fl.abs() && fm.abs() <= fr.abs();
        if is_dip && fl * fr > 0.0 && fm.abs() <= scale * 1e-2 {
            let (mut lo, mut hi) = (xs[i - 1], xs[i + 1]);
            for _ in 0..80 {
                let m1 = lo + (hi - lo) / 3.0;
                let m2 = hi - (hi - lo) / 3.0;
                if expr.eval(m1, params).abs() < expr.eval(m2, params).abs() {
                    hi = m2;
                } else {
                    lo = m1;
                }
            }
            let xc = 0.5 * (lo + hi);
            if expr.eval(xc, params).abs() <= tangent_tol {
                pts.push(xc);
            }
        }
    }

    // Merge points that are close together.
    pts.sort_by(|u, v| u.partial_cmp(v).unwrap());
    let tol = (xmax - xmin) * 1e-4;
    let mut merged: Vec<f64> = Vec::new();
    for p in pts {
        if merged.last().is_none_or(|&q| (p - q).abs() > tol) {
            merged.push(p);
        }
    }
    merged
}

/// Classify the stability at fixed point `x` using a numerical derivative.
pub fn classify(expr: &Expr, params: &[f64; N_PARAMS], x: f64) -> FixedPoint {
    let eps = 1e-5;
    let slope = (expr.eval(x + eps, params) - expr.eval(x - eps, params)) / (2.0 * eps);
    let stability = if slope < -1e-4 {
        Stability::Stable
    } else if slope > 1e-4 {
        Stability::Unstable
    } else {
        Stability::SemiStable
    };
    FixedPoint {
        x,
        slope,
        stability,
    }
}

/// Detect fixed points in `[xmin, xmax]` and annotate each with its stability.
pub fn fixed_points(
    expr: &Expr,
    params: &[f64; N_PARAMS],
    xmin: f64,
    xmax: f64,
) -> Vec<FixedPoint> {
    find_roots(expr, params, xmin, xmax, 2000)
        .into_iter()
        .map(|x| classify(expr, params, x))
        .collect()
}

/// One step of the 4th-order Runge-Kutta method.
#[inline]
pub fn rk4_step(expr: &Expr, params: &[f64; N_PARAMS], x: f64, dt: f64) -> f64 {
    let k1 = expr.eval(x, params);
    let k2 = expr.eval(x + 0.5 * dt * k1, params);
    let k3 = expr.eval(x + 0.5 * dt * k2, params);
    let k4 = expr.eval(x + dt * k3, params);
    x + dt * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0
}

/// Integrate from initial value `x0` over `[0, tmax]`. Diverging trajectories are
/// clamped to `[lo, hi]`. Returns the `x` sequence at times `i*dt`
/// (length `round(tmax/dt)+1`).
pub fn integrate(
    expr: &Expr,
    params: &[f64; N_PARAMS],
    x0: f64,
    tmax: f64,
    dt: f64,
    lo: f64,
    hi: f64,
) -> Vec<f64> {
    let n = (tmax / dt).round() as usize;
    let mut xs = Vec::with_capacity(n + 1);
    let mut x = x0;
    xs.push(x);
    for _ in 0..n {
        if x > lo && x < hi {
            x = rk4_step(expr, params, x, dt);
            if !x.is_finite() {
                x = if x > 0.0 { hi } else { lo };
            }
            x = x.clamp(lo, hi);
        }
        xs.push(x);
    }
    xs
}
