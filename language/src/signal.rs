use std::{
    cell::RefCell,
    ops::{Add, DerefMut, Mul},
    rc::Rc,
};

#[derive(Debug, Clone, Copy)]
pub struct SignalCtx {
    pub sample_index: u64,
    pub sample_rate: u32,
}

pub trait SignalTrait<T> {
    fn sample(&mut self, ctx: &SignalCtx) -> T;
}

struct BufferedSignalUnshared<T> {
    signal: Box<dyn SignalTrait<T>>,
    buffered_sample: Option<T>,
    next_sample_index: u64,
}

impl<T: Clone> BufferedSignalUnshared<T> {
    pub fn new<S: SignalTrait<T> + 'static>(signal: S) -> Self {
        Self {
            signal: Box::new(signal),
            buffered_sample: None,
            next_sample_index: 0,
        }
    }

    fn sample_signal(&mut self, ctx: &SignalCtx) -> T {
        self.signal.sample(ctx)
    }

    pub fn sample(&mut self, ctx: &SignalCtx) -> T {
        if ctx.sample_index < self.next_sample_index {
            if let Some(buffered_sample) = self.buffered_sample.as_ref() {
                buffered_sample.clone()
            } else {
                let sample = self.sample_signal(ctx);
                self.buffered_sample = Some(sample.clone());
                sample
            }
        } else {
            self.next_sample_index = ctx.sample_index + 1;
            let sample = self.sample_signal(ctx);
            self.buffered_sample = Some(sample.clone());
            sample
        }
    }
}

pub struct BufferedSignal<T>(Rc<RefCell<BufferedSignalUnshared<T>>>);

pub type Sf64 = BufferedSignal<f64>;
pub type Sf32 = BufferedSignal<f32>;
pub type Sbool = BufferedSignal<bool>;
pub type Su8 = BufferedSignal<u8>;

impl<T: Clone + 'static> BufferedSignal<T> {
    pub fn new<S: SignalTrait<T> + 'static>(signal: S) -> Self {
        Self(Rc::new(RefCell::new(BufferedSignalUnshared::new(signal))))
    }

    pub fn sample(&mut self, ctx: &SignalCtx) -> T {
        self.0.borrow_mut().sample(ctx)
    }

    pub fn clone_ref(&self) -> Self {
        Self(Rc::clone(&self.0))
    }

    pub fn map<U: Clone + 'static, F: FnMut(T) -> U + 'static>(&self, f: F) -> BufferedSignal<U> {
        BufferedSignal::new(Map {
            buffered_signal: self.clone_ref(),
            f,
        })
    }

    pub fn both<U: Clone + 'static>(&self, other: &BufferedSignal<U>) -> BufferedSignal<(T, U)> {
        BufferedSignal::new(Both(self.clone_ref(), other.clone_ref()))
    }

    pub fn map_sample_rate<U: Clone + 'static, F: FnMut(T, f64) -> U + 'static>(
        &self,
        f: F,
    ) -> BufferedSignal<U> {
        BufferedSignal::new(MapSampleRate {
            buffered_signal: self.clone_ref(),
            f,
        })
    }

    pub fn force<U: Clone + 'static>(&self, forced_signal: BufferedSignal<U>) -> Self {
        BufferedSignal::new(Force {
            signal: self.clone_ref(),
            forced_signal,
        })
    }

    pub fn debug<F: FnMut(T, &SignalCtx) + 'static>(&self, f: F) -> Self {
        BufferedSignal::new(Debug {
            signal: self.clone_ref(),
            f,
        })
    }

    pub fn debug_<F: FnMut() + 'static>(&self, mut f: F) -> Self {
        self.debug(move |_, _| f())
    }
}

impl BufferedSignal<bool> {
    pub fn trigger(&self) -> Self {
        BufferedSignal::new(Trigger {
            signal: self.clone_ref(),
            prev_sample: false,
        })
    }
}

impl Sf64 {
    pub fn clamp_nyquist(self) -> Self {
        self.map_sample_rate(|x, sample_rate| {
            let nyquist = sample_rate / 2.0;
            x.clamp(-nyquist, nyquist)
        })
    }
    pub fn exp01(&self, k: f64) -> Sf64 {
        BufferedSignal::new(Exp01Signal {
            signal: self.clone_ref(),
            exp01: Exp01::new(k),
        })
    }
    pub fn f32(&self) -> Sf32 {
        self.map(|x| x as f32)
    }
}

