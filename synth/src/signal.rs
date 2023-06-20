use std::{cell::RefCell, rc::Rc};

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

struct Map<T: Clone, F> {
    buffered_signal: BufferedSignal<T>,
    f: F,
}

impl<T: Clone + 'static, U, F: FnMut(T) -> U> SignalTrait<U> for Map<T, F> {
    fn sample(&mut self, ctx: &SignalCtx) -> U {
        (self.f)(self.buffered_signal.sample(ctx))
    }
}
