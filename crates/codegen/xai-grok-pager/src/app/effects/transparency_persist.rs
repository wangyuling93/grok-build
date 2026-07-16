//! Coalesced transparency config writes.
//!
//! Every JoinSet task takes a process-wide write lock, then persists only the
//! latest desired value. Superseded tasks still return a [`TaskResult`] tagged
//! with *their* generation so the JoinSet can drain; AppView ignores stale
//! generations. Failed writes roll back to the last value known to be on disk.

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
}

static SHARED: LazyLock<StdMutex<Shared>> = LazyLock::new(|| {
    StdMutex::new(Shared {
        confirmed: None,
        desired: None,
    })
});

/// Serializes transparency disk writes so concurrent JoinSet tasks coalesce
/// onto the latest desired value instead of racing.
static WRITE_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

fn lock_shared() -> std::sync::MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Persist transparency, coalescing concurrent requests under [`WRITE_LOCK`].
///
/// `generation` is the AppView epoch for this optimistic choice. It is used to
/// decide which desired value is newest and is returned on the result so
/// dispatch can ignore stale completions.
pub(crate) async fn persist_transparent_background(
    value: bool,
    rollback_value: bool,
    generation: u64,
) -> TaskResult {
    {
        let mut shared = lock_shared();
        // Prime confirmed once from the caller's pre-toggle value (disk truth
        // for a fresh process or first write after reset).
        if shared.confirmed.is_none() {
            shared.confirmed = Some(rollback_value);
        }
        // Keep only the newest generation as desired.
        let keep = shared
            .desired
            .map(|d| generation >= d.generation)
            .unwrap_or(true);
        if keep {
            shared.desired = Some(Desired { value, generation });
        }
    }

    let _write = WRITE_LOCK.lock().await;

    loop {
        let snapshot = {
            let shared = lock_shared();
            shared.desired
        };

        let Some(Desired {
            value: write_value,
            generation: write_gen,
        }) = snapshot
        else {
            // Fully coalesced away: a peer already wrote the latest value.
            return TaskResult::TransparentBackgroundPersisted { value, generation };
        };

        let result = write_value_to_disk(write_value).await;

        let mut shared = lock_shared();
        match result {
            Ok(()) => {
                shared.confirmed = Some(write_value);
                let still_latest = shared.desired == Some(Desired {
                    value: write_value,
                    generation: write_gen,
                });
                if still_latest {
                    shared.desired = None;
                }
                // If a newer toggle arrived mid-write, loop and write again.
                if !still_latest && shared.desired.is_some() {
                    drop(shared);
                    continue;
                }
                // Report against *this task's* generation. AppView drops stale
                // gens; the task that owns the current gen accepts the result.
                return TaskResult::TransparentBackgroundPersisted { value: write_value, generation };
            }
            Err(error) => {
                let rollback = shared.confirmed.unwrap_or(rollback_value);
                // Clear desired only if it still points at the failed write.
                if shared.desired == Some(Desired {
                    value: write_value,
                    generation: write_gen,
                }) {
                    shared.desired = None;
                } else if shared.desired.is_some() {
                    // Newer desired remains — retry that write.
                    drop(shared);
                    continue;
                }
                return TaskResult::TransparentBackgroundPersistFailed {
                    rollback_value: rollback,
                    generation,
                    error,
                };
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn coalesce_writes_only_the_latest_value() {
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

        let (a, b, c) = tokio::join!(
            persist_transparent_background(true, false, 1),
            persist_transparent_background(false, true, 2),
            persist_transparent_background(true, false, 3),
        );

        // All tasks settle.
        assert!(matches!(
            a,
            TaskResult::TransparentBackgroundPersisted { generation: 1, .. }
        ));
        assert!(matches!(
            b,
            TaskResult::TransparentBackgroundPersisted { generation: 2, .. }
        ));
        assert!(matches!(
            c,
            TaskResult::TransparentBackgroundPersisted {
                value: true,
                generation: 3,
            }
        ));

        // Coalesce: only the final desired value is on disk, and we never write
        // more times than concurrent lock holders (typically 1–3, never thrash).
        assert_eq!(*last.lock().unwrap(), Some(true));
        assert!(confirmed_transparency_value_for_test(false));
        let n = writes.load(Ordering::SeqCst);
        assert!(n >= 1 && n <= 3, "unexpected write count {n}");

        reset_transparency_persist_state_for_test();
    }

    #[tokio::test]
    async fn failure_rolls_back_to_confirmed_not_effect_local_prev() {
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

        let ok = persist_transparent_background(true, false, 1).await;
        assert!(matches!(
            ok,
            TaskResult::TransparentBackgroundPersisted {
                value: true,
                generation: 1,
            }
        ));
        assert!(confirmed_transparency_value_for_test(false));

        let fail = persist_transparent_background(false, true, 2).await;
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
    async fn flush_waits_for_in_flight_write() {
        reset_transparency_persist_state_for_test();
        let (gate_tx, gate_rx) = std::sync::mpsc::channel::<()>();
        {
            test_hooks::set_intercept(move |_value| {
                // Block the write worker until the test releases the gate.
                let _ = gate_rx.recv();
                Ok(())
            });
        }

        let persist = tokio::spawn(persist_transparent_background(true, false, 1));
        // Give the writer time to enter the intercept.
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

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

