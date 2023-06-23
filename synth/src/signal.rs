use std::{cell::RefCell, ops::DerefMut, rc::Rc};

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
}

impl BufferedSignal<bool> {
    pub fn trigger(&self) -> Self {
        BufferedSignal::new(Trigger {
            signal: self.clone_ref(),
            prev_sample: false,
        })
    }
}

impl BufferedSignal<f64> {
    pub fn add(&self, other: &Self) -> Self {
        BufferedSignal::new(Add(self.clone_ref(), other.clone_ref()))
    }
    pub fn mul(&self, other: &Self) -> Self {
        BufferedSignal::new(Mul(self.clone_ref(), other.clone_ref()))
    }
    pub fn scale(&self, by: f64) -> Self {
        BufferedSignal::new(Scale {
            signal: self.clone_ref(),
            by,
        })
    }
}

pub struct Const<T>(T);

impl<T> Const<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T: Clone> SignalTrait<T> for Const<T> {
    fn sample(&mut self, _ctx: &SignalCtx) -> T {
        self.0.clone()
    }
}

impl<T: Clone + 'static> From<Const<T>> for BufferedSignal<T> {
    fn from(value: Const<T>) -> Self {
        BufferedSignal::new(value)
    }
}

pub struct Var<T>(Rc<RefCell<T>>);

impl<T: Clone> Var<T> {
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
}

impl<T: Clone> SignalTrait<T> for Var<T> {
    fn sample(&mut self, _ctx: &SignalCtx) -> T {
        self.get()
    }
}

impl<T: Clone + 'static> From<Var<T>> for BufferedSignal<T> {
    fn from(value: Var<T>) -> Self {
        BufferedSignal::new(value)
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

struct Add(BufferedSignal<f64>, BufferedSignal<f64>);
impl SignalTrait<f64> for Add {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.0.sample(ctx) + self.1.sample(ctx)
    }
}

struct Mul(BufferedSignal<f64>, BufferedSignal<f64>);
impl SignalTrait<f64> for Mul {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.0.sample(ctx) * self.1.sample(ctx)
    }
}

struct Scale {
    signal: BufferedSignal<f64>,
    by: f64,
}
impl SignalTrait<f64> for Scale {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.signal.sample(ctx) * self.by
    }
}
