//! Local ELK execution. Only trusted installed code runs; graph data is a JSON
//! argument. There are no filesystem, network, process, module or timer hosts.
use rquickjs::{Context, Function, Runtime};
use std::{
    mem::ManuallyDrop,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub(super) const ENGINE_ID: &str =
    "elkjs/0.11.1;quickjs-ng/rquickjs-0.11.0;layered-polyline/1;metrics/2;worker/1";
const SOURCE: &str = include_str!("../vendor/elkjs/elk.bundled.js");
const HEAP_LIMIT: usize = 64 * 1024 * 1024;
pub(super) const OUTPUT_LIMIT: usize = 262_144;
pub(super) const DEADLINE: Duration = Duration::from_secs(3);
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

pub(super) struct Engine {
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
    pub(super) fn new() -> Result<Self, String> {
        #[cfg(test)]
        initialization_trace("runtime-create");
        // This constructor runs only inside the disposable native worker.
        // A failed native initialization may leave a partially linked Context
        // in the VM's GC list. Do not call native destructors on that state:
        // retain both handles until the worker reports failure and exits.
        // Fully initialized engines regain ordinary ownership below.
        let runtime =
            ManuallyDrop::new(Runtime::new().map_err(|_| "local layout runtime unavailable")?);
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
        #[cfg(test)]
        if std::env::var("VOLICORD_LAYOUT_TEST_ENGINE").as_deref() == Ok("context-memory") {
            // Permit the native Context allocation, but refuse its prototype
            // array. This control is separate from a layout-operation budget.
            runtime.set_memory_limit(runtime.memory_usage().malloc_size as usize + 1024);
        }
        #[cfg(test)]
        initialization_trace("context-create");
        let context = ManuallyDrop::new(
            Context::full(&runtime).map_err(|_| "local layout context unavailable")?,
        );
        #[cfg(test)]
        if std::env::var("VOLICORD_LAYOUT_TEST_ENGINE").as_deref() == Ok("bundle-memory") {
            // Admit the small host shim, but reject the embedded bundle's
            // compilation. Keep this separate from Context allocation failure.
            runtime.set_memory_limit(runtime.memory_usage().malloc_size as usize + 16 * 1024);
        }
        context
            .with(|ctx| -> rquickjs::Result<()> {
                #[cfg(test)]
                initialization_trace("host-eval");
                ctx.eval::<(), _>(HOST)?;
                #[cfg(test)]
                initialization_trace("bundle-eval");
                ctx.eval::<(), _>(SOURCE)?;
                #[cfg(test)]
                initialization_trace("adapter-eval");
                ctx.eval::<(), _>(ADAPTER)
            })
            .map_err(|_| "local layout initialization failed or exceeded its bound")?;
        Ok(Self {
            context: ManuallyDrop::into_inner(context),
            runtime: ManuallyDrop::into_inner(runtime),
            deadline,
            unhandled_rejections,
        })
    }

    pub(super) fn layout(&self, input: &str) -> Result<String, String> {
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

    #[cfg(test)]
    pub(super) fn enqueue_test_callback(&self) -> Result<(), String> {
        self.context
            .with(|ctx| ctx.eval::<(), _>("setTimeout(function(){throw undefined;});"))
            .map_err(|_| "cannot enqueue controlled native callback".into())
    }

    // Closed fixtures run inside the same disposable process as production
    // native execution. Never dispose a deliberately damaged VM in the reader.
    #[cfg(test)]
    pub(super) fn controlled(control: &str) -> Result<Self, String> {
        let engine = Self::new()?;
        let entered = Arc::new(AtomicUsize::new(0));
        let reached = entered.clone();
        initialization_trace("bridge-install");
        engine
            .context
            .with(|ctx| -> rquickjs::Result<()> {
                ctx.globals().set(
                    "nativeControlReached",
                    Function::new(ctx.clone(), move || {
                        reached.store(1, Ordering::Relaxed);
                        super::worker::test_control_reached("operation");
                    })?,
                )
            })
            .map_err(|_| "cannot install native control probe")?;
        super::worker::test_control_reached("prepared");
        if matches!(control, "interrupted-job" | "handle-probe-failure") {
            engine
                .context
                .with(|ctx| -> rquickjs::Result<()> {
                    let job: Function =
                        ctx.eval("() => { nativeControlReached(); while (true) {} }")?;
                    job.defer(())
                })
                .map_err(|_| "cannot enqueue interrupted control")?;
            let probe = Arc::new(());
            let released = Arc::downgrade(&probe);
            let retained = (control == "handle-probe-failure").then(|| probe.clone());
            engine.runtime.set_interrupt_handler(Some(Box::new(move || {
                let _keep_alive = &probe;
                entered.load(Ordering::Relaxed) > 0
            })));
            let outcome = engine.execute_job();
            drop(engine);
            if released.upgrade().is_some() {
                drop(retained);
                return Err("native control retained a Rust runtime handle".into());
            }
            super::worker::test_control_reached("handle-released");
            return match outcome {
                Err(error) => Err(error.into()),
                Ok(_) => Err("interrupted control unexpectedly completed".into()),
            };
        }
        let job = match control {
            "job-error" => {
                Some("() => { nativeControlReached(); throw new Error('controlled job failure'); }")
            }
            "job-null" => Some("() => { nativeControlReached(); throw null; }"),
            "job-undefined" => Some("() => { nativeControlReached(); throw undefined; }"),
            "job-number" => Some("() => { nativeControlReached(); throw 7; }"),
            _ => None,
        };
        if let Some(job) = job {
            engine
                .context
                .with(|ctx| -> rquickjs::Result<()> {
                    let job: Function = ctx.eval(job)?;
                    job.defer(())
                })
                .map_err(|_| "cannot enqueue failed-job control")?;
            return Ok(engine);
        }
        let script = match control {
            "async-error" => "setTimeout(function(){nativeControlReached();throw new Error('private diagnostic must not escape');});",
            "async-null" => "setTimeout(function(){nativeControlReached();throw null;});",
            "async-undefined" => "setTimeout(function(){nativeControlReached();throw undefined;});",
            "async-number" => "setTimeout(function(){nativeControlReached();throw 7;});",
            "time" => "setTimeout(function(){nativeControlReached();while(true){}});",
            "jobs" => "nativeControlReached();setTimeout(function again(){setTimeout(again);});",
            "memory" => "setTimeout(function(){nativeControlReached();new ArrayBuffer(128*1024*1024);});",
            "stack" => "setTimeout(function(){nativeControlReached();(function recurse(){recurse();})();});",
            "output" => "takeLocalLayout=function(){nativeControlReached();return 'x'.repeat(262145);};",
            "handled" => "var rejected=Promise.reject(null);Promise.resolve().then(function(){rejected.catch(function(){});});",
            "restricted-hosts" => "if(!(typeof fetch==='undefined'&&typeof require==='undefined'&&typeof process==='undefined'&&typeof std==='undefined'&&typeof Worker==='undefined'))throw new Error('unexpected external host');",
            _ => return Err("unknown native test control".into()),
        };
        engine
            .context
            .with(|ctx| ctx.eval::<(), _>(script))
            .map_err(|_| "cannot prepare native control")?;
        Ok(engine)
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

// Content-free, test-only attribution for faults that happen before `prepared`.
// Writing a phase marks entry, not completion, and never proves callback entry.
#[cfg(test)]
fn initialization_trace(stage: &str) {
    if std::env::var_os("VOLICORD_LAYOUT_TEST_TRACE").is_some() {
        super::worker::test_control_reached(stage);
    }
}

// Geometry and authority never cross this boundary as retained state. The
// supervisor discards a failed native process before another request can use it.
pub(super) fn layout(input: &str) -> Result<String, String> {
    super::worker::layout(input)
}

#[cfg(test)]
mod tests {
    use super::super::worker::{
        layout,
        proof::{assert_geometry, Failure, Mechanism, GRAPH},
        select_test_engine, test_control_trace, test_worker_pid,
    };

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
        super::super::worker::select_test_worker(
            "diagram::overview::worker::tests::callback_failure_child",
        );
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

    fn reject_and_recover(control: &'static str, expected: &[Mechanism<'_>]) -> Result<(), String> {
        select_test_engine(control);
        let failure = layout(GRAPH).expect_err("failed native control cannot admit geometry");
        let trace = test_control_trace();
        let outcome = Failure::parse(&failure);
        eprintln!("native control={control}; trace={trace:?}; {failure}");
        assert!(
            trace.iter().any(|s| s == "prepared"),
            "control initialization: {failure}"
        );
        assert_eq!(
            trace.get(..6),
            Some(
                [
                    "runtime-create",
                    "context-create",
                    "host-eval",
                    "bundle-eval",
                    "adapter-eval",
                    "bridge-install"
                ]
                .map(str::to_owned)
                .as_slice()
            ),
            "initialization phases must precede control preparation"
        );
        assert!(
            trace.iter().any(|s| s == "operation"),
            "intended operation never reached: {failure}"
        );
        assert!(
            expected.contains(&outcome.mechanism()),
            "{control}: {outcome:?}"
        );
        assert_eq!(
            outcome.stage,
            if control == "interrupted-job" {
                "initialization"
            } else {
                "request"
            }
        );
        if control == "interrupted-job" {
            assert!(
                trace.iter().any(|s| s == "handle-released"),
                "missing handle-probe result"
            );
        }
        assert!(!failure.contains("private diagnostic"));
        assert!(
            test_worker_pid().is_none(),
            "failed child must be discarded"
        );
        outcome.assert_reaped();
        let recovered = layout(GRAPH)?;
        assert_geometry(&recovered);
        assert_ne!(test_worker_pid(), Some(outcome.pid as u32));
        assert!(
            test_worker_pid().is_some(),
            "fresh child serves the next request"
        );
        Ok(())
    }

    #[test]
    fn initialization_failures_discard_partial_state_and_recover() -> Result<(), String> {
        for (control, reason, phases) in [
            ("context-memory", "local layout context unavailable", 2),
            (
                "bundle-memory",
                "local layout initialization failed or exceeded its bound",
                4,
            ),
        ] {
            select_test_engine(control);
            let failure = layout(GRAPH).expect_err("failed initialization cannot admit geometry");
            let trace = test_control_trace();
            let attributed = Failure::parse(&failure);
            eprintln!("initialization control={control}; trace={trace:?}; {failure}");
            assert_eq!(attributed.stage, "initialization");
            assert_eq!(attributed.mechanism(), Mechanism::EngineRejection(reason));
            assert_eq!(
                trace,
                [
                    "runtime-create",
                    "context-create",
                    "host-eval",
                    "bundle-eval"
                ][..phases]
            );
            attributed.assert_reaped();
            assert!(test_worker_pid().is_none());
            assert_geometry(&layout(GRAPH)?);
        }
        Ok(())
    }

    #[test]
    fn handled_rejections_are_not_unhandled_execution_failures() -> Result<(), String> {
        select_test_engine("handled");
        assert_geometry(&layout(GRAPH)?);
        let original = test_worker_pid().ok_or("successful worker")?;
        layout(GRAPH)?;
        assert_eq!(
            test_worker_pid(),
            Some(original),
            "successful reuse retains no rejected state"
        );
        Ok(())
    }

    #[test]
    fn native_budget_failures_discard_partial_work_and_recover() -> Result<(), String> {
        for control in ["time", "jobs", "memory", "stack", "output"] {
            let reasons: &[Mechanism<'_>] = match control {
                "time" => &[
                    Mechanism::Timeout,
                    Mechanism::EngineRejection("local layout time/job budget exceeded"),
                    Mechanism::EngineRejection("local layout asynchronous execution failed"),
                ],
                "jobs" => &[Mechanism::EngineRejection(
                    "local layout time/job budget exceeded",
                )],
                "memory" | "stack" => &[Mechanism::EngineRejection(
                    "local layout asynchronous execution failed",
                )],
                "output" => &[Mechanism::EngineRejection(
                    "local layout geometry budget exceeded",
                )],
                _ => unreachable!(),
            };
            reject_and_recover(control, reasons)?;
        }
        Ok(())
    }

    #[test]
    fn asynchronous_shim_failure_rejects_geometry_and_recovers() -> Result<(), String> {
        for control in [
            "async-error",
            "async-null",
            "async-undefined",
            "async-number",
        ] {
            reject_and_recover(
                control,
                &[Mechanism::EngineRejection(
                    "local layout asynchronous execution failed",
                )],
            )?;
        }
        Ok(())
    }

    #[test]
    fn interrupted_job_discards_child_and_allows_fresh_geometry() -> Result<(), String> {
        reject_and_recover(
            "interrupted-job",
            &[Mechanism::EngineRejection(
                "local layout execution failed or exceeded its bound",
            )],
        )
    }

    #[test]
    fn pending_job_errors_discard_process_without_reusing_a_failed_context() -> Result<(), String> {
        for control in ["job-error", "job-null", "job-undefined", "job-number"] {
            reject_and_recover(
                control,
                &[Mechanism::EngineRejection(
                    "local layout execution failed or exceeded its bound",
                )],
            )?;
        }
        Ok(())
    }

    #[test]
    fn embedded_runtime_has_no_external_hosts() -> Result<(), String> {
        select_test_engine("restricted-hosts");
        layout(GRAPH)?;
        Ok(())
    }

    #[test]
    fn explicit_handle_probe_failure_cannot_pass_as_interruption() -> Result<(), String> {
        select_test_engine("handle-probe-failure");
        let failure = layout(GRAPH).expect_err("held handle must fail");
        let outcome = Failure::parse(&failure);
        assert_eq!(
            outcome.mechanism(),
            Mechanism::EngineRejection("native control retained a Rust runtime handle")
        );
        assert!(test_control_trace().iter().any(|s| s == "operation"));
        assert!(!test_control_trace().iter().any(|s| s == "handle-released"));
        outcome.assert_reaped();
        assert_geometry(&layout(GRAPH)?);
        Ok(())
    }

    #[test]
    fn cleanup_signal_never_supplies_a_failure_mechanism() {
        for (primary, stage, mechanism) in [
            (
                "local layout worker protocol rejected",
                "initialization",
                Mechanism::Protocol,
            ),
            (
                "local layout worker ended before a complete response",
                "initialization",
                Mechanism::Initialization,
            ),
            (
                "local layout worker deadline exceeded",
                "request",
                Mechanism::Timeout,
            ),
        ] {
            let diagnostic = format!("{primary}; stage={stage}; worker_pid=123; observed=running_or_unobserved; cleanup=signal:9; native_startup=");
            let failure = Failure::parse(&diagnostic);
            assert_eq!(failure.mechanism(), mechanism);
            assert!(!matches!(
                failure.mechanism(),
                Mechanism::EngineRejection(_) | Mechanism::NativeFault(_)
            ));
        }
    }

    #[test]
    fn execution_errors_and_oversize_are_not_empty_success() -> Result<(), String> {
        select_test_engine("restricted-hosts");
        assert!(layout(&" ".repeat(65_537)).is_err());
        assert!(
            test_worker_pid().is_none(),
            "input bound is checked before native execution"
        );
        assert!(layout("not JSON").is_err());
        assert!(test_worker_pid().is_none());
        layout(GRAPH)?;
        Ok(())
    }
}
