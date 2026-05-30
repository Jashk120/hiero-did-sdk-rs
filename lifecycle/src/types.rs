use hiero_did_core::DIDError;
use std::future::Future;
use std::pin::Pin;

pub type LifecycleFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub(crate) type CallbackStep<M, C> = Box<
    dyn for<'a> Fn(&'a mut M, &'a mut C) -> LifecycleFuture<'a, Result<(), DIDError>> + Send + Sync,
>;

pub(crate) type Hook<M> =
    Box<dyn for<'a> Fn(&'a M) -> LifecycleFuture<'a, Result<(), DIDError>> + Send + Sync>;

pub(crate) type CatchStep =
    Box<dyn Fn(DIDError) -> LifecycleFuture<'static, Result<(), DIDError>> + Send + Sync>;
