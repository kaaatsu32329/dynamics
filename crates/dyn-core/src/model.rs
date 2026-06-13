//! Builds all the data needed for plotting (the range of f, fixed points, and
//! many trajectories) from an equation and its parameters.

use crate::dynamics::{FixedPoint, fixed_points, integrate};
use crate::parser::{Expr, N_PARAMS, ParseError};

/// Visualization parameters.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Parameter values (PARAM_NAMES order = p, q, r). Only those used by the
    /// expression matter.
    pub vals: [f64; N_PARAMS],
    /// Lower bound of the displayed state range x.
    pub xmin: f64,
    /// Upper bound of the displayed state range x.
    pub xmax: f64,
    /// Maximum integration time.
    pub tmax: f64,
    /// Number of trajectories (initial conditions) to draw.
    pub n_traj: usize,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            vals: [0.0; N_PARAMS],
            xmin: -2.0,
            xmax: 2.0,
            tmax: 8.0,
            n_traj: 15,
        }
    }
}

/// A model expanded for drawing.
pub struct Model {
    /// The compiled f.
    pub expr: Expr,
    /// The parameters used.
    pub params: Params,
    /// Lower display bound of f(x) (with padding).
    pub fmin: f64,
    /// Upper display bound of f(x) (with padding).
    pub fmax: f64,
    /// Fixed points (with stability).
    pub fixed_points: Vec<FixedPoint>,
    /// Trajectory from each initial condition. `trajectories[k][i]` is x at time `i*dt`.
    pub trajectories: Vec<Vec<f64>>,
    /// Integration time step.
    pub dt: f64,
}

impl Model {
    /// Build a model from an equation string and parameters.
    pub fn build(src: &str, params: Params) -> Result<Model, ParseError> {
        let expr = Expr::compile(src)?;
        let Params {
            vals,
            xmin,
            xmax,
            tmax,
            n_traj,
        } = params;

        // Range of f(x) (used to decide the display bounds).
        let mut fmin = f64::INFINITY;
        let mut fmax = f64::NEG_INFINITY;
        for i in 0..=400 {
            let xv = xmin + (xmax - xmin) * i as f64 / 400.0;
            let v = expr.eval(xv, &vals);
            if v.is_finite() {
                fmin = fmin.min(v);
                fmax = fmax.max(v);
            }
        }
        if !fmin.is_finite() || !fmax.is_finite() || (fmax - fmin) < 1e-9 {
            fmin = -1.0;
            fmax = 1.0;
        }
        let pad = (fmax - fmin) * 0.12;
        fmin -= pad;
        fmax += pad;
        if fmin > 0.0 {
            fmin = -pad;
        }
        if fmax < 0.0 {
            fmax = pad;
        }

        let fixed_points = fixed_points(&expr, &vals, xmin, xmax);

        // Time step and trajectories.
        let steps = ((tmax * 100.0).round() as usize).max(400);
        let dt = tmax / steps as f64;
        let margin = (xmax - xmin) * 0.08;
        let (lo, hi) = (xmin - margin, xmax + margin);
        let n = n_traj.max(1);
        let mut trajectories = Vec::with_capacity(n);
        for k in 0..n {
            let x0 = xmin + (xmax - xmin) * (k as f64 + 0.5) / n as f64;
            trajectories.push(integrate(&expr, &vals, x0, tmax, dt, lo, hi));
        }

        Ok(Model {
            expr,
            params,
            fmin,
            fmax,
            fixed_points,
            trajectories,
            dt,
        })
    }

    /// Evaluate f(x) at the current parameter values.
    #[inline]
    pub fn f(&self, x: f64) -> f64 {
        self.expr.eval(x, &self.params.vals)
    }

    /// Returns the sign of f (the flow direction) at the midpoint of each interval
    /// delimited by the fixed points: a vector of `(interval midpoint x, sign of f +1/-1)`.
    pub fn flow_segments(&self) -> Vec<(f64, f64)> {
        let mut bounds = vec![self.params.xmin];
        bounds.extend(self.fixed_points.iter().map(|p| p.x));
        bounds.push(self.params.xmax);
        let mut out = Vec::new();
        for w in bounds.windows(2) {
            let xm = 0.5 * (w[0] + w[1]);
            let v = self.f(xm);
            if v.is_finite() && v != 0.0 {
                out.push((xm, v.signum()));
            }
        }
        out
    }
}
