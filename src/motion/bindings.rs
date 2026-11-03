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
        None => {
            out.fill(true);
        }
        Some(RowSelector::Name(name)) => match name {
            RowSelectorName::All => {
                out.fill(true);
            }
            RowSelectorName::Even => {
                for (i, v) in out.iter_mut().enumerate() {
                    *v = i.is_multiple_of(2);
                }
            }
            RowSelectorName::Odd => {
                for (i, v) in out.iter_mut().enumerate() {
                    *v = !i.is_multiple_of(2);
                }
            }
            RowSelectorName::KeyRow => {
                if (key_row as usize) < n {
                    out[key_row as usize] = true;
                }
            }
        },
        Some(RowSelector::Indices(ix)) => {
            for &i in ix {
                if (i as usize) < n {
                    out[i as usize] = true;
                }
            }
        }
    }
    out
}
