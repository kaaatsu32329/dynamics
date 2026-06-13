//! Draw a `dyn-core::Model` as two panels (left: f(x)+phase line / right: time series)
//! using plotters.

use dyn_core::{Model, Stability};
use plotters::coord::Shift;
use plotters::prelude::*;

const RED: RGBColor = RGBColor(0xd1, 0x49, 0x5b); // increasing direction (f>0)
const BLUE: RGBColor = RGBColor(0x2a, 0x6f, 0x97); // decreasing direction (f<0)
const STABLE: RGBColor = RGBColor(0x1b, 0x43, 0x32);
const UNSTABLE: RGBColor = RGBColor(0x9d, 0x02, 0x08);
const SEMI: RGBColor = RGBColor(0x8a, 0x6d, 0x00);
const CURVE: RGBColor = RGBColor(0x34, 0x3a, 0x40);
const AXIS: RGBColor = RGBColor(0x9a, 0xa3, 0xad);

type Res = Result<(), Box<dyn std::error::Error>>;

/// Approximate viridis colormap.
fn viridis(t: f64) -> RGBColor {
    let stops = [
        (68, 1, 84),
        (59, 82, 139),
        (33, 144, 140),
        (93, 201, 99),
        (253, 231, 37),
    ];
    let s = t.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let i = s.floor() as usize;
    let f = s - i as f64;
    let a = stops[i];
    let b = stops[(i + 1).min(stops.len() - 1)];
    let lerp = |x: i32, y: i32| (x as f64 + (y - x) as f64 * f).round() as u8;
    RGBColor(lerp(a.0, b.0), lerp(a.1, b.1), lerp(a.2, b.2))
}

fn finite(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else if v > 0.0 {
        hi
    } else {
        lo
    }
}

/// Left panel: graph of f(x) + phase line (flow direction) + fixed points.
/// If `particles` is given, draw particles at those positions on the phase line (for animation).
fn draw_phase<DB>(area: &DrawingArea<DB, Shift>, m: &Model, particles: Option<&[f64]>) -> Res
where
    DB: DrawingBackend,
    DB::ErrorType: std::error::Error + 'static,
{
    let p = &m.params;
    let mut chart = ChartBuilder::on(area)
        .margin(8)
        .caption("phase line:  f(x) = dx/dt", ("sans-serif", 15))
        .x_label_area_size(32)
        .y_label_area_size(48)
        .build_cartesian_2d(p.xmin..p.xmax, m.fmin..m.fmax)?;
    chart
        .configure_mesh()
        .light_line_style(RGBColor(0xee, 0xf1, 0xf4))
        .x_desc("x")
        .y_desc("f(x) = dx/dt")
        .draw()?;

    // samples of f
    let ns = 500usize;
    let xs: Vec<f64> = (0..=ns)
        .map(|i| p.xmin + (p.xmax - p.xmin) * i as f64 / ns as f64)
        .collect();

    // fill f>0 in red, f<0 in blue
    chart.draw_series(AreaSeries::new(
        xs.iter()
            .map(|&x| (x, finite(m.f(x), m.fmin, m.fmax).max(0.0))),
        0.0,
        RED.mix(0.15),
    ))?;
    chart.draw_series(AreaSeries::new(
        xs.iter()
            .map(|&x| (x, finite(m.f(x), m.fmin, m.fmax).min(0.0))),
        0.0,
        BLUE.mix(0.15),
    ))?;

    // phase line (the f = 0 axis)
    chart.draw_series(LineSeries::new(
        vec![(p.xmin, 0.0), (p.xmax, 0.0)],
        AXIS.stroke_width(2),
    ))?;

    // f(x) curve
    chart.draw_series(LineSeries::new(
        xs.iter().map(|&x| (x, finite(m.f(x), m.fmin, m.fmax))),
        CURVE.stroke_width(3),
    ))?;

    // flow arrows (at the midpoint of each interval, on f=0)
    let half = (p.xmax - p.xmin) * 0.04;
    let hw = (p.xmax - p.xmin) * 0.015;
    let hh = (m.fmax - m.fmin) * 0.028;
    for (xm, sign) in m.flow_segments() {
        let col = if sign > 0.0 { RED } else { BLUE };
        let (x1, x2) = if sign > 0.0 {
            (xm - half, xm + half)
        } else {
            (xm + half, xm - half)
        };
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(x1, 0.0), (x2, 0.0)],
            col.stroke_width(2),
        )))?;
        let back = if sign > 0.0 { x2 - hw } else { x2 + hw };
        chart.draw_series(std::iter::once(Polygon::new(
            vec![(x2, 0.0), (back, hh), (back, -hh)],
            col.filled(),
        )))?;
    }

    // fixed points (filled = stable / hollow = unstable / semi-stable)
    for fp in &m.fixed_points {
        let c = (fp.x, 0.0);
        match fp.stability {
            Stability::Stable => {
                chart.draw_series(std::iter::once(Circle::new(c, 6, STABLE.filled())))?;
            }
            Stability::Unstable => {
                chart.draw_series(std::iter::once(Circle::new(c, 6, WHITE.filled())))?;
                chart.draw_series(std::iter::once(Circle::new(c, 6, UNSTABLE.stroke_width(2))))?;
            }
            Stability::SemiStable => {
                chart.draw_series(std::iter::once(Circle::new(
                    c,
                    6,
                    RGBColor(0xff, 0xd2, 0x4d).filled(),
                )))?;
                chart.draw_series(std::iter::once(Circle::new(c, 6, SEMI.stroke_width(2))))?;
            }
        };
    }

    // particles (animation)
    if let Some(ps) = particles {
        for &xv in ps {
            if xv < p.xmin || xv > p.xmax {
                continue;
            }
            let col = if m.f(xv) >= 0.0 { RED } else { BLUE };
            chart.draw_series(std::iter::once(Circle::new((xv, 0.0), 5, col.filled())))?;
        }
    }
    Ok(())
}

