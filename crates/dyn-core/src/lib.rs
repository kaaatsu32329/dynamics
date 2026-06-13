//! `dyn-core` — analysis core for the first-order system `dx/dt = f(x)` (no dependencies).
//!
//! - [`Expr`] : parser / fast evaluator for an expression `f(x; p, q, r)`
//! - [`fixed_points`] / [`classify`] : fixed-point detection and stability classification
//! - [`integrate`] / [`rk4_step`] : RK4 time integration
//! - [`Model`] : builds all data needed for plotting in one shot
//!
//! ```
//! use dyn_core::{Expr, Model, Params};
//! let m = Model::build("x - x^3", Params::default()).unwrap();
//! assert_eq!(m.fixed_points.len(), 3);
//! ```

pub mod dynamics;
pub mod model;
pub mod parser;

pub use dynamics::{
    FixedPoint, Stability, classify, find_roots, fixed_points, integrate, rk4_step,
};
pub use model::{Model, Params};
pub use parser::{Expr, N_PARAMS, Op, PARAM_NAMES, ParseError};
