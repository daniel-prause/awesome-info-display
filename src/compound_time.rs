struct CompoundTime {
    w: usize,
    d: usize,
    h: usize,
    m: usize,
    s: usize,
}

macro_rules! reduce {
    ($s: ident, $(($from: ident, $to: ident, $factor: expr)),+) => {{
        $(
            $s.$to += $s.$from / $factor;
            $s.$from %= $factor;
        )+
    }}
}
impl CompoundTime {
    #[inline]
    fn new(w: usize, d: usize, h: usize, m: usize, s: usize) -> Self {
        CompoundTime {
            w: w,
            d: d,
            h: h,
            m: m,
            s: s,
        }
    }
    #[inline]
    fn balance(&mut self) {
        reduce!(self, (s, m, 60), (m, h, 60), (h, d, 24), (d, w, 7));
    }
}
impl fmt::Display for CompoundTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}w {}d {}h {}m {}s",
            self.w, self.d, self.h, self.m, self.s
        )
    }
}
