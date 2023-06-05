use std::{
    fs,
    path::{self, PathBuf},
    sync::Arc,
};

use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use nanoid::nanoid;
use sqlx::{Executor, PgPool};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
struct AppState {
    static_folder: PathBuf,
    pool: PgPool,
}

async fn hello_world() -> &'static str {
    "Hello, world!"
}

async fn upload(mut multipart: Multipart, state: Arc<AppState>) -> Result<String, StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        let path = state.static_folder.as_path().join(name);

        // let mut f = File::create(static_folder);
        fs::write(path.clone(), data).unwrap();

        let id = nanoid!(6);
        sqlx::query("INSERT INTO files(id, path) VALUES ($1, $2)")
            .bind(id.clone())
            .bind(path.to_str())
            .execute(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok(id);
    }

    Err(StatusCode::BAD_REQUEST)
}

// #[axum_macros::debug_handler]
async fn redirect(Path(id): Path<String>, state: Arc<AppState>) -> Result<ServeFile, StatusCode> {
    let file: (String,) = sqlx::query_as("SELECT path FROM fiels WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| match e {
            sqlx::error::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(ServeFile::new(path::Path::new(&file.0)))
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_static_folder::StaticFolder(folder = "files")] static_folder: PathBuf,
    #[shuttle_shared_db::Postgres] pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    pool.execute(include_str!("../schema.sql")).await.unwrap();

    let state = Arc::new(AppState {
        static_folder,
        pool,
    });

    let router = Router::new()
        .route("/", get(hello_world))
        .nest_service("/files", {
            let state = state.clone();
            ServeDir::new(state.static_folder.to_path_buf()).not_found_service(ServeFile::new(
                format!("{}/screenshot.png", state.static_folder.to_str().unwrap()),
            ))
        })
        .route("/upload", post(move |body| upload(body, state.clone())));
    // TODO: get this route to work: serve a file at baseurl.shuttleapp.rs/:id
    //.route("/:id", get(move |id| redirect(id, state.clone())));

    Ok(router.into())
}
