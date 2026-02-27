//! Connector for the graphql API

use crate::{
    Query,
    connector::{Sink, Source},
    encode::{Codec, Decode, Encode},
    errors::{ConnectionError, DecodeError, FetchError, FetchOneError, SendError},
    query::{HttpQuery, Single},
};
use async_trait::async_trait;
use futures::{StreamExt as _, TryStreamExt as _, future::ready, stream::BoxStream};

mod builder;

/// A connector that works with our specific graphql API.
#[derive(Debug, Clone)]
pub struct ReadWrite<T, E, D, C> {}
