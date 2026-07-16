//! Coalesced transparency config writes.
//!
//! Requests are registered synchronously via [`TransparencyPersist::register`]
//! before their JoinSet tasks are spawned with [`.run()`](TransparencyPersist::run).
//! Every task then takes a process-wide write lock and persists its value only
//! while its generation is still desired. Superseded tasks return
//! [`TaskResult::TransparentBackgroundPersistCoalesced`] so the JoinSet can
//! drain without claiming a disk write. AppView ignores stale generations.
//! Failed writes roll back to the last value known to be on disk.

use std::sync::{LazyLock, Mutex as StdMutex};

use tokio::sync::Mutex as AsyncMutex;

use super::actions::TaskResult;
use super::helpers::persist_setting;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Desired {
    value: bool,
    generation: u64,
}

struct Shared {
    /// Last value known to be on disk.
    confirmed: Option<bool>,
    /// Latest accepted UI choice that should end up on disk.
    desired: Option<Desired>,
    /// Highest generation ever registered, retained after `desired` clears so
    /// a delayed older request can never become current again.
    latest_generation: Option<u64>,
}

static SHARED: LazyLock<StdMutex<Shared>> = LazyLock::new(|| {
    StdMutex::new(Shared {
        confirmed: None,
        desired: None,
        latest_generation: None,
    })
});

/// Serializes transparency disk writes so concurrent JoinSet tasks coalesce
/// onto the latest desired value instead of racing.
static WRITE_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

fn lock_shared() -> std::sync::MutexGuard<'static, Shared> {
    SHARED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Registered transparency persistence work.
///
/// [`TransparencyPersist::register`] mutates process-wide desired state
/// **synchronously**. Call [`.run()`](Self::run) (and spawn that future) so
/// dispatch order — not poll order — decides which generation is desired.
///
/// Do **not** move `register` inside an async block:
/// ```ignore
/// // Wrong: registration delayed until the task is first polled
/// tasks.spawn(async move { TransparencyPersist::register(...).run().await });
/// // Right: register on the dispatch thread, then spawn the write
/// tasks.spawn(TransparencyPersist::register(...).run());
/// ```
#[must_use = "call .run() and spawn/await it after register, or the disk write never starts"]
pub(crate) struct TransparencyPersist {
    value: bool,
    rollback_value: bool,
    generation: u64,
}

impl TransparencyPersist {
    /// Register this generation as desired and return a handle for the write.
    ///
    /// `generation` is the AppView epoch for this optimistic choice. It is used
    /// to decide which desired value is newest and is returned on the result so
    /// dispatch can ignore stale completions.
    pub(crate) fn register(value: bool, rollback_value: bool, generation: u64) -> Self {
        {
            let mut shared = lock_shared();
            // Prime confirmed once from the caller's pre-toggle value (disk truth
            // for a fresh process or first write after reset).
            if shared.confirmed.is_none() {
                shared.confirmed = Some(rollback_value);
            }
            // Keep only the newest generation as desired. `latest_generation` is
            // intentionally not cleared after a successful write: otherwise a
            // delayed older request could resurrect a stale value.
            let keep = shared
                .latest_generation
                .map(|latest| generation >= latest)
                .unwrap_or(true);
            if keep {
                shared.latest_generation = Some(generation);
                shared.desired = Some(Desired { value, generation });
            }
        }

        Self {
            value,
            rollback_value,
            generation,
        }
    }

    /// Persist this handle's value if it is still the desired generation.
    pub(crate) async fn run(self) -> TaskResult {
        let Self {
            value,
            rollback_value,
            generation,
        } = self;

        let _write = WRITE_LOCK.lock().await;

        // Re-check immediately before I/O under the write lock so a newer
        // registration that arrived while we waited cannot be overwritten by
        // a stale generation.
        let is_desired = {
            let shared = lock_shared();
            shared.desired == Some(Desired { value, generation })
        };

        if !is_desired {
            return TaskResult::TransparentBackgroundPersistCoalesced { generation };
        }

        let result = write_value_to_disk(value).await;

        let mut shared = lock_shared();
        match result {
            Ok(()) => {
                // Disk now holds `value`. Record that even if a newer generation
                // registered mid-write — confirmed tracks disk truth until the
                // newer task finishes.
                shared.confirmed = Some(value);
                let still_desired = shared.desired == Some(Desired { value, generation });
                if still_desired {
                    shared.desired = None;
                }
                // Mid-write supersession: we still wrote our value (possible
                // intermediate). Report Persisted for our gen; AppView drops
                // stale gens and the newer task writes next.
                TaskResult::TransparentBackgroundPersisted { value, generation }
            }
            Err(error) => {
                let rollback = shared.confirmed.unwrap_or(rollback_value);
                // Clear desired only if it still points at the failed write.
                if shared.desired == Some(Desired { value, generation }) {
                    shared.desired = None;
                }
                // A newer desired value, if any, belongs to its own task.
                TaskResult::TransparentBackgroundPersistFailed {
                    rollback_value: rollback,
                    generation,
                    error,
                }
            }
        }
    }
}

