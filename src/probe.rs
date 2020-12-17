#[cfg(feature = "tokio_executor")]
use std::{
    future::Future,
    pin::Pin,
};

#[cfg_attr(feature = "tokio_executor", async_trait::async_trait)]
pub trait Probe {
    type Msg: Send;
    type Pay: Clone + Send;
    
    #[cfg(not(feature = "tokio_executor"))]
    fn event(&self, evt: Self::Msg);
    #[cfg(feature = "tokio_executor")]
    fn event(&self, evt: Self::Msg) -> Pin<Box<dyn Future<Output=()> + Send>>;
    fn payload(&self) -> &Self::Pay;
}

#[cfg_attr(feature = "tokio_executor", async_trait::async_trait)]
pub trait ProbeReceive {
    type Msg: Send;

    #[cfg(not(feature = "tokio_executor"))]
    fn recv(&self) -> Self::Msg;
    #[cfg(feature = "tokio_executor")]
    async fn recv(&mut self) -> Self::Msg;
    fn reset_timer(&mut self);
    fn last_event_milliseconds(&self) -> u64;
    fn last_event_seconds(&self) -> u64;
}

/// The channel module provides an std::sync::mpsc::channel() based Probe
/// that is suitable for use in a single, local application.
/// This Probe cannot be serialized.
pub mod channel {
    use super::{Probe, ProbeReceive};

    use chrono::prelude::*;
    #[cfg(not(feature = "tokio_executor"))]
    use std::sync::mpsc::{channel, Sender, Receiver};
    #[cfg(feature = "tokio_executor")]
    use tokio::sync::mpsc::{channel, Sender, Receiver};
    #[cfg(feature = "tokio_executor")]
    use std::{
        future::Future,
        pin::Pin,
    };

    pub fn probe<T: Send>() -> (ChannelProbe<(), T>, ChannelProbeReceive<T>) {
        probe_with_payload(())
    }

    pub fn probe_with_payload<P: Clone + Send, T: Send>(payload: P) -> (ChannelProbe<P, T>, ChannelProbeReceive<T>) {
        #[cfg(not(feature = "tokio_executor"))]
        let (tx, rx) = channel::<T>();
        #[cfg(feature = "tokio_executor")]
        let (tx, rx) = channel::<T>(100);

        let probe = ChannelProbe {
            payload: Some(payload),
            tx: tx.clone()
        };

        let receiver = ChannelProbeReceive {
            rx: rx,
            tx: tx,
            timer_start: Utc::now()
        };

        (probe, receiver)
    }

    #[cfg(not(feature = "tokio_executor"))]
    #[derive(Clone, Debug)]
    pub struct ChannelProbe<P: Send, T> {
        payload: Option<P>,
        tx: Sender<T>,
    }
    #[cfg(feature = "tokio_executor")]
    #[derive(Clone, Debug)]
    pub struct ChannelProbe<P, T> {
        payload: Option<P>,
        tx: Sender<T>,
    }

    #[cfg(not(feature = "tokio_executor"))]
    impl<P, T> Probe for ChannelProbe<P, T> 
        where P: Clone + Send, T: Send {
            type Msg = T;
            type Pay = P;

            fn event(&self, evt: T) {
                drop(self.tx.send(evt));
            }

            fn payload(&self) -> &P {
                &self.payload.as_ref().unwrap()
            }
    }

    #[cfg(not(feature = "tokio_executor"))]
    impl<P, T> Probe for Option<ChannelProbe<P, T>>
        where P: Clone + Send, T: Send {
            type Msg = T;
            type Pay = P;

            fn event(&self, evt: T) {
                drop(self.as_ref().unwrap().tx.send(evt));
            }

            fn payload(&self) -> &P {
                &self.as_ref().unwrap().payload.as_ref().unwrap()
            }
    }
    #[cfg(feature = "tokio_executor")]
    #[async_trait::async_trait]
    impl<P, T: std::fmt::Debug + 'static> Probe for ChannelProbe<P, T> 
        where P: Clone + Send, T: Send {
            type Msg = T;
            type Pay = P;

            fn event(&self, evt: T) -> Pin<Box<dyn Future<Output=()> + Send>> {
                let tx = self.clone().tx.clone();
                Box::pin(async move {
                    tx.send(evt).await.unwrap()
                })
            }

            fn payload(&self) -> &P {
                &self.payload.as_ref().unwrap()
            }
    }

