//! Local ELK execution. Only trusted installed code runs; graph data is a JSON
//! argument. There are no filesystem, network, process, module or timer hosts.
use rquickjs::{Context, Function, Runtime};
use std::{
    cell::RefCell,
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) const ENGINE_ID: &str =
    "elkjs/0.11.1;quickjs-ng/rquickjs-0.11.0;layered-polyline/1;metrics/1";
const SOURCE: &str = include_str!("../vendor/elkjs/elk.bundled.js");
const HEAP_LIMIT: usize = 64 * 1024 * 1024;
const OUTPUT_LIMIT: usize = 262_144;
const DEADLINE: Duration = Duration::from_secs(3);
const HOST: &str = r#"var global=globalThis;
var setTimeout=function(callback){Promise.resolve().then(callback);};
var console={log:function(){},warn:function(){},error:function(){},err:function(){}};
"#;
const ADAPTER: &str = r#"
var localLayoutResult=null;
var localLayoutFailed=false;
function localLayout(input){
  localLayoutResult=null; localLayoutFailed=false;
  new ELK({algorithms:['layered']}).layout(JSON.parse(input)).then(
    function(graph){localLayoutResult=JSON.stringify(graph);},
    function(){localLayoutFailed=true;});
}
function takeLocalLayout(){
  var result=localLayoutResult;
  localLayoutResult=null;
  if(localLayoutFailed || result===null) throw new Error('layout unavailable');
  return result;
}
"#;

struct Engine {
    context: Context,
    runtime: Runtime,
    // The deadline is replaced before every entry, including initialization.
    deadline: Arc<std::sync::Mutex<Instant>>,
}
impl Engine {
    fn new() -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|_| "local layout runtime unavailable")?;
        runtime.set_memory_limit(HEAP_LIMIT);
        runtime.set_max_stack_size(1024 * 1024);
        let deadline = Arc::new(std::sync::Mutex::new(Instant::now() + DEADLINE));
        let bound = deadline.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || {
            bound.lock().map_or(true, |end| Instant::now() >= *end)
        })));
        let context = Context::full(&runtime).map_err(|_| "local layout context unavailable")?;
        context
            .with(|ctx| -> rquickjs::Result<()> {
                ctx.eval::<(), _>(HOST)?;
                ctx.eval::<(), _>(SOURCE)?;
                ctx.eval::<(), _>(ADAPTER)
            })
            .map_err(|_| "local layout initialization failed or exceeded its bound")?;
        Ok(Self {
            context,
            runtime,
            deadline,
        })
    }

    fn layout(&self, input: &str) -> Result<String, String> {
        *self
            .deadline
            .lock()
            .map_err(|_| "local layout deadline unavailable")? = Instant::now() + DEADLINE;
        self.context
            .with(|ctx| {
                let run: Function = ctx.globals().get("localLayout")?;
                run.call::<_, ()>((input,))
            })
            .map_err(|_| "local layout rejected input or exceeded its bound")?;
        let mut jobs = 0;
        while self.runtime.is_job_pending() {
            jobs += 1;
            if jobs > 1024
                || self
                    .deadline
                    .lock()
                    .map_or(true, |end| Instant::now() >= *end)
            {
                return Err("local layout time/job budget exceeded".into());
            }
            self.execute_job()?;
        }
        let output: String = self
            .context
            .with(|ctx| {
                let take: Function = ctx.globals().get("takeLocalLayout")?;
                take.call(())
            })
            .map_err(|_| "local layout returned no complete geometry")?;
        self.runtime.run_gc();
        if output.len() > OUTPUT_LIMIT {
            return Err("local layout geometry budget exceeded".into());
        }
        Ok(output)
    }

    fn execute_job(&self) -> Result<bool, &'static str> {
        self.runtime.execute_pending_job().map_err(|error| {
            // rquickjs 0.11.0 wraps JS_ExecutePendingJob's borrowed
            // context in an owning JobException without duplicating its
            // C reference. Ctx::clone owns only a C reference (no Rust
            // runtime handle): transfer one to that error's destructor.
            // Forgetting the JobException itself would leak the runtime.
            error.0.with(|ctx| std::mem::forget(ctx.clone()));
            "local layout execution failed or exceeded its bound"
        })
    }
}

