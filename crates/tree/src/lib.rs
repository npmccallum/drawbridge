// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

use drawbridge_type::Entry;
use drawbridge_type::Meta;

use axum::async_trait;
use axum::body::HttpBody;
use axum::body::StreamBody;
use axum::extract::{Extension, FromRequest, Path};
use axum::response::Error;
use axum::Router;
use tokio::io::AsyncRead;
use tokio_util::io::ReaderStream;

#[async_trait]
pub trait Storage: Send {
    type Body: 'static + Send + AsyncRead;

    async fn get(&self, path: &[&str]) -> Result<(Meta, Self::Body), Error>;
    async fn put<T>(&self, path: &[&str], meta: Meta, body: T) -> Result<(), Error>
    where
        T: Send + AsyncRead + Unpin;
}

pub fn app<T: Storage, B: HttpBody>() -> Router<B>
where
    Extension<T>: FromRequest<B>,
    T: 'static + Send,
    B: 'static + Send,
{
    use axum::routing::*;

    Router::new()
        .route("/*path", put(|s, e, p, m| self::put::<T>(s, e, p, m)))
        .route("/*path", head(|s, e, p| self::head::<T>(s, e, p)))
        .route("/*path", get(|s, e, p| self::get::<T>(s, e, p)))
}

async fn head<T: Storage>(
    Extension(storage): Extension<T>,
    Extension(entry): Extension<Entry>,
    Path(path): Path<String>,
) -> Result<Meta, Error> {
    let path: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let (meta, ..) = storage.get(&path).await?;
    Ok(meta)
}

async fn put<T: Storage>(
    Extension(storage): Extension<T>,
    Extension(entry): Extension<Entry>,
    Path(path): Path<String>,
    meta: Meta,
) -> Result<(), Error> {
    let path: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    Ok(())
}

async fn get<T: Storage>(
    Extension(storage): Extension<T>,
    Extension(entry): Extension<Entry>,
    Path(path): Path<String>,
) -> Result<(Meta, StreamBody<ReaderStream<T::Body>>), Error> {
    let path: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let (meta, data) = storage.get(&path).await?;
    Ok((meta, StreamBody::new(ReaderStream::new(data))))
}