/// Right panel: time series x(t). If `t_now` is given, draw up to it and mark the tip.
fn draw_time<DB>(area: &DrawingArea<DB, Shift>, m: &Model, t_now: Option<f64>) -> Res
where
    DB: DrawingBackend,
    DB::ErrorType: std::error::Error + 'static,
{
    let p = &m.params;
    let mut chart = ChartBuilder::on(area)
        .margin(8)
        .caption("time series  x(t)", ("sans-serif", 15))
        .x_label_area_size(32)
        .y_label_area_size(48)
        .build_cartesian_2d(0f64..p.tmax, p.xmin..p.xmax)?;
    chart
        .configure_mesh()
        .light_line_style(RGBColor(0xee, 0xf1, 0xf4))
        .x_desc("t")
        .y_desc("x(t)")
        .draw()?;

    // horizontal lines at the fixed points
    for fp in &m.fixed_points {
        let col = match fp.stability {
            Stability::Stable => STABLE,
            Stability::Unstable => UNSTABLE,
            Stability::SemiStable => SEMI,
        };
        chart.draw_series(LineSeries::new(
            vec![(0.0, fp.x), (p.tmax, fp.x)],
            col.mix(0.5).stroke_width(1),
        ))?;
    }

    // trajectories
    let n = m.trajectories.len();
    let imax = match t_now {
        Some(t) => ((t / m.dt).round() as usize).min(m.trajectories[0].len() - 1),
        None => m.trajectories[0].len() - 1,
    };
    for (k, tr) in m.trajectories.iter().enumerate() {
        let col = viridis(k as f64 / (n.max(2) - 1) as f64);
        chart.draw_series(LineSeries::new(
            (0..=imax).map(|i| (i as f64 * m.dt, tr[i])),
            col.stroke_width(2),
        ))?;
        if t_now.is_some() {
            chart.draw_series(std::iter::once(Circle::new(
                (imax as f64 * m.dt, tr[imax]),
                3,
                col.filled(),
            )))?;
        }
    }
    Ok(())
}

/// Render a single still image (PNG).
pub fn render_png(m: &Model, path: &str, size: (u32, u32)) -> Res {
    let root = BitMapBackend::new(path, size).into_drawing_area();
    root.fill(&WHITE)?;
    let root = root.titled(
        &format!("first-order system   dx/dt = {}", m.expr.source()),
        ("sans-serif", 20),
    )?;
    let (left, right) = root.split_horizontally(size.0 / 2);
    draw_phase(&left, m, None)?;
    draw_time(&right, m, None)?;
    root.present()?;
    Ok(())
}

/// Render an animated GIF.
pub fn render_gif(m: &Model, path: &str, size: (u32, u32), frames: usize) -> Res {
    let frame_ms = 50u32;
    let root = BitMapBackend::gif(path, size, frame_ms)?.into_drawing_area();
    for fr in 0..frames {
        let t = m.params.tmax * fr as f64 / (frames - 1).max(1) as f64;
        let idx = ((t / m.dt).round() as usize).min(m.trajectories[0].len() - 1);
        let particles: Vec<f64> = m.trajectories.iter().map(|tr| tr[idx]).collect();
        root.fill(&WHITE)?;
        let titled = root.titled(
            &format!("dx/dt = {}    t = {:.2}", m.expr.source(), t),
            ("sans-serif", 18),
        )?;
        let (left, right) = titled.split_horizontally(size.0 / 2);
        draw_phase(&left, m, Some(&particles))?;
        draw_time(&right, m, Some(t))?;
        root.present()?;
    }
    Ok(())
}