    #[cfg(feature = "tokio_executor")]
    #[async_trait::async_trait]
    impl<P, T: std::fmt::Debug + 'static> Probe for Option<ChannelProbe<P, T>>
        where P: Clone + Send, T: Send {
            type Msg = T;
            type Pay = P;

            fn event(&self, evt: T) -> Pin<Box<dyn Future<Output=()> + Send>> {
                let tx = self.clone().as_ref().unwrap().tx.clone();
                Box::pin(async move {
                    drop(tx.send(evt).await)
                })
            }

            fn payload(&self) -> &P {
                &self.as_ref().unwrap().payload.as_ref().unwrap()
            }
    }

    #[allow(dead_code)]
    pub struct ChannelProbeReceive<T> {
        rx: Receiver<T>,
        tx: Sender<T>,
        timer_start: DateTime<Utc>,
    }

    #[cfg_attr(feature = "tokio_executor", async_trait::async_trait)]
    impl<T: Send> ProbeReceive for ChannelProbeReceive<T> {
        type Msg = T;

        #[cfg(not(feature = "tokio_executor"))]
        fn recv(&self) -> T {
            self.rx.recv().unwrap()
        }
        #[cfg(feature = "tokio_executor")]
        async fn recv(&mut self) -> T {
            self.rx.recv().await.unwrap()
        }

        fn reset_timer(&mut self) {
            self.timer_start = Utc::now();
        }

        fn last_event_milliseconds(&self) -> u64 {
            let now = Utc::now();
            now.time().signed_duration_since(self.timer_start.time()).num_milliseconds() as u64
        }

        fn last_event_seconds(&self) -> u64 {
            let now = Utc::now();
            now.time().signed_duration_since(self.timer_start.time()).num_seconds() as u64
        }
    }
}


/// Macros that provide easy use of Probes
#[macro_use]
pub mod macros {
    /// Mimicks assert_eq!
    /// Performs an assert_eq! on the first event sent by the probe.
    #[cfg(not(feature = "tokio_executor"))]
    #[macro_export]
    macro_rules! p_assert_eq {
        ($listen:expr, $expected:expr) => {
            assert_eq!($listen.recv(), $expected);
        };
    }
    #[cfg(feature = "tokio_executor")]
    #[macro_export]
    macro_rules! p_assert_eq {
        ($listen:expr, $expected:expr) => {
            assert_eq!($listen.recv().await, $expected);
        };
    }

    /// Evaluates events sent from the probe with a vector of expected events.
    /// If an unexpected event is received it will assert!(false).
    /// Each good event is removed from the expected vector.
    /// The assertion is complete when there are no more expected events.
    #[cfg(feature = "tokio_executor")]
    #[macro_export]
    macro_rules! p_assert_events {
        ($listen:expr, $expected:expr) => {
            let mut expected = $expected.clone(); // so we don't need the original mutable
            
            loop {
                let val = $listen.recv().await;
                match expected.iter().position(|x| x == &val) {
                    Some(pos) => {
                        expected.remove(pos);
                        if expected.len() == 0 {
                            break;
                        }
                    }
                    _ => {
                        // probe has received an unexpected event value
                        assert!(false);
                    }
                }
            }
        };
    }
    #[cfg(not(feature = "tokio_executor"))]
    #[macro_export]
    macro_rules! p_assert_events {
        ($listen:expr, $expected:expr) => {
            let mut expected = $expected.clone(); // so we don't need the original mutable
            
            loop {
                let val = $listen.recv();
                match expected.iter().position(|x| x == &val) {
                    Some(pos) => {
                        expected.remove(pos);
                        if expected.len() == 0 {
                            break;
                        }
                    }
                    _ => {
                        // probe has received an unexpected event value
                        assert!(false);
                    }
                }
            }
        };
    }

