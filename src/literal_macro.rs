#[macro_export]
macro_rules! sinstr_literal {
    ($s:literal) => {{
        const S: &str = $s;
        use $crate::discriminant::NICHE_MAX_INT as NMT;
        const {
            if NMT < S.len() {
                ::core::panic!("string does not fit into inline storage");
            }
        }

        use $crate::SinStr as SS;
        const RS: SS = if S.is_empty() {
            SS::EMPTY
        } else {
            unsafe { SS::new_inline(S) }
        };
        RS
    }};
}

#[macro_export]
macro_rules! ne_sinstr_literal {
    ($s:literal) => {{
        const S: &str = $s;
        use $crate::discriminant::NICHE_MAX_INT as NMT;
        const {
            if NMT < S.len() {
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
    const INLINED_EMPTY: SinStr = sinstr_literal!("");
    #[allow(unused)]
    const INLINED_NE: NonEmptySinStr = ne_sinstr_literal!("y");

    #[test]
    fn empty_string() {
        let s = sinstr_literal!("");
        assert_eq!(s.as_str(), "");
    }

    #[test]
    fn short_string() {
        let s = sinstr_literal!("hi");
        assert_eq!(s.as_str(), "hi");
    }

    #[test]
    fn non_empty() {
        let s = ne_sinstr_literal!("hello");
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn const_context() {
        const S: SinStr = sinstr_literal!("const");
        assert_eq!(S.as_str(), "const");
    }
}
