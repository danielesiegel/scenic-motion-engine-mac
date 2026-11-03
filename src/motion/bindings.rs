//! Row-selector resolution. Mirrors `scenic-motion-engine-web/src/motion/bindings.ts`.

use crate::types::{Binding, BindingTarget, RowSelector, RowSelectorName};

pub fn select_rows(binding: &Binding, row_count: u32, key_row: u32) -> Vec<bool> {
    let n = row_count as usize;
    if (binding.target) == BindingTarget::Camera {
        return vec![false; n];
    }
    let sel = binding.rows.as_ref();
    let mut out = vec![false; n];
    match sel {
        None => { for v in &mut out { *v = true; } }
        Some(RowSelector::Name(name)) => match name {
            RowSelectorName::All => { for v in &mut out { *v = true; } }
            RowSelectorName::Even => for i in 0..n { out[i] = i % 2 == 0 },
            RowSelectorName::Odd  => for i in 0..n { out[i] = i % 2 == 1 },
            RowSelectorName::KeyRow => {
                if (key_row as usize) < n { out[key_row as usize] = true; }
            }
        }
        Some(RowSelector::Indices(ix)) => {
            for &i in ix {
                if (i as usize) < n { out[i as usize] = true; }
            }
        }
    }
    out
}
