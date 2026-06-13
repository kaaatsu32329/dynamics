//! Bridge to use `dyn-core` from the browser (WebAssembly).
//!
//! All the heavy work (expression parsing, fixed-point detection, RK4 integration)
//! happens in Rust; results are returned to JS as plain shapes such as `Float64Array`.
//! Drawing is handled by JS (canvas).

use dyn_core::{Model, Params, Stability};
use wasm_bindgen::prelude::*;

/// Dynamical-system model exposed to the browser.
#[wasm_bindgen]
pub struct WasmModel {
    m: Model,
}

#[wasm_bindgen]
impl WasmModel {
    /// Build a model from an equation and parameters (p, q, r). Throws a string on failure.
    #[allow(clippy::too_many_arguments)] // a flat argument list is simplest for a wasm-bindgen export
    pub fn build(
        eq: &str,
        p: f64,
        q: f64,
        r: f64,
        xmin: f64,
        xmax: f64,
        tmax: f64,
        n_traj: u32,
    ) -> Result<WasmModel, String> {
        let params = Params {
            vals: [p, q, r],
            xmin,
            xmax,
            // Cap the upper bound to avoid runaway memory (huge tmax -> huge trajectory arrays).
            tmax: tmax.clamp(0.5, 1000.0),
            n_traj: n_traj.clamp(2, 60) as usize,
        };
        Model::build(eq, params)
            .map(|m| WasmModel { m })
            .map_err(|e| e.to_string())
    }

    /// Whether the expression uses parameter p.
    #[wasm_bindgen(getter)]
    pub fn uses_p(&self) -> bool {
        self.m.expr.uses_param(0)
    }

    /// Whether the expression uses parameter q.
    #[wasm_bindgen(getter)]
    pub fn uses_q(&self) -> bool {
        self.m.expr.uses_param(1)
    }

    /// Whether the expression uses parameter r.
    #[wasm_bindgen(getter)]
    pub fn uses_r(&self) -> bool {
        self.m.expr.uses_param(2)
    }

    /// Lower display bound of f(x).
    #[wasm_bindgen(getter)]
    pub fn fmin(&self) -> f64 {
        self.m.fmin
    }

    /// Upper display bound of f(x).
    #[wasm_bindgen(getter)]
    pub fn fmax(&self) -> f64 {
        self.m.fmax
    }

    /// Integration time step.
    #[wasm_bindgen(getter)]
    pub fn dt(&self) -> f64 {
        self.m.dt
    }

    /// Number of trajectories (initial conditions).
    #[wasm_bindgen(getter)]
    pub fn n_traj(&self) -> usize {
        self.m.trajectories.len()
    }

    /// Number of samples per trajectory.
    #[wasm_bindgen(getter)]
    pub fn traj_len(&self) -> usize {
        self.m.trajectories.first().map_or(0, |t| t.len())
    }

    /// f(x) sampled at n+1 points over [xmin, xmax].
    pub fn f_samples(&self, n: usize) -> Vec<f64> {
        let p = &self.m.params;
        let n = n.max(1);
        (0..=n)
            .map(|i| {
                let x = p.xmin + (p.xmax - p.xmin) * i as f64 / n as f64;
                self.m.f(x)
            })
            .collect()
    }

    /// Fixed points flattened as `[x, slope, kind, ...]`.
    /// kind: 0=stable, 1=unstable, 2=semi-stable.
    pub fn fixed_points(&self) -> Vec<f64> {
        let mut v = Vec::with_capacity(self.m.fixed_points.len() * 3);
        for fp in &self.m.fixed_points {
            v.push(fp.x);
            v.push(fp.slope);
            v.push(match fp.stability {
                Stability::Stable => 0.0,
                Stability::Unstable => 1.0,
                Stability::SemiStable => 2.0,
            });
        }
        v
    }

    /// Flow intervals flattened as `[midpoint x, sign(±1), ...]`.
    pub fn flow_segments(&self) -> Vec<f64> {
        let mut v = Vec::new();
        for (xm, sign) in self.m.flow_segments() {
            v.push(xm);
            v.push(sign);
        }
        v
    }

    /// All trajectories flattened in row-major order (length = n_traj * traj_len).
    /// `trajectories[k][i]` is `flat[k * traj_len + i]`.
    pub fn trajectories(&self) -> Vec<f64> {
        let len = self.traj_len();
        let mut v = Vec::with_capacity(self.m.trajectories.len() * len);
        for tr in &self.m.trajectories {
            v.extend_from_slice(tr);
        }
        v
    }

    /// f(x) at a single point (used to color particles).
    pub fn f_at(&self, x: f64) -> f64 {
        self.m.f(x)
    }

    /// Bifurcation-diagram data. Sweeps parameter `index` over [vmin, vmax] in n steps and
    /// returns the fixed points at each value, flattened as `[v, x*, kind, ...]`
    /// (kind: 0=stable, 1=unstable, 2=semi-stable). Non-swept parameters stay at their
    /// current values.
    ///
    /// Since it is called repeatedly over many sweep points, the root search uses a
    /// resolution coarse enough for the diagram (`ROOT_SAMPLES`, lighter than the
    /// default 2000 in `fixed_points`).
    pub fn bifurcation(&self, index: usize, vmin: f64, vmax: f64, n: usize) -> Vec<f64> {
        const ROOT_SAMPLES: usize = 600;
        let base = &self.m.params;
        let steps = n.max(2);
        let mut out = Vec::new();
        for i in 0..=steps {
            let v = vmin + (vmax - vmin) * i as f64 / steps as f64;
            let mut vals = base.vals;
            if let Some(slot) = vals.get_mut(index) {
                *slot = v;
            }
            for x in dyn_core::find_roots(&self.m.expr, &vals, base.xmin, base.xmax, ROOT_SAMPLES) {
                let fp = dyn_core::classify(&self.m.expr, &vals, x);
                out.push(v);
                out.push(fp.x);
                out.push(match fp.stability {
                    Stability::Stable => 0.0,
                    Stability::Unstable => 1.0,
                    Stability::SemiStable => 2.0,
                });
            }
        }
        out
    }
}
