use std::time::Duration;

use dioxus::{
    prelude::ServerFnError,
    signals::{Signal, Writable},
};
use server::ServerError;

#[derive(PartialEq)]
pub enum PacketState<T> {
    Response(T),
    Waiting,
    ServerError(ServerFnError<ServerError>),
    RequestTimeout,
    NotStarted,
}

impl<T: Clone> Clone for PacketState<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Response(arg0) => Self::Response(arg0.clone()),
            Self::Waiting => Self::Waiting,
            Self::ServerError(arg0) => Self::ServerError(arg0.clone()),
            Self::RequestTimeout => Self::RequestTimeout,
            Self::NotStarted => Self::NotStarted,
        }
    }
}

pub struct PacketSender {
    pub wait_timeout: Duration,
    pub retry_interval: Duration,
}

impl Default for PacketSender {
    fn default() -> Self {
        Self {
            wait_timeout: DEFAULT_WAIT_TIMEOUT,
            retry_interval: DEFAULT_RETRY_INTERVAL,
        }
    }
}

pub const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_RETRY_INTERVAL: Duration = Duration::from_secs(3);

impl PacketSender {
    pub async fn retry<T, F>(&mut self, func: F) -> PacketState<T>
    where
        F: Future<Output = Result<T, ServerFnError<ServerError>>>,
    {
        let value = match tokio::time::timeout(self.wait_timeout, func).await {
            Ok(value) => value,
            Err(elapsed) => {
                eprintln!("Request timed out: {elapsed:?}");
                return PacketState::RequestTimeout;
            }
        };
        match value {
            Ok(value) => PacketState::Response(value),
            Err(err) => PacketState::ServerError(err),
        }
    }

    pub async fn retry_loop<T, F>(
        &mut self,
        mut func: impl FnMut() -> F,
        signal: &mut Signal<PacketState<T>>,
    ) where
        F: Future<Output = Result<T, ServerFnError<ServerError>>>,
    {
        let mut retry_after: bool = true;
        while retry_after {
            signal.set(PacketState::Waiting);

            let state = self.retry(func()).await;
            if matches!(state, PacketState::Response(_)) {
                retry_after = false;
            }

            signal.set(state);
            tokio::time::sleep(self.retry_interval).await;
        }
    }
}

#[macro_export]
macro_rules! future_retry_loop {
    ($future:expr) => {{
        let mut result =
            dioxus::prelude::use_signal(|| $crate::packet_sender::PacketState::Waiting);
        dioxus::prelude::use_future(move || async move {
            $crate::packet_sender::PacketSender::default()
                .retry_loop(|| $future, &mut result)
                .await;
        });
        let value = result.read();
        value.clone()
    }};
    ($signal:ident, $resource:ident, $future:expr) => {
        let mut $signal =
            dioxus::prelude::use_signal(|| $crate::packet_sender::PacketState::Waiting);
        let mut $resource = dioxus::prelude::use_resource(move || async move {
            $crate::packet_sender::PacketSender::default()
                .retry_loop(|| $future, &mut $signal)
                .await;
        });
    };
}
