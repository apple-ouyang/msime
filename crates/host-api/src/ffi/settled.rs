//! Settled rerank that does not run the model on the session thread.
//!
//! The 24M desktop model measures p95 153ms. macOS delivers keystrokes on the same thread as the
//! settle timer, so a synchronous `msime_client_rerank_settled` makes the next key — often space,
//! pressed once the user has paused to look — wait until the forward pass returns. The worker
//! below owns the reranker and a copy of the rows. It never calls `with_session`.

use crate::*;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};

struct SettledJob {
    session: u64,
    inner: Mutex<SettledJobInner>,
}

struct SettledJobInner {
    abandoned: bool,
    phase: SettledPhase,
}

enum SettledPhase {
    Running,
    Ready(msime_input_runtime::SettledRerankScore),
    Done,
}

struct Registry {
    jobs: HashMap<u64, Arc<SettledJob>>,
    by_session: HashMap<u64, u64>,
}

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

fn registry() -> std::sync::MutexGuard<'static, Registry> {
    REGISTRY
        .get_or_init(|| {
            Mutex::new(Registry {
                jobs: HashMap::new(),
                by_session: HashMap::new(),
            })
        })
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

fn job_inner(job: &SettledJob) -> std::sync::MutexGuard<'_, SettledJobInner> {
    job.inner.lock().unwrap_or_else(|error| error.into_inner())
}

/// Drop an in-flight score for a session that is going away. The worker may still be inside the
/// model; it sees `abandoned` and drops the reranker instead of publishing it.
pub(crate) fn abandon_settled_job(handle: u64) {
    let job = {
        let mut registry = registry();
        let Some(token) = registry.by_session.remove(&handle) else {
            return;
        };
        registry.jobs.remove(&token)
    };
    let Some(job) = job else {
        return;
    };
    let mut inner = job_inner(&job);
    inner.abandoned = true;
    if let SettledPhase::Ready(score) = std::mem::replace(&mut inner.phase, SettledPhase::Done) {
        drop(score);
    }
}

fn open_token(handle: u64) -> Option<u64> {
    let mut registry = registry();
    let token = *registry.by_session.get(&handle)?;
    let finished = registry
        .jobs
        .get(&token)
        .is_some_and(|job| matches!(job_inner(job).phase, SettledPhase::Done));
    if finished {
        registry.jobs.remove(&token);
        registry.by_session.remove(&handle);
        return None;
    }
    Some(token)
}

fn remove_job(handle: u64, token: u64) {
    let mut registry = registry();
    registry.jobs.remove(&token);
    if registry.by_session.get(&handle).copied() == Some(token) {
        registry.by_session.remove(&handle);
    }
}

fn publish(
    session: &mut HostSession,
    score: msime_input_runtime::SettledRerankScore,
) -> Result<Value, String> {
    let pending = session.lifecycle_begin(lifecycle::Call::SideChannel);
    let applied = session.runtime.apply_settled_rerank(score);
    if applied.moved {
        let view = session.runtime.view();
        session.lifecycle_view(pending, &view);
        Ok(json!({"status": "done", "moved": true, "current": true, "view": view}))
    } else {
        Ok(json!({"status": "done", "moved": false, "current": applied.current}))
    }
}

fn next_token() -> u64 {
    loop {
        let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
        if token != 0 {
            return token;
        }
    }
}

fn spawn_score(
    handle: u64,
    request: msime_input_runtime::SettledRerankRequest,
) -> Result<u64, Box<msime_input_runtime::SettledRerankRequest>> {
    let token = next_token();
    let job = Arc::new(SettledJob {
        session: handle,
        inner: Mutex::new(SettledJobInner {
            abandoned: false,
            phase: SettledPhase::Running,
        }),
    });
    let (sender, receiver) = mpsc::channel();
    let worker = Arc::clone(&job);
    let spawned = std::thread::Builder::new()
        .name("msime-settled-rerank".into())
        .spawn(move || {
            let Ok(request) = receiver.recv() else {
                return;
            };
            let scored = catch_unwind(AssertUnwindSafe(|| {
                msime_input_runtime::score_settled_rerank(request)
            }));
            let mut inner = job_inner(&worker);
            match scored {
                Ok(score) if !inner.abandoned => inner.phase = SettledPhase::Ready(score),
                Ok(score) => {
                    drop(score);
                    inner.phase = SettledPhase::Done;
                }
                Err(_) => {
                    // The request, including the reranker, was dropped by the unwind.
                    inner.abandoned = true;
                    inner.phase = SettledPhase::Done;
                }
            }
        });
    if spawned.is_err() {
        return Err(Box::new(request));
    }
    if let Err(sent) = sender.send(request) {
        return Err(Box::new(sent.0));
    }
    let mut registry = registry();
    registry.jobs.insert(token, job);
    registry.by_session.insert(handle, token);
    Ok(token)
}