// Reuse trusted engine code, never geometry or authority. Each request supplies
// its whole current graph; result objects are released before returning. A failed
// execution discards the entire runtime so partial jobs cannot affect a later read.
thread_local! { static LOCAL: RefCell<Option<Engine>> = const { RefCell::new(None) }; }
pub(super) fn layout(input: &str) -> Result<String, String> {
    if input.len() > 65_536 {
        return Err("local layout input budget exceeded".into());
    }
    LOCAL.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| "local layout already active")?;
        if slot.is_none() {
            *slot = Some(Engine::new()?);
        }
        let result = slot
            .as_ref()
            .ok_or("local layout unavailable")?
            .layout(input);
        if result.is_err() {
            *slot = None;
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_job_releases_context_and_runtime_without_leaking() -> Result<(), String> {
        let engine = Engine::new()?;
        engine
            .context
            .with(|ctx| -> rquickjs::Result<()> {
                let job: Function = ctx.eval("() => { while (true) {} }")?;
                job.defer(())
            })
            .map_err(|_| "cannot enqueue interrupted job".to_owned())?;
        let probe = Arc::new(());
        let released = Arc::downgrade(&probe);
        engine
            .runtime
            .set_interrupt_handler(Some(Box::new(move || Arc::strong_count(&probe) > 0)));
        assert!(
            engine.execute_job().is_err(),
            "actual native job is interrupted"
        );
        drop(engine);
        assert!(
            released.upgrade().is_none(),
            "failure does not retain a Rust runtime handle"
        );
        Ok(())
    }
    #[test]
    fn pending_job_errors_discard_runtime_without_releasing_a_borrowed_context(
    ) -> Result<(), String> {
        const GRAPH: &str = r#"{"id":"root","layoutOptions":{"elk.algorithm":"layered"},"children":[{"id":"a","width":20,"height":20}]}"#;
        for thrown in [
            "new Error('controlled job failure')",
            "null",
            "undefined",
            "7",
        ] {
            LOCAL.with(|slot| -> Result<(), String> {
                let engine = Engine::new()?;
                engine
                    .context
                    .with(|ctx| -> rquickjs::Result<()> {
                        let fail: Function = ctx.eval(format!("() => {{ throw {thrown}; }}"))?;
                        fail.defer(())
                    })
                    .map_err(|_| "cannot enqueue controlled failure".to_owned())?;
                *slot.borrow_mut() = Some(engine);
                Ok(())
            })?;
            let failure = layout(GRAPH).expect_err("a failed job cannot supply geometry");
            assert!(failure.contains("execution failed"), "{failure}");
            assert!(
                LOCAL.with(|slot| slot.borrow().is_none()),
                "failed runtime is discarded"
            );
            assert!(
                layout(GRAPH).is_ok(),
                "a fresh runtime recovers without stale jobs"
            );
        }
        Ok(())
    }
    #[test]
    fn embedded_runtime_has_no_external_hosts_and_interrupts_work() -> Result<(), String> {
        let engine = Engine::new()?;
        engine.context.with(|ctx| -> rquickjs::Result<()> {
            assert!(ctx.eval::<bool,_>("typeof fetch==='undefined' && typeof require==='undefined' && typeof process==='undefined' && typeof std==='undefined' && typeof Worker==='undefined'")?);
            Ok(())
        }).map_err(|e|e.to_string())?;
        assert!(engine
            .context
            .with(|ctx| ctx.eval::<(), _>("new ArrayBuffer(128 * 1024 * 1024);"))
            .is_err());
        *engine.deadline.lock().map_err(|e| e.to_string())? = Instant::now();
        assert!(engine
            .context
            .with(|ctx| ctx.eval::<(), _>("while(true){}"))
            .is_err());
        Ok(())
    }
    #[test]
    fn execution_errors_and_oversize_are_not_empty_success() {
        assert!(layout(&" ".repeat(65_537)).is_err());
        assert!(layout("not JSON").is_err());
        assert!(layout("{\"id\":\"root\",\"layoutOptions\":{\"elk.algorithm\":\"layered\"},\"children\":[{\"id\":\"a\",\"width\":20,\"height\":20}]}").is_ok());
    }
}
