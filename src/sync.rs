#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::ops::{Deref, DerefMut};

    pub struct Notify(tokio::sync::Notify);

    impl Default for Notify {
        fn default() -> Self {
            Notify(tokio::sync::Notify::default())
        }
    }

    impl Notify {
        pub fn new() -> Self {
            Self::default()
        }

        pub async fn notified(&self) {
            self.0.notified().await
        }

        pub fn notify_one(&self) {
            self.0.notify_one();
        }

        pub fn notify_waiters(&self) {
            self.0.notify_waiters();
        }
    }

    pub struct RwLock<T: ?Sized>(tokio::sync::RwLock<T>);
    unsafe impl<T> Send for RwLock<T> where T: ?Sized + Send {}
    unsafe impl<T> Sync for RwLock<T> where T: ?Sized + Send + Sync {}

    impl<T: ?Sized> RwLock<T> {
        pub fn new(value: T) -> Self
        where
            T: Sized,
        {
            Self(tokio::sync::RwLock::new(value))
        }

        pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
            RwLockWriteGuard(self.0.write().await)
        }

        pub async fn read(&self) -> RwLockReadGuard<'_, T> {
            RwLockReadGuard(self.0.read().await)
        }
    }

    pub struct RwLockWriteGuard<'a, T: ?Sized>(tokio::sync::RwLockWriteGuard<'a, T>);

    impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    pub struct RwLockReadGuard<'a, T: ?Sized>(tokio::sync::RwLockReadGuard<'a, T>);

    impl<'a, T> Deref for RwLockReadGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    pub mod oneshot {
        use std::{
            future::Future,
            pin::Pin,
            task::{Context, Poll},
        };

        pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
            let (sender, receiver) = tokio::sync::oneshot::channel();
            (Sender(sender), Receiver(receiver))
        }

        pub struct Sender<T>(tokio::sync::oneshot::Sender<T>);

        impl<T> Sender<T> {
            pub fn send(self, value: T) -> Result<(), T> {
                self.0.send(value)
            }
        }

        #[derive(Debug, Eq, PartialEq, Clone)]
        pub struct RecvError(());

        pub struct Receiver<T>(tokio::sync::oneshot::Receiver<T>);

        impl<T> Future for Receiver<T> {
            type Output = Result<T, RecvError>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                Future::poll(Pin::new(&mut self.0), cx)
                    .map(|value| value.map_err(|_| RecvError(())))
            }
        }
    }

    pub mod mpsc {
        pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
            let (sender, receiver) = tokio::sync::mpsc::channel(buffer);
            (Sender(sender), Receiver(receiver))
        }

        #[derive(Debug)]
        pub struct Sender<T>(tokio::sync::mpsc::Sender<T>);

        impl<T> Clone for Sender<T> {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        impl<T> Sender<T> {
            pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
                self.0.send(value).await.map_err(|err| SendError(err.0))
            }

            pub fn blocking_send(&self, value: T) -> Result<(), SendError<T>> {
                tokio::task::block_in_place(|| {
                    self.0.blocking_send(value).map_err(|err| SendError(err.0))
                })
            }
        }

        #[derive(PartialEq, Eq, Clone, Copy)]
        pub struct SendError<T>(pub T);

        impl<T> std::fmt::Debug for SendError<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("SendError").finish_non_exhaustive()
            }
        }

        impl<T> std::fmt::Display for SendError<T> {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(fmt, "channel closed")
            }
        }

        impl<T> std::error::Error for SendError<T> {}

        #[derive(PartialEq, Eq, Clone, Copy, Debug)]
        pub enum TryRecvError {
            Empty,
            Disconnected,
        }

        impl std::fmt::Display for TryRecvError {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match *self {
                    TryRecvError::Empty => "receiving on an empty channel".fmt(fmt),
                    TryRecvError::Disconnected => "receiving on a closed channel".fmt(fmt),
                }
            }
        }

        impl std::error::Error for TryRecvError {}

        #[derive(Debug)]
        pub struct Receiver<T>(tokio::sync::mpsc::Receiver<T>);

        impl<T> Unpin for Receiver<T> {}

        impl<T> Receiver<T> {
            pub async fn recv(&mut self) -> Option<T> {
                self.0.recv().await
            }

            pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
                self.0.try_recv().map_err(|err| match err {
                    tokio::sync::mpsc::error::TryRecvError::Empty => TryRecvError::Empty,
                    tokio::sync::mpsc::error::TryRecvError::Disconnected => TryRecvError::Disconnected,
                })
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::{
        cell::UnsafeCell,
        collections::VecDeque,
        ops::{Deref, DerefMut},
        sync::Arc,
    };

    use js_sys::{Function, Promise};
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;

    pub struct InteriorMutability<T: ?Sized> {
        cell: UnsafeCell<T>,
    }
    unsafe impl<T: ?Sized> Send for InteriorMutability<T> {}

    impl<T: ?Sized> InteriorMutability<T> {
        pub fn new(value: T) -> Self
        where
            T: Sized,
        {
            Self {
                cell: UnsafeCell::new(value),
            }
        }
    }

    impl<T: ?Sized> InteriorMutability<T> {
        pub fn as_ref(&self) -> &T {
            unsafe { &*self.cell.get() }
        }

        pub fn as_mut(&self) -> &mut T {
            unsafe { &mut *self.cell.get() }
        }
    }

    #[derive(Clone)]
    pub struct Notify {
        permit: Arc<InteriorMutability<bool>>,
        wait_list: Arc<InteriorMutability<VecDeque<Function>>>,
    }

    impl Default for Notify {
        fn default() -> Self {
            Notify {
                permit: Arc::new(InteriorMutability::new(false)),
                wait_list: Arc::new(InteriorMutability::new(VecDeque::new())),
            }
        }
    }

    impl Notify {
        pub fn new() -> Self {
            Self::default()
        }

        pub async fn notified(&self) {
            if *(*self.permit).as_ref() {
                *self.permit.as_mut() = false;
            } else {
                let mut notify = None;
                let promise = Promise::new(&mut |resolve, _reject| {
                    notify = Some(resolve);
                });
                self.wait_list.as_mut().push_back(notify.unwrap());
                JsFuture::from(promise).await.unwrap();
            }
        }

        pub fn notify_one(&self) {
            if let Some(notify) = self.wait_list.as_mut().pop_front() {
                notify.call0(&JsValue::null()).unwrap();
            } else {
                *self.permit.as_mut() = true;
            }
        }

        pub fn notify_waiters(&self) {
            while let Some(notify) = self.wait_list.as_mut().pop_front() {
                notify.call0(&JsValue::null()).unwrap();
            }
        }
    }

    pub struct RwLock<T: ?Sized> {
        writers: Arc<InteriorMutability<usize>>,
        write_notify: Arc<Notify>,
        readers: Arc<InteriorMutability<usize>>,
        read_notify: Arc<Notify>,
        value: Arc<InteriorMutability<T>>,
    }
    unsafe impl<T> Send for RwLock<T> where T: ?Sized + Send {}
    unsafe impl<T> Sync for RwLock<T> where T: ?Sized + Send + Sync {}

    impl<T: ?Sized> RwLock<T> {
        pub fn new(value: T) -> Self
        where
            T: Sized,
        {
            Self {
                writers: Arc::new(InteriorMutability::new(0)),
                write_notify: Arc::new(Notify::new()),
                readers: Arc::new(InteriorMutability::new(0)),
                read_notify: Arc::new(Notify::new()),
                value: Arc::new(InteriorMutability::new(value)),
            }
        }

        pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
            if *self.writers.as_ref().as_ref() == 0 && *self.readers.as_ref().as_ref() == 0 {
                *self.writers.as_mut() += 1;
            } else {
                *self.writers.as_mut() += 1;
                self.write_notify.notified().await;
            }
            RwLockWriteGuard { lock: &self }
        }

        pub async fn read(&self) -> RwLockReadGuard<'_, T> {
            if *self.writers.as_ref().as_ref() == 0 {
                *self.readers.as_mut() += 1;
            } else {
                *self.readers.as_mut() += 1;
                self.read_notify.notified().await;
            }
            RwLockReadGuard { lock: &self }
        }
    }

    pub struct RwLockWriteGuard<'a, T: ?Sized> {
        lock: &'a RwLock<T>,
    }

    impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.lock.value.as_ref().as_ref()
        }
    }

    impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.lock.value.as_mut()
        }
    }

    impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
        fn drop(&mut self) {
            *self.lock.writers.as_mut() -= 1;
            if *self.lock.writers.as_ref().as_ref() > 0 {
                self.lock.write_notify.notify_one();
            } else if *self.lock.readers.as_ref().as_ref() > 0 {
                self.lock.read_notify.notify_waiters();
            }
        }
    }

    pub struct RwLockReadGuard<'a, T: ?Sized> {
        lock: &'a RwLock<T>,
    }

    impl<'a, T> Deref for RwLockReadGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.lock.value.as_ref().as_ref()
        }
    }

    impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
        fn drop(&mut self) {
            *self.lock.readers.as_mut() -= 1;
            if *self.lock.readers.as_ref().as_ref() == 0 && *self.lock.writers.as_ref().as_ref() > 0
            {
                self.lock.write_notify.notify_one();
            }
        }
    }

    pub mod oneshot {
        use std::{
            future::Future,
            mem::replace,
            pin::Pin,
            sync::Arc,
            task::{Context, Poll},
        };

        use js_sys::{Function, Promise};
        use wasm_bindgen::JsValue;
        use wasm_bindgen_futures::JsFuture;

        use super::InteriorMutability;

        pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
            let value = Arc::new(InteriorMutability::new(Err(RecvError(()))));
            let hungup = Arc::new(InteriorMutability::new(false));
            let mut resolve_fn = None;
            let future = JsFuture::from(Promise::new(&mut |resolve, _reject| {
                resolve_fn = Some(resolve);
            }));
            (
                Sender {
                    value: value.clone(),
                    hungup: hungup.clone(),
                    resolve: resolve_fn.unwrap(),
                },
                Receiver {
                    value,
                    hungup,
                    future,
                },
            )
        }

        pub struct Sender<T> {
            value: Arc<InteriorMutability<Result<T, RecvError>>>,
            hungup: Arc<InteriorMutability<bool>>,
            resolve: Function,
        }

        impl<T> Sender<T> {
            pub fn send(self, value: T) -> Result<(), T> {
                if *self.hungup.as_mut() {
                    Err(value)
                } else {
                    *self.value.as_mut() = Ok(value);
                    self.resolve.call0(&JsValue::null()).unwrap();
                    Ok(())
                }
            }
        }

        impl<T> Drop for Sender<T> {
            fn drop(&mut self) {
                *self.hungup.as_mut() = true;
                self.resolve.call0(&JsValue::null()).unwrap();
            }
        }

        #[derive(Debug, Eq, PartialEq, Clone)]
        pub struct RecvError(());

        pub struct Receiver<T> {
            value: Arc<InteriorMutability<Result<T, RecvError>>>,
            hungup: Arc<InteriorMutability<bool>>,
            future: JsFuture,
        }

        impl<T> Future for Receiver<T> {
            type Output = Result<T, RecvError>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                Future::poll(Pin::new(&mut self.future), cx)
                    .map(|_| replace(self.value.as_mut(), Err(RecvError(()))))
            }
        }

        impl<T> Drop for Receiver<T> {
            fn drop(&mut self) {
                *self.hungup.as_mut() = true;
            }
        }
    }

    pub mod mpsc {
        use std::{collections::VecDeque, sync::Arc};

        use super::{InteriorMutability, Notify};

        pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
            let mut data = VecDeque::new();
            data.reserve_exact(buffer);
            let data = Arc::new(InteriorMutability::new(data));
            let hungup = Arc::new(InteriorMutability::new(false));
            let notify = Arc::new(Notify::new());
            (
                Sender {
                    reference_count: Arc::new(InteriorMutability::new(1)),
                    data: data.clone(),
                    hungup: hungup.clone(),
                    notify: notify.clone(),
                },
                Receiver {
                    data,
                    hungup,
                    notify,
                },
            )
        }

        pub struct Sender<T> {
            reference_count: Arc<InteriorMutability<usize>>,
            data: Arc<InteriorMutability<VecDeque<T>>>,
            hungup: Arc<InteriorMutability<bool>>,
            notify: Arc<Notify>,
        }

        impl<T> std::fmt::Debug for Sender<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("Sender").finish_non_exhaustive()
            }
        }

        impl<T> Clone for Sender<T> {
            fn clone(&self) -> Self {
                *self.reference_count.as_mut() += 1;
                Self {
                    reference_count: self.reference_count.clone(),
                    data: self.data.clone(),
                    hungup: self.hungup.clone(),
                    notify: self.notify.clone(),
                }
            }
        }

        impl<T> Sender<T> {
            pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
                self.blocking_send(value)
            }

            pub fn blocking_send(&self, value: T) -> Result<(), SendError<T>> {
                if *self.hungup.as_ref().as_ref() {
                    Err(SendError(value))
                } else {
                    self.data.as_mut().push_back(value);
                    self.notify.notify_waiters();
                    Ok(())
                }
            }
        }

        impl<T> Drop for Sender<T> {
            fn drop(&mut self) {
                *self.reference_count.as_mut() -= 1;
                if *self.reference_count.as_ref().as_ref() == 0 {
                    *self.hungup.as_mut() = true;
                    self.notify.notify_waiters();
                }
            }
        }

        #[derive(PartialEq, Eq, Clone, Copy)]
        pub struct SendError<T>(pub T);

        impl<T> std::fmt::Debug for SendError<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("SendError").finish_non_exhaustive()
            }
        }

        impl<T> std::fmt::Display for SendError<T> {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(fmt, "channel closed")
            }
        }

        impl<T> std::error::Error for SendError<T> {}

        #[derive(PartialEq, Eq, Clone, Copy, Debug)]
        pub enum TryRecvError {
            Empty,
            Disconnected,
        }

        impl std::fmt::Display for TryRecvError {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match *self {
                    TryRecvError::Empty => "receiving on an empty channel".fmt(fmt),
                    TryRecvError::Disconnected => "receiving on a closed channel".fmt(fmt),
                }
            }
        }

        impl std::error::Error for TryRecvError {}

        pub struct Receiver<T> {
            data: Arc<InteriorMutability<VecDeque<T>>>,
            hungup: Arc<InteriorMutability<bool>>,
            notify: Arc<Notify>,
        }

        impl<T> std::fmt::Debug for Receiver<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("Receiver").finish_non_exhaustive()
            }
        }

        impl<T> Unpin for Receiver<T> {}

        impl<T> Receiver<T> {
            pub async fn recv(&mut self) -> Option<T> {
                loop {
                    if let Some(value) = self.data.as_mut().pop_front() {
                        return Some(value);
                    } else if *self.hungup.as_ref().as_ref() {
                        return None;
                    }
                    self.notify.notified().await;
                }
            }

            pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
                if let Some(value) = self.data.as_mut().pop_front() {
                    return Ok(value);
                } else if *self.hungup.as_ref().as_ref() {
                    return Err(TryRecvError::Disconnected);
                } else {
                    return Err(TryRecvError::Empty);
                }
            }
        }

        impl<T> Drop for Receiver<T> {
            fn drop(&mut self) {
                *self.hungup.as_mut() = true;
            }
        }
    }
}
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
