//! Easings + cubic bezier. Mirrors `scenic-motion-engine-web/src/motion/phase.ts`.

use crate::types::{Easing, EasingName};
use std::f32::consts::PI;

const C1: f32 = 1.70158;
const C2: f32 = C1 * 1.525;
const C3: f32 = C1 + 1.0;
const C4: f32 = (2.0 * PI) / 3.0;

pub fn apply_easing(e: Option<Easing>, x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    match e {
        None => t,
        Some(Easing::Named(n)) => apply_named(n, t),
        Some(Easing::Bezier { bezier }) => cubic_bezier(t, bezier[0], bezier[1], bezier[2], bezier[3]),
    }
}

pub fn apply_named(name: EasingName, t: f32) -> f32 {
    match name {
        EasingName::Linear => t,
        EasingName::EaseIn => t * t,
        EasingName::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        EasingName::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        EasingName::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
        EasingName::EaseInOutCubic => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }
        EasingName::EaseInQuad => t * t,
        EasingName::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
        EasingName::EaseInOutQuad => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        EasingName::EaseInQuart => t * t * t * t,
        EasingName::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
        EasingName::EaseInOutQuart => {
            if t < 0.5 {
                8.0 * t * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
            }
        }
        EasingName::EaseInExpo => {
            if t == 0.0 { 0.0 } else { (2.0_f32).powf(10.0 * t - 10.0) }
        }
        EasingName::EaseOutExpo => {
            if t == 1.0 { 1.0 } else { 1.0 - (2.0_f32).powf(-10.0 * t) }
        }
        EasingName::EaseInOutExpo => {
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else if t < 0.5 {
                (2.0_f32).powf(20.0 * t - 10.0) / 2.0
            } else {
                (2.0 - (2.0_f32).powf(-20.0 * t + 10.0)) / 2.0
            }
        }
        EasingName::EaseOutBack => 1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2),
        EasingName::EaseInOutBack => {
            if t < 0.5 {
                ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
            } else {
                ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
            }
        }
        EasingName::EaseOutElastic => {
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else {
                (2.0_f32).powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
            }
        }
        EasingName::EaseOutBounce => {
            let n1 = 7.5625_f32;
            let d1 = 2.75_f32;
            if t < 1.0 / d1 {
                n1 * t * t
            } else if t < 2.0 / d1 {
                let u = t - 1.5 / d1;
                n1 * u * u + 0.75
            } else if t < 2.5 / d1 {
                let u = t - 2.25 / d1;
                n1 * u * u + 0.9375
            } else {
                let u = t - 2.625 / d1;
                n1 * u * u + 0.984375
            }
        }
    }
}

pub fn cubic_bezier(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    let mut s = t;
    for _ in 0..10 {
        let u = 1.0 - s;
        let bx = 3.0 * u * u * s * x1 + 3.0 * u * s * s * x2 + s * s * s;
        let dx = bx - t;
        if dx.abs() < 1e-6 { break; }
        let der = 3.0 * u * u * x1 + 6.0 * u * s * (x2 - x1) + 3.0 * s * s * (1.0 - x2);
        if der.abs() < 1e-6 { break; }
        s = (s - dx / der).clamp(0.0, 1.0);
    }
    let u = 1.0 - s;
    3.0 * u * u * s * y1 + 3.0 * u * s * s * y2 + s * s * s
}