    #[macro_export]
    macro_rules! p_timer {
        ($listen:expr) => {
            $listen.last_event_milliseconds()
        };
    }

    #[cfg(not(feature = "tokio_executor"))]
    #[cfg(test)]
    mod tests {
        use crate::probe::{Probe, ProbeReceive, channel::probe};

        #[test]
        fn p_assert_eq() {
            let (probe, listen) = probe();

            probe.event("test".to_string());
            
            p_assert_eq!(listen, "test".to_string());
        }

        #[test]
        fn p_assert_events() {
            let (probe, listen) = probe();

            let expected = vec!["event_1", "event_2", "event_3"];
            probe.event("event_1");
            probe.event("event_2");
            probe.event("event_3");
            
            p_assert_events!(listen, expected);
        }

        #[test]
        fn p_timer() {
            let (probe, listen) = probe();
            probe.event("event_3");
            
            println!("Milliseconds: {}", p_timer!(listen));
        }

    }
    #[cfg(feature = "tokio_executor")]
    #[cfg(test)]
    mod tests {
        use crate::probe::{Probe, ProbeReceive, channel::probe};

        #[tokio::test]
        async fn p_assert_eq() {
            let (probe, mut listen) = probe();

            probe.event("test".to_string()).await;
            
            p_assert_eq!(listen, "test".to_string());
        }

        #[tokio::test]
        async fn p_assert_events() {
            let (probe, mut listen) = probe();

            let expected = vec!["event_1", "event_2", "event_3"];
            probe.event("event_1").await;
            probe.event("event_2").await;
            probe.event("event_3").await;
            
            p_assert_events!(listen, expected);
        }

        #[tokio::test]
        async fn p_timer() {
            let (probe, listen) = probe();
            probe.event("event_3").await;
            
            println!("Milliseconds: {}", p_timer!(listen));
        }

    }
}

#[cfg(not(feature = "tokio_executor"))]
#[cfg(test)]
mod tests {
    use super::{Probe, ProbeReceive};
    use super::channel::{probe, probe_with_payload};
    use std::thread;

    #[test]
    fn chan_probe() {
        let (probe, listen) = probe();

        thread::spawn(move || {
            probe.event("some event");
        });

        p_assert_eq!(listen, "some event");
    }

    #[test]
    fn chan_probe_with_payload() {
        let payload = "test data".to_string();
        let (probe, listen) = probe_with_payload(payload);

        thread::spawn(move || {
            // only event the expected result if the payload is what we expect
            if probe.payload() == "test data" {
                probe.event("data received");
            } else {
                probe.event("");
            }
            
        });

        p_assert_eq!(listen, "data received");
    }
}

#[cfg(feature = "tokio_executor")]
#[cfg(test)]
mod tests {
    use super::{Probe, ProbeReceive};
    use super::channel::{probe, probe_with_payload};

    #[tokio::test]
    async fn chan_probe() {
        let (probe, mut listen) = probe();

        tokio::spawn(async move {
            probe.event("some event").await;
        });

        p_assert_eq!(listen, "some event");
    }

    #[tokio::test]
    async fn chan_probe_with_payload() {
        let payload = "test data".to_string();
        let (probe, mut listen) = probe_with_payload(payload);

        tokio::spawn(async move {
            // only event the expected result if the payload is what we expect
            if probe.payload() == "test data" {
                probe.event("data received").await;
            } else {
                probe.event("").await;
            }
            
        });

        p_assert_eq!(listen, "data received");
    }
}
