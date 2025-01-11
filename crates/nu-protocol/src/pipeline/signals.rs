use crate::{ErrSpan, IntoSpanned, ShellError, Span, Spanned};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

#[cfg(feature = "async")]
use futures_lite::{future, FutureExt};
use serde::{Deserialize, Serialize};

/// Used to check for signals to suspend or terminate the execution of Nushell code.
///
/// For now, this struct only supports interruption (ctrl+c or SIGINT).
#[derive(Debug, Clone)]
pub struct Signals {
    signals: Option<Arc<AtomicBool>>,
}

impl Signals {
    /// A [`Signals`] that is not hooked up to any event/signals source.
    ///
    /// So, this [`Signals`] will never be interrupted.
    pub const EMPTY: Self = Signals { signals: None };

    /// Create a new [`Signals`] with `ctrlc` as the interrupt source.
    ///
    /// Once `ctrlc` is set to `true`, [`check`](Self::check) will error
    /// and [`interrupted`](Self::interrupted) will return `true`.
    pub fn new(ctrlc: Arc<AtomicBool>) -> Self {
        Self {
            signals: Some(ctrlc),
        }
    }

    /// Create a [`Signals`] that is not hooked up to any event/signals source.
    ///
    /// So, the returned [`Signals`] will never be interrupted.
    ///
    /// This should only be used in test code, or if the stream/iterator being created
    /// already has an underlying [`Signals`].
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Returns an `Err` if an interrupt has been triggered.
    ///
    /// Otherwise, returns `Ok`.
    #[inline]
    pub fn check(&self, span: Span) -> Result<(), ShellError> {
        #[inline]
        #[cold]
        fn interrupt_error(span: Span) -> Result<(), ShellError> {
            Err(ShellError::Interrupted { span })
        }

        if self.interrupted() {
            interrupt_error(span)
        } else {
            Ok(())
        }
    }

    /// Triggers an interrupt.
    pub fn trigger(&self) {
        if let Some(signals) = &self.signals {
            signals.store(true, Ordering::Relaxed);
        }
    }

    /// Returns whether an interrupt has been triggered.
    #[inline]
    pub fn interrupted(&self) -> bool {
        self.signals
            .as_deref()
            .is_some_and(|b| b.load(Ordering::Relaxed))
    }

    /// Polls the [interrupted](`Self::interrupted`) method until an interrupt is triggered.
    #[cfg(feature = "async")]
    async fn interrupted_async(&self) {
        let poller = |_: &mut Context<'_>| match self.interrupted() {
            true => Poll::Ready(()),
            false => Poll::Pending,
        };
        future::poll_fn(poller).await;
        self.reset();
    }

    /// Interrupt protect an async operation.
    #[cfg(feature = "async")]
    pub fn interrupt_protect<T>(&self, fut: impl Future<Output = T>) -> InterruptResult<T> {
        let blocking = async {
            let out = fut.await;
            InterruptResult::Ok(out)
        };
        let interrupt = async {
            self.interrupted_async().await;
            InterruptResult::Interrupted
        };
        future::block_on(blocking.or(interrupt))
    }


    /// Interrupt protect an async operation which returns [Result<T, ShellError>].
    ///
    /// If you have some other Error type which implements [`ErrSpan`],
    /// consider using [`interrupt_protect_err_span`](Self::interrupt_protect_err_span).
    #[cfg(feature = "async")]
    pub fn interrupt_protect_result<T>(
        &self,
        fut: impl Future<Output = Result<T, ShellError>>,
    ) -> Result<T, ShellError>
    where
        T: Send + 'static,
    {
        match self.interrupt_protect(fut) {
            InterruptResult::Ok(inner) => inner,
            InterruptResult::Interrupted => Err(ShellError::InterruptedByUser { span: None }),
        }
    }

    /// Interrupt protect an async operation, and automatically convert its
    /// [`Result<T,E>`] into a [`Result<T, ShellError>`].
    ///
    /// The error type `E` must implement [`ErrSpan`].
    #[cfg(feature = "async")]
    pub fn interrupt_protect_err_span<T, E>(
        &self,
        fut: impl Future<Output = Result<T, E>>,
        span: Span,
    ) -> Result<T, ShellError>
    where
        T: Send + 'static,
        Result<T, E>: ErrSpan,
        Spanned<E>: Into<ShellError>,
    {
        match self.interrupt_protect(fut) {
            // TODO(async): inner.err_span().map_err(Into<ShellError>::into) doesn't seem to work?
            InterruptResult::Ok(inner) => inner.map_err(|err| err.into_spanned(span).into()),
            InterruptResult::Interrupted => Err(ShellError::InterruptedByUser { span: Some(span) }),
        }
    }

    /// No-op for when async is disabled.
    #[cfg(not(feature = "async"))]
    pub fn interrupt_protect<T>(&self, val: T) -> InterruptResult<T> {
        InterruptResult::Ok(val)
    }

    /// No-op for when async is disabled.
    #[cfg(not(feature = "async"))]
    pub fn interrupt_protect_result<T>(&self, val: Result<T, ShellError>) -> Result<T, ShellError> {
        val
    }

    /// No-op for when async is disabled.
    #[cfg(not(feature = "async"))]
    pub fn interrupt_protect_err_span<T, E>(&self, val: Result<T, E>, span: Span) -> Result<T, ShellError>
    where
        Result<T, E>: ErrSpan,
        Spanned<E>: Into<ShellError>,
    {
        val.map_err(|err| err.into_spanned(span).into())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.signals.is_none()
    }

    pub fn reset(&self) {
        if let Some(signals) = &self.signals {
            signals.store(false, Ordering::Relaxed);
        }
    }
}

/// The types of things that can be signaled. It's anticipated this will change as we learn more
/// about how we'd like signals to be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalAction {
    Interrupt,
    Reset,
}

/// The result of an interrupt protected blocking operation.
#[must_use]
pub enum InterruptResult<T> {
    Ok(T),
    Interrupted,
}
