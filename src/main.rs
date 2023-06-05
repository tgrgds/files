use std::{fs, path::PathBuf, sync::Arc};

use axum::{
    extract::Multipart,
    routing::{get, post},
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

async fn hello_world() -> &'static str {
    "Hello, world!"
}

async fn upload(mut multipart: Multipart, static_folder: Arc<PathBuf>) {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        // let mut f = File::create(static_folder);
        fs::write(static_folder.as_path().join(name), data).unwrap();
    }
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_static_folder::StaticFolder(folder = "files")] static_folder: PathBuf,
) -> shuttle_axum::ShuttleAxum {
    let folder = Arc::new(static_folder);

    let router = Router::new()
        .route("/", get(hello_world))
        .nest_service(
            "/files",
            ServeDir::new(folder.clone().to_path_buf()).not_found_service(ServeFile::new(format!(
                "{}/screenshot.png",
                folder.to_str().unwrap()
            ))),
        )
        .route(
            "/upload",
            post({
                let folder = Arc::clone(&folder);
                move |body| upload(body, folder)
            }),
        );
    // .route_service("/upload", post(|req: Request<Multipart> | async { upload(req, static_folder) }));

    Ok(router.into())
}
