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

pub struct BufferedSignal<T: Clone> {
    signal: Rc<RefCell<dyn SignalTrait<T>>>,
    buffered_sample: Option<T>,
    next_sample_index: u64,
}

pub type Sf64 = BufferedSignal<f64>;
pub type Sf32 = BufferedSignal<f32>;
pub type Sbool = BufferedSignal<bool>;

impl<T: Clone + 'static> BufferedSignal<T> {
    pub fn new<S: SignalTrait<T> + 'static>(signal: S) -> Self {
        Self {
            signal: Rc::new(RefCell::new(signal)),
            buffered_sample: None,
            next_sample_index: 0,
        }
    }

    fn sample_signal(&self, ctx: &SignalCtx) -> T {
        self.signal.borrow_mut().sample(ctx)
    }

    pub fn sample(&mut self, ctx: &SignalCtx) -> T {
        if ctx.sample_index < self.next_sample_index {
            self.buffered_sample
                .clone()
                .unwrap_or_else(|| self.sample_signal(ctx))
        } else {
            self.next_sample_index = ctx.sample_index + 1;
            let sample = self.sample_signal(ctx);
            self.buffered_sample = Some(sample.clone());
            sample
        }
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            signal: Rc::clone(&self.signal),
            buffered_sample: self.buffered_sample.clone(),
            next_sample_index: self.next_sample_index,
        }
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
}

pub struct Const<T: Clone>(T);

impl<T: Clone + 'static> Const<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_buffered_signal(self) -> BufferedSignal<T> {
        BufferedSignal::new(self)
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

    pub fn into_buffered_signal(self) -> BufferedSignal<T> {
        BufferedSignal::new(self)
    }
}

impl<T: Clone + 'static> SignalTrait<T> for Var<T> {
    fn sample(&mut self, _ctx: &SignalCtx) -> T {
        self.get()
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

impl<T> Add for &BufferedSignal<T>
where
    T: Add + Clone + 'static,
    <T as Add>::Output: Clone,
{
    type Output = BufferedSignal<<T as Add>::Output>;
    fn add(self, rhs: Self) -> Self::Output {
        self.clone_ref() + rhs.clone_ref()
    }
}

impl<T> Add<T> for &BufferedSignal<T>
where
    T: Add + Copy + 'static,
    <T as Add>::Output: Clone,
{
    type Output = BufferedSignal<<T as Add>::Output>;
    fn add(self, rhs: T) -> Self::Output {
        self.clone_ref() + rhs
    }
}

impl<T> Mul for &BufferedSignal<T>
where
    T: Mul + Clone + 'static,
    <T as Mul>::Output: Clone,
{
    type Output = BufferedSignal<<T as Mul>::Output>;
    fn mul(self, rhs: Self) -> Self::Output {
        self.clone_ref() * rhs.clone_ref()
    }
}

impl<T> Mul<T> for &BufferedSignal<T>
where
    T: Mul + Copy + 'static,
    <T as Mul>::Output: Clone,
{
    type Output = BufferedSignal<<T as Mul>::Output>;
    fn mul(self, rhs: T) -> Self::Output {
        self.clone_ref() * rhs
    }
}
