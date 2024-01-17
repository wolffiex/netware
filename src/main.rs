#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
use anyhow::Result;

use axum::{
    body::Body,
    extract::Query,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Error as AxumError, Router,
};
use axum_macros::debug_handler;
use futures::stream::select;
use futures::Stream;
use futures::TryStreamExt;
use futures::{stream, StreamExt};
use serde_json::Value as Json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio_postgres::{Client, NoTls};
use tower_http::services::ServeDir;
mod template;
mod sql;

use template::get_page;
use sql::{get_sql_client, stream_sql_response};

#[derive(Clone)]
pub struct AppState {
    client: Arc<Client>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let now = Instant::now(); // get current time

    let res = get_page("/weather".into(), PathBuf::from("project/src/templates"))?;
    let elapsed = now.elapsed(); // get elapsed time
    println!("Compilation took: {:.2?}", elapsed);

    let sql_client = get_sql_client().await;

    let state = AppState {
        client: Arc::new(sql_client),
    };

    let dist_service = ServeDir::new("project/dist");

    // Set up the router and routes
    let app = Router::new()
        .nest_service("/www", ServeDir::new("./project/www"))
        .route("/api", get(stream_sql_response))
        .fallback(get(|| async { Ok::<Html<String>, Infallible>(Html(res)) }))
        .with_state(state);

    // Run the application
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

type JsonStream = Pin<Box<dyn Stream<Item = Result<String, AxumError>> + Send>>;

