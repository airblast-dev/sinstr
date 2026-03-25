#[macro_export]
macro_rules! sinstr_literal {
    ($s:literal) => {{
        const S: &str = $s;
        use $crate::discriminant::NICHE_MAX_INT as NMT;
        const {
            if NMT <= S.len() {
                ::core::panic!("string does not fit into inline storage");
            }
        }

        use $crate::SinStr as SS;
        const RS: SS = unsafe { SS::new_inline(S) };
        RS
    }};
}

#[macro_export]
macro_rules! ne_sinstr_literal {
    ($s:literal) => {{
        const S: &str = $s;
        use $crate::discriminant::NICHE_MAX_INT as NMT;
        const {
            if NMT <= S.len() {
                ::core::panic!("string does not fit into inline storage");
            }
            if S.is_empty() {
                ::core::panic!("string is empty");
            }
        }

        use $crate::NonEmptySinStr as NES;
        const RS: NES = unsafe { NES::new_inline(S) };
        RS
    }};
}

#[cfg(test)]
mod tests {
    use crate::{NonEmptySinStr, SinStr};

    #[allow(unused)]
    const INLINED: SinStr = sinstr_literal!("x");
    #[allow(unused)]
    const INLINED_NE: NonEmptySinStr = ne_sinstr_literal!("y");
}