impl Sf32 {
    pub fn f64(&self) -> Sf64 {
        self.map(|x| x as f64)
    }
}

impl Su8 {
    pub fn expand(&self) -> [Sbool; 8] {
        [
            self.map(|x| x & (1 << 0) != 0),
            self.map(|x| x & (1 << 1) != 0),
            self.map(|x| x & (1 << 2) != 0),
            self.map(|x| x & (1 << 3) != 0),
            self.map(|x| x & (1 << 4) != 0),
            self.map(|x| x & (1 << 5) != 0),
            self.map(|x| x & (1 << 6) != 0),
            self.map(|x| x & (1 << 7) != 0),
        ]
    }
}

struct Debug<T: Clone + 'static, F: FnMut(T, &SignalCtx)> {
    signal: BufferedSignal<T>,
    f: F,
}
impl<T: Clone + 'static, F: FnMut(T, &SignalCtx)> SignalTrait<T> for Debug<T, F> {
    fn sample(&mut self, ctx: &SignalCtx) -> T {
        let sample = self.signal.sample(ctx);
        (self.f)(sample.clone(), ctx);
        sample
    }
}

#[derive(Clone)]
pub struct Const<T: Clone>(T);

impl<T: Clone + 'static> Const<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn buffered_signal(&self) -> BufferedSignal<T> {
        BufferedSignal::new(self.clone())
    }
}

impl<T: Clone> SignalTrait<T> for Const<T> {
    fn sample(&mut self, _ctx: &SignalCtx) -> T {
        self.0.clone()
    }
}

pub struct Var<T>(Rc<RefCell<T>>);

impl<T: Clone + 'static> Var<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }

    pub fn clone_ref(&self) -> Self {
        Var(Rc::clone(&self.0))
    }

    pub fn get(&self) -> T {
        self.0.borrow().clone()
    }

    pub fn set(&self, value: T) {
        *self.0.borrow_mut().deref_mut() = value;
    }

    pub fn buffered_signal(&self) -> BufferedSignal<T> {
        BufferedSignal::new(self.clone_ref())
    }
}

impl Var<bool> {
    pub fn bool_var(&self) -> BoolVar {
        BoolVar::Var(self.clone_ref())
    }
}

impl<T: Clone + 'static> SignalTrait<T> for Var<T> {
    fn sample(&mut self, _ctx: &SignalCtx) -> T {
        self.get()
    }
}

pub struct TriggerVar {
    var: Var<bool>,
}

impl TriggerVar {
    pub fn new() -> Self {
        Self {
            var: Var::new(false),
        }
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            var: self.var.clone_ref(),
        }
    }

    pub fn set(&self) {
        self.var.set(true);
    }

    pub fn buffered_signal(&self) -> Sbool {
        Sbool::new(self.clone_ref())
    }

    pub fn bool_var(&self) -> BoolVar {
        BoolVar::TriggerVar(self.clone_ref())
    }
}

impl SignalTrait<bool> for TriggerVar {
    fn sample(&mut self, _: &SignalCtx) -> bool {
        let value = self.var.get();
        self.var.set(false);
        value
    }
}

/// Convenience wrapper of `Var<bool>` and `TriggerVar` which supports setting and clearing.
/// Clearing a `TriggerVar` has no effect as it is cleared automatically.
pub enum BoolVar {
    Var(Var<bool>),
    TriggerVar(TriggerVar),
}

impl BoolVar {
    pub fn set(&self) {
        match self {
            Self::Var(var) => var.set(true),
            Self::TriggerVar(trigger_var) => trigger_var.set(),
        }
    }
    pub fn clear(&self) {
        match self {
            Self::Var(var) => var.set(false),
            Self::TriggerVar(_) => (),
        }
    }
}

struct Map<T: Clone, F> {
    buffered_signal: BufferedSignal<T>,
    f: F,
}
impl<T: Clone + 'static, U, F: FnMut(T) -> U> SignalTrait<U> for Map<T, F> {
    fn sample(&mut self, ctx: &SignalCtx) -> U {
        (self.f)(self.buffered_signal.sample(ctx))
    }
}

