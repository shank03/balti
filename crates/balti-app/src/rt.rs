use gpui::*;

pub fn init(cx: &mut App) {
    cx.set_global(GlobalTokio::new());
}

struct GlobalTokio {
    rt: tokio::runtime::Runtime,
}

impl Global for GlobalTokio {}

impl GlobalTokio {
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(
                std::thread::available_parallelism()
                    .map(|v| v.get())
                    .unwrap_or(4),
            )
            .enable_all()
            .build()
            .expect("Failed to build tokio");
        Self { rt }
    }
}

pub fn spawn<C, Fut, R>(cx: &C, f: Fut) -> C::Result<Task<Result<R, super::err::AppError>>>
where
    C: AppContext,
    Fut: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    cx.read_global(|rt: &GlobalTokio, cx| {
        let join = rt.rt.spawn(f);
        let abort = join.abort_handle();
        let cancel = defer(move || {
            abort.abort();
        });
        cx.background_spawn(async move {
            let result = join.await;
            drop(cancel);
            result.map_err(super::err::AppError::from)
        })
    })
}

pub struct Deferred<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for Deferred<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f()
        }
    }
}

/// Run the given function when the returned value is dropped (unless it's cancelled).
#[must_use]
pub fn defer<F: FnOnce()>(f: F) -> Deferred<F> {
    Deferred(Some(f))
}
