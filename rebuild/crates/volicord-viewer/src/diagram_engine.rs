//! Local ELK execution. Only trusted installed code runs; graph data is a JSON
//! argument. There are no filesystem, network, process, module or timer hosts.
use rquickjs::{Context, Function, Runtime};
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
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
    // Only a count crosses the native callback: never retain JS values, reasons
    // or contexts past their callback lifetime. QuickJS reports a matching
    // handled notification if a handler is attached after rejection.
    unhandled_rejections: Arc<AtomicUsize>,
}
impl Engine {
    fn new() -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|_| "local layout runtime unavailable")?;
        runtime.set_memory_limit(HEAP_LIMIT);
        runtime.set_max_stack_size(1024 * 1024);
        let unhandled_rejections = Arc::new(AtomicUsize::new(0));
        let pending = unhandled_rejections.clone();
        runtime.set_host_promise_rejection_tracker(Some(Box::new(move |_, _, _, handled| {
            if handled {
                pending.fetch_sub(1, Ordering::Relaxed);
            } else {
                pending.fetch_add(1, Ordering::Relaxed);
            }
        })));
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
            unhandled_rejections,
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
        // Promise callbacks (including the actual asynchronous host shim) turn
        // throws into rejected promises. JS_ExecutePendingJob returns success
        // for those jobs, so its Result alone cannot admit complete geometry.
        if self.unhandled_rejections.load(Ordering::Relaxed) != 0 {
            return Err("local layout asynchronous execution failed".into());
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
    const GRAPH: &str = r#"{"id":"root","layoutOptions":{"elk.algorithm":"layered"},"children":[{"id":"a","width":20,"height":20}]}"#;

    #[test]
    fn actual_callback_failure_reaches_the_shared_svg_consumer() -> Result<(), String> {
        use volicord_projections::*;
        let claim = "Two components exchange an admitted request.";
        let label = |text: &str| DiagramLabel {
            text: text.into(),
            unit: 0,
            start_byte: 0,
            end_byte: claim.len(),
        };
        let diagram = GroundedDiagram {
            id: "request".into(),
            title: label("Request"),
            caption_unit: 0,
            semantic_fingerprint: String::new(),
            meaning: DiagramMeaning::Component {
                nodes: ["a", "b"]
                    .into_iter()
                    .map(|id| DiagramNode {
                        id: id.into(),
                        kind: DiagramNodeKind::Component,
                        label: label(id),
                        identifier: None,
                    })
                    .collect(),
                relations: vec![DiagramRelation {
                    id: "edge".into(),
                    from: "a".into(),
                    to: "b".into(),
                    kind: DiagramRelationKind::ControlFlow,
                    label: label("request"),
                    condition: None,
                }],
            },
        };
        LOCAL.with(|slot| -> Result<(), String> {
            let engine = Engine::new()?;
            engine
                .context
                .with(|ctx| ctx.eval::<(), _>("setTimeout(function(){throw undefined;});"))
                .map_err(|_| "cannot enqueue actual callback failure")?;
            *slot.borrow_mut() = Some(engine);
            Ok(())
        })?;
        let render = || {
            crate::diagram::svg(
                &diagram,
                &[claim],
                "publication",
                &|_, _| "claim-origin".into(),
                FixedLocale::English,
            )
        };
        assert!(
            render().is_err(),
            "shared renderer cannot publish partial SVG as success"
        );
        let recovered = render()?;
        assert!(recovered.contains("claim-origin"));
        assert!(recovered.contains("request"));
        Ok(())
    }

    #[test]
    fn handled_rejections_are_not_unhandled_execution_failures() -> Result<(), String> {
        LOCAL.with(|slot| -> Result<(), String> {
            let engine = Engine::new()?;
            engine.context.with(|ctx| ctx.eval::<(), _>(
                "var rejected = Promise.reject(null); Promise.resolve().then(function(){rejected.catch(function(){});});"
            )).map_err(|_| "cannot enqueue handled rejection")?;
            *slot.borrow_mut() = Some(engine);
            Ok(())
        })?;
        assert!(layout(GRAPH).is_ok());
        assert!(
            layout(GRAPH).is_ok(),
            "successful reuse retains no rejected state"
        );
        Ok(())
    }

    #[test]
    fn native_budget_failures_discard_partial_work_and_recover() -> Result<(), String> {
        for script in [
            "setTimeout(function(){while(true){}});",
            "setTimeout(function again(){setTimeout(again);});",
            "setTimeout(function(){new ArrayBuffer(128*1024*1024);});",
            "setTimeout(function recurse(){recurse();});",
            "takeLocalLayout=function(){return 'x'.repeat(262145);};",
        ] {
            LOCAL.with(|slot| -> Result<(), String> {
                let engine = Engine::new()?;
                engine
                    .context
                    .with(|ctx| ctx.eval::<(), _>(script))
                    .map_err(|_| "cannot prepare bounded failure control")?;
                *slot.borrow_mut() = Some(engine);
                Ok(())
            })?;
            assert!(
                layout(GRAPH).is_err(),
                "bounded failure cannot admit geometry"
            );
            assert!(LOCAL.with(|slot| slot.borrow().is_none()));
            assert!(
                layout(GRAPH).is_ok(),
                "fresh engine recovers after each bound"
            );
        }
        Ok(())
    }

    #[test]
    fn asynchronous_shim_failure_rejects_geometry_and_recovers() -> Result<(), String> {
        for thrown in [
            "new Error('private diagnostic must not escape')",
            "null",
            "undefined",
            "7",
        ] {
            LOCAL.with(|slot| -> Result<(), String> {
                let engine = Engine::new()?;
                engine
                    .context
                    .with(|ctx| {
                        ctx.eval::<(), _>(format!("setTimeout(function() {{ throw {thrown}; }});"))
                    })
                    .map_err(|_| "cannot enqueue asynchronous shim failure")?;
                *slot.borrow_mut() = Some(engine);
                Ok(())
            })?;
            let failure = layout(GRAPH).expect_err("a failed shim callback cannot supply geometry");
            assert!(!failure.contains("private diagnostic"));
            assert!(LOCAL.with(|slot| slot.borrow().is_none()));
            assert!(layout(GRAPH).is_ok(), "a fresh engine recovers");
        }
        Ok(())
    }
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
