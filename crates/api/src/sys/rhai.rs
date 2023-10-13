use core::any::Any;

use alloc::string::String;

pub struct RhaiEngine {
    engine: rhai::Engine,
}

impl RhaiEngine {
    pub fn new() -> Self {
        RhaiEngine {
            engine: rhai::Engine::new_raw(),
        }
    }

    pub fn register_type_with_name<T: Any + Clone>(&mut self, name: &str) {
        self.engine.register_type_with_name::<T>(name);
    }

    pub fn register_fn<
        A: 'static,
        const N: usize,
        const C: bool,
        R: Any + Clone,
        const L: bool,
        F: rhai::RegisterNativeFunction<A, N, C, R, L>,
    >(
        &mut self,
        name: &str,
        func: F,
    ) {
        self.engine.register_fn(name, func);
    }

    pub fn init(&mut self) {
        self.register_fn("print", |x: String| log::info!("{:#?}", x));
    }
    
    pub fn eval<T: Any + Clone>(&mut self, code: &str) -> T {
        self.engine.eval_expression::<T>(code).unwrap()
    }

    pub fn eval_with_scope<T: Any + Clone>(&mut self, scope: &mut rhai::Scope, code: &str) -> T {
        self.engine
            .eval_expression_with_scope::<T>(scope, code)
            .unwrap()
    }

    pub fn run(&mut self, code: &str) {
        self.engine.run(code).unwrap()
    }
}
