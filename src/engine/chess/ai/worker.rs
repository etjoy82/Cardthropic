use super::api::{AiConfig, SearchLimits, SearchResult};
use super::search;
use crate::game::ChessPosition;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct AsyncSearch {
    cancel: Arc<AtomicBool>,
    rx: mpsc::Receiver<SearchResult>,
    join: Option<JoinHandle<()>>,
}

impl AsyncSearch {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn try_recv(&self) -> Option<SearchResult> {
        self.rx.try_recv().ok()
    }

    pub fn wait(mut self) -> Option<SearchResult> {
        let result = self.rx.recv().ok();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        result
    }
}

pub fn spawn_search(
    position: ChessPosition,
    limits: SearchLimits,
    config: AiConfig,
) -> AsyncSearch {
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_for_thread = Arc::clone(&cancel);
    let (tx, rx) = mpsc::channel::<SearchResult>();
    let join = std::thread::spawn(move || {
        let result =
            search::iterative::search(&position, limits, config, Some(cancel_for_thread.as_ref()));
        let _ = tx.send(result);
    });
    AsyncSearch {
        cancel,
        rx,
        join: Some(join),
    }
}