struct Both<T: Clone, U: Clone>(BufferedSignal<T>, BufferedSignal<U>);
impl<T: Clone + 'static, U: Clone + 'static> SignalTrait<(T, U)> for Both<T, U> {
    fn sample(&mut self, ctx: &SignalCtx) -> (T, U) {
        (self.0.sample(ctx), self.1.sample(ctx))
    }
}

struct MapSampleRate<T: Clone, F> {
    buffered_signal: BufferedSignal<T>,
    f: F,
}
impl<T: Clone + 'static, U, F: FnMut(T, f64) -> U> SignalTrait<U> for MapSampleRate<T, F> {
    fn sample(&mut self, ctx: &SignalCtx) -> U {
        (self.f)(self.buffered_signal.sample(ctx), ctx.sample_rate as f64)
    }
}

struct Trigger {
    signal: BufferedSignal<bool>,
    prev_sample: bool,
}

impl SignalTrait<bool> for Trigger {
    fn sample(&mut self, ctx: &SignalCtx) -> bool {
        let sample = self.signal.sample(ctx);
        let trigger_sample = sample && !self.prev_sample;
        self.prev_sample = sample;
        trigger_sample
    }
}

struct Force<T: Clone, U: Clone> {
    signal: BufferedSignal<T>,
    forced_signal: BufferedSignal<U>,
}
impl<T: Clone + 'static, U: Clone + 'static> SignalTrait<T> for Force<T, U> {
    fn sample(&mut self, ctx: &SignalCtx) -> T {
        self.forced_signal.sample(ctx);
        self.signal.sample(ctx)
    }
}

// The function f(x) = exp(k * (x - a)) - b
// ...where a and b are chosen so that f(0) = 0 and f(1) = 1.
// The k parameter controls how sharp the curve is.
// It approaches a linear function as k approaches 0.
// k = 0 is special cased as a linear function for convenience.
struct Exp01 {
    k: f64,
    a: f64,
    b: f64,
}
impl Exp01 {
    fn new(k: f64) -> Self {
        if k == 0.0 {
            Self {
                k: 0.0,
                a: 0.0,
                b: 0.0,
            }
        } else {
            let b = 1.0 / (k.exp() - 1.0);
            let a = -b.ln() / k;
            Self { k, a, b }
        }
    }

    fn get(&self, x: f64) -> f64 {
        if self.k == 0.0 {
            x
        } else {
            (self.k * (x - self.a)).exp() - self.b
        }
    }
}
struct Exp01Signal {
    signal: Sf64,
    exp01: Exp01,
}
impl SignalTrait<f64> for Exp01Signal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.exp01.get(self.signal.sample(ctx))
    }
}

impl<T> Add for BufferedSignal<T>
where
    T: Add + Clone + 'static,
    <T as Add>::Output: Clone,
{
    type Output = BufferedSignal<<T as Add>::Output>;
    fn add(self, rhs: Self) -> Self::Output {
        self.both(&rhs).map(|(lhs, rhs)| lhs + rhs)
    }
}

impl<T> Add<T> for BufferedSignal<T>
where
    T: Add + Copy + 'static,
    <T as Add>::Output: Clone,
{
    type Output = BufferedSignal<<T as Add>::Output>;
    fn add(self, rhs: T) -> Self::Output {
        self.map(move |lhs| lhs + rhs)
    }
}

impl<T> Mul for BufferedSignal<T>
where
    T: Mul + Clone + 'static,
    <T as Mul>::Output: Clone,
{
    type Output = BufferedSignal<<T as Mul>::Output>;
    fn mul(self, rhs: Self) -> Self::Output {
        self.both(&rhs).map(|(lhs, rhs)| lhs * rhs)
    }
}

impl<T> Mul<T> for BufferedSignal<T>
where
    T: Mul + Copy + 'static,
    <T as Mul>::Output: Clone,
{
    type Output = BufferedSignal<<T as Mul>::Output>;
    fn mul(self, rhs: T) -> Self::Output {
        self.map(move |lhs| lhs * rhs)
    }
}