async fn write_value_to_disk(value: bool) -> Result<(), String> {
    #[cfg(test)]
    if let Some(result) = test_hooks::try_intercept(value) {
        return result;
    }
    persist_setting(
        crate::settings::defs::TRANSPARENT_BACKGROUND_KEY,
        crate::settings::SettingValue::Bool(value),
    )
    .await
}

/// Wait until no write is held and no desired value remains. Shutdown calls
/// this before dropping the event-loop JoinSet.
pub(crate) async fn flush_transparency_persistence() {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        // If a writer holds WRITE_LOCK, wait for it briefly.
        if let Ok(_guard) =
            tokio::time::timeout(std::time::Duration::from_millis(50), WRITE_LOCK.lock()).await
        {
            let pending = lock_shared().desired.is_some();
            if !pending {
                return;
            }
            // Desired was re-queued under the lock; loop to let the owner write.
            drop(_guard);
        }
        if tokio::time::Instant::now() >= deadline {
            tracing::warn!(
                target: "settings",
                "timed out waiting for transparency persistence during shutdown"
            );
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[cfg(test)]
pub(crate) fn reset_transparency_persist_state_for_test() {
    let mut shared = lock_shared();
    shared.confirmed = None;
    shared.desired = None;
    shared.latest_generation = None;
    test_hooks::clear();
}

#[cfg(test)]
pub(crate) fn confirmed_transparency_value_for_test(fallback: bool) -> bool {
    lock_shared().confirmed.unwrap_or(fallback)
}

#[cfg(test)]
mod test_hooks {
    use std::sync::{Mutex, OnceLock};

    static INTERCEPT: OnceLock<Mutex<Option<Box<dyn Fn(bool) -> Result<(), String> + Send>>>> =
        OnceLock::new();

    fn cell() -> &'static Mutex<Option<Box<dyn Fn(bool) -> Result<(), String> + Send>>> {
        INTERCEPT.get_or_init(|| Mutex::new(None))
    }

    pub(super) fn try_intercept(value: bool) -> Option<Result<(), String>> {
        let guard = cell().lock().unwrap_or_else(|p| p.into_inner());
        guard.as_ref().map(|f| f(value))
    }

    pub(crate) fn set_intercept(f: impl Fn(bool) -> Result<(), String> + Send + 'static) {
        *cell().lock().unwrap_or_else(|p| p.into_inner()) = Some(Box::new(f));
    }

    pub(crate) fn clear() {
        *cell().lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// The persistence state and intercept are process-wide. Keep tests that
    /// replace them from running concurrently under the default harness.
    static TEST_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

    async fn lock_test_state() -> tokio::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().await
    }

    #[tokio::test]
    async fn coalesce_writes_only_the_latest_value() {
        let _test = lock_test_state().await;
        reset_transparency_persist_state_for_test();
        let writes = Arc::new(AtomicUsize::new(0));
        let last = Arc::new(StdMutex::new(None));
        {
            let writes = Arc::clone(&writes);
            let last = Arc::clone(&last);
            test_hooks::set_intercept(move |value| {
                writes.fetch_add(1, Ordering::SeqCst);
                *last.lock().unwrap() = Some(value);
                Ok(())
            });
        }

        // `register` runs left-to-right before any future is polled, so only
        // generation 3 remains desired.
        let (a, b, c) = tokio::join!(
            TransparencyPersist::register(true, false, 1).run(),
            TransparencyPersist::register(false, true, 2).run(),
            TransparencyPersist::register(true, false, 3).run(),
        );

        assert!(matches!(
            a,
            TaskResult::TransparentBackgroundPersistCoalesced { generation: 1 }
        ));
        assert!(matches!(
            b,
            TaskResult::TransparentBackgroundPersistCoalesced { generation: 2 }
        ));
        assert!(matches!(
            c,
            TaskResult::TransparentBackgroundPersisted {
                value: true,
                generation: 3,
            }
        ));

        assert_eq!(*last.lock().unwrap(), Some(true));
        assert!(confirmed_transparency_value_for_test(false));
        assert_eq!(
            writes.load(Ordering::SeqCst),
            1,
            "only the latest registered generation may write"
        );

        reset_transparency_persist_state_for_test();
    }

    #[tokio::test]
    async fn dispatch_order_wins_when_older_future_is_polled_last() {
        let _test = lock_test_state().await;
        reset_transparency_persist_state_for_test();
        let written = Arc::new(StdMutex::new(Vec::new()));
        {
            let written = Arc::clone(&written);
            test_hooks::set_intercept(move |value| {
                written.lock().unwrap().push(value);
                Ok(())
            });
        }

        // Calls register synchronously in dispatch order. Poll the latest
        // future first, then the older one, to model reversed task scheduling.
        let older = TransparencyPersist::register(true, false, 1).run();
        let latest = TransparencyPersist::register(false, true, 2).run();
        let latest_result = latest.await;
        let older_result = older.await;

        assert!(matches!(
            latest_result,
            TaskResult::TransparentBackgroundPersisted {
                value: false,
                generation: 2,
            }
        ));
        assert!(matches!(
            older_result,
            TaskResult::TransparentBackgroundPersistCoalesced { generation: 1 }
        ));
        assert_eq!(
            *written.lock().unwrap(),
            vec![false],
            "the delayed older task must not write its stale value"
        );

        reset_transparency_persist_state_for_test();
    }

    #[tokio::test]
    async fn generation_high_water_rejects_stale_request_after_desired_clears() {
        let _test = lock_test_state().await;
        reset_transparency_persist_state_for_test();
        let written = Arc::new(StdMutex::new(Vec::new()));
        {
            let written = Arc::clone(&written);
            test_hooks::set_intercept(move |value| {
                written.lock().unwrap().push(value);
                Ok(())
            });
        }

        let latest = TransparencyPersist::register(true, false, 2).run().await;
        assert!(matches!(
            latest,
            TaskResult::TransparentBackgroundPersisted { generation: 2, .. }
        ));
        let stale = TransparencyPersist::register(false, true, 1).run().await;
        assert!(matches!(
            stale,
            TaskResult::TransparentBackgroundPersistCoalesced { generation: 1 }
        ));
        assert_eq!(
            *written.lock().unwrap(),
            vec![true],
            "clearing desired must not clear the generation high-water mark"
        );

        reset_transparency_persist_state_for_test();
    }

    #[tokio::test]
    async fn failure_rolls_back_to_confirmed_not_effect_local_prev() {
        let _test = lock_test_state().await;
        reset_transparency_persist_state_for_test();
        let calls = Arc::new(AtomicUsize::new(0));
        {
            let calls = Arc::clone(&calls);
            test_hooks::set_intercept(move |value| {
                let n = calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    assert!(value, "first write should be true");
                    Ok(())
                } else {
                    Err("disk full".into())
                }
            });
        }

        let ok = TransparencyPersist::register(true, false, 1).run().await;
        assert!(matches!(
            ok,
            TaskResult::TransparentBackgroundPersisted {
                value: true,
                generation: 1,
            }
        ));
        assert!(confirmed_transparency_value_for_test(false));

        let fail = TransparencyPersist::register(false, true, 2).run().await;
        match fail {
            TaskResult::TransparentBackgroundPersistFailed {
                rollback_value,
                generation: 2,
                error,
            } => {
                assert!(rollback_value, "must roll back to confirmed true, not prev");
                assert_eq!(error, "disk full");
            }
            other => panic!("expected failure, got {other:?}"),
        }
        // Confirmed stays at the last successful disk value.
        assert!(confirmed_transparency_value_for_test(false));

        reset_transparency_persist_state_for_test();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn newer_failure_is_reported_by_the_newer_generation() {
        let _test = lock_test_state().await;
        reset_transparency_persist_state_for_test();
        let calls = Arc::new(AtomicUsize::new(0));
        let (entered_tx, entered_rx) = std::sync::mpsc::channel::<()>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        {
            let calls = Arc::clone(&calls);
            test_hooks::set_intercept(move |value| {
                let n = calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    assert!(value, "generation 1 writes first");
                    let _ = entered_tx.send(());
                    let _ = release_rx.recv();
                    Ok(())
                } else {
                    assert!(!value, "generation 2 owns the second write");
                    Err("disk full".into())
                }
            });
        }

        let first = tokio::spawn(TransparencyPersist::register(true, false, 1).run());
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("first write should enter the hook");

        // Register generation 2 while generation 1 holds WRITE_LOCK. The old
        // worker must not perform or report generation 2's failing write.
        let second = tokio::spawn(TransparencyPersist::register(false, true, 2).run());
        let _ = release_tx.send(());

        assert!(matches!(
            first.await.unwrap(),
            TaskResult::TransparentBackgroundPersisted {
                value: true,
                generation: 1,
            }
        ));
        match second.await.unwrap() {
            TaskResult::TransparentBackgroundPersistFailed {
                rollback_value,
                generation: 2,
                error,
            } => {
                assert!(rollback_value, "roll back to generation 1's disk value");
                assert_eq!(error, "disk full");
            }
            other => panic!("generation 2 must observe its failure, got {other:?}"),
        }

        reset_transparency_persist_state_for_test();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn flush_waits_for_in_flight_write() {
        let _test = lock_test_state().await;
        reset_transparency_persist_state_for_test();
        let (gate_tx, gate_rx) = std::sync::mpsc::channel::<()>();
        {
            test_hooks::set_intercept(move |_value| {
                // Block the write worker until the test releases the gate.
                let _ = gate_rx.recv();
                Ok(())
            });
        }

        let persist = tokio::spawn(TransparencyPersist::register(true, false, 1).run());
        // Model toggle immediately followed by quit: flush starts without
        // waiting for the persistence task to be polled.
        let flush = tokio::spawn(flush_transparency_persistence());
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        assert!(
            !flush.is_finished(),
            "flush must wait while a write is in flight"
        );

        let _ = gate_tx.send(());
        let result = persist.await.unwrap();
        assert!(matches!(
            result,
            TaskResult::TransparentBackgroundPersisted { .. }
        ));
        flush.await.unwrap();

        reset_transparency_persist_state_for_test();
    }
}