fn settled_on_session(session: &mut HostSession) -> Result<Value, String> {
    let pending = session.lifecycle_begin(lifecycle::Call::SideChannel);
    let moved = session.runtime.rerank_settled();
    if moved {
        let view = session.runtime.view();
        session.lifecycle_view(pending, &view);
        Ok(json!({"status": "done", "moved": true, "current": true, "view": view}))
    } else {
        Ok(json!({"status": "done", "moved": false, "current": true}))
    }
}

/// Start a settled rerank without running the model on this thread.
///
/// `{"status":"done","moved":false,"current":true}` when nothing should change, or when only the
/// cheap presentation passes ran here. `current` means this composition has been answered.
/// `{"status":"done","moved":true,"current":true,"view":...}` when those passes reordered.
/// `{"status":"pending","token":N}` when the model is scoring in the background. One score per
/// session: a second start returns the same token.
#[no_mangle]
pub extern "C" fn msime_client_start_settled_rerank(handle: u64) -> *mut c_char {
    response(|| {
        with_session(handle, |session| {
            if let Some(token) = open_token(handle) {
                return Ok(json!({"status": "pending", "token": token}));
            }
            let Some(request) = session.runtime.begin_settled_rerank() else {
                return settled_on_session(session);
            };
            match spawn_score(handle, request) {
                Ok(token) => Ok(json!({"status": "pending", "token": token})),
                Err(request) => {
                    let score = msime_input_runtime::score_settled_rerank(*request);
                    publish(session, score)
                }
            }
        })
    })
}

/// Collect a score started by [`msime_client_start_settled_rerank`]. Pending until the worker
/// finishes. `accept` is false when the host has already left this composition: the model is put
/// back and the list is not reordered. Call it on the session thread.
#[no_mangle]
pub extern "C" fn msime_client_poll_settled_rerank(
    handle: u64,
    token: u64,
    accept: bool,
) -> *mut c_char {
    response(|| {
        if token == 0 {
            return Ok(json!({"status": "done", "moved": false, "current": false}));
        }
        with_session(handle, |session| {
            let job = registry().jobs.get(&token).cloned();
            let Some(job) = job else {
                return Ok(json!({"status": "done", "moved": false, "current": false}));
            };
            if job.session != handle {
                return Ok(json!({"status": "done", "moved": false, "current": false}));
            }
            let take = {
                let mut inner = job_inner(&job);
                match std::mem::replace(&mut inner.phase, SettledPhase::Done) {
                    SettledPhase::Running => {
                        inner.phase = SettledPhase::Running;
                        Take::Pending
                    }
                    SettledPhase::Ready(score) => Take::Ready(score),
                    SettledPhase::Done => Take::Absent,
                }
            };
            match take {
                Take::Pending => Ok(json!({"status": "pending", "token": token})),
                Take::Absent => {
                    remove_job(handle, token);
                    Ok(json!({"status": "done", "moved": false, "current": false}))
                }
                Take::Ready(score) => {
                    remove_job(handle, token);
                    if !accept {
                        session.runtime.discard_settled_rerank(score);
                        Ok(json!({"status": "done", "moved": false, "current": false}))
                    } else {
                        publish(session, score)
                    }
                }
            }
        })
    })
}

enum Take {
    Pending,
    Ready(msime_input_runtime::SettledRerankScore),
    Absent,
}

#[cfg(test)]
pub(crate) fn test_hold_settled_job(handle: u64) -> u64 {
    let token = next_token();
    let job = Arc::new(SettledJob {
        session: handle,
        inner: Mutex::new(SettledJobInner {
            abandoned: false,
            phase: SettledPhase::Running,
        }),
    });
    let mut registry = registry();
    registry.jobs.insert(token, job);
    registry.by_session.insert(handle, token);
    token
}

#[cfg(test)]
pub(crate) fn test_settled_job_count() -> usize {
    registry().jobs.len()
}
