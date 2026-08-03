mod session;

pub use session::{Session, SessionEvent, SessionState};

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use gpui::{App, Global};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct Io(Arc<Runtime>);

impl Global for Io {}

impl Io {
    pub fn global(cx: &App) -> Self {
        cx.global::<Self>().clone()
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.spawn(future)
    }
}

pub(crate) async fn join<T>(handle: JoinHandle<Result<T>>) -> Result<T> {
    handle.await?
}
