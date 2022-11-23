use axum::{
    body::{Bytes, StreamBody},
    extract::{Multipart, Path},
    response::TypedHeader,
    routing::{delete, get, put},
    Extension, Json, Router,
};
use axum_auth::AuthBearer;
use headers::ContentType;
use log::debug;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use walkdir::WalkDir;

use crate::{
    auth::user::UserAction,
    traits::{Error, ErrorInner},
    util::{list_dir, scoped_join_win_safe},
    AppState,
};

// list of protected file extension that cannot be modified
static PROTECTED_EXTENSIONS: [&str; 10] = [
    "jar",
    "lua",
    "sh",
    "exe",
    "bat",
    "cmd",
    "msi",
    "lodestone_config",
    "out",
    "inf",
];

fn is_file_protected(path: impl AsRef<std::path::Path>) -> bool {
    let path = path.as_ref();
    if let Some(ext) = path.extension() {
        PROTECTED_EXTENSIONS.contains(&ext.to_str().unwrap())
    } else {
        true
    }
}

use super::{global_fs::File, util::try_auth};

async fn list_instance_files(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    AuthBearer(token): AuthBearer,
) -> Result<Json<Vec<File>>, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::ReadInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to access instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path = scoped_join_win_safe(&root, relative_path)?;
    if !path.exists() {
        return Err(Error {
            inner: ErrorInner::FileOrDirNotFound,
            detail: "File not found".to_string(),
        });
    }
    if !path.is_dir() {
        return Err(Error {
            inner: ErrorInner::FileOrDirNotFound,
            detail: "Path is not a directory".to_string(),
        });
    }
    Ok(Json(
        list_dir(&path, None)
            .await?
            .iter()
            .map(move |p| {
                // remove the root path from the file path
                let mut r: File = p.as_path().into();
                r.path = p.strip_prefix(&root).unwrap().to_str().unwrap().to_string();
                r
            })
            .collect(),
    ))
}

async fn read_instance_file(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    AuthBearer(token): AuthBearer,
) -> Result<String, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::ReadInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to access instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path = scoped_join_win_safe(root, relative_path)?;
    if !path.exists() || !path.is_file() {
        return Err(Error {
            inner: ErrorInner::MalformedRequest,
            detail: "Path is not a file".to_string(),
        });
    }
    tokio::fs::read_to_string(path).await.map_err(|_| Error {
        inner: ErrorInner::MalformedFile,
        detail: "Only text file encoded in UTF-8 is supported.".to_string(),
    })
}

async fn write_instance_file(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    body: Bytes,
    AuthBearer(token): AuthBearer,
) -> Result<Json<()>, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::WriteInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to access instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path = scoped_join_win_safe(root, relative_path)?;
    // if target has a protected extension, or no extension, deny
    if !requester.can_perform_action(&UserAction::WriteGlobalFile) && is_file_protected(&path) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: format!(
                "File extension {} is protected",
                path.extension()
                    .map(|s| s.to_str().unwrap())
                    .unwrap_or("none")
            ),
        });
    }
    // create the file if it doesn't exist
    tokio::fs::write(path, body).await.map_err(|_| Error {
        inner: ErrorInner::MalformedRequest,
        detail: "Failed to write file".to_string(),
    })?;
    Ok(Json(()))
}

async fn make_instance_directory(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    AuthBearer(token): AuthBearer,
) -> Result<Json<()>, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::WriteInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to access instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path = scoped_join_win_safe(root, relative_path)?;
    // create the file if it doesn't exist
    tokio::fs::create_dir_all(path).await.map_err(|_| Error {
        inner: ErrorInner::MalformedRequest,
        detail: "Failed to create directory".to_string(),
    })?;
    Ok(Json(()))
}

async fn remove_instance_file(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    AuthBearer(token): AuthBearer,
) -> Result<Json<()>, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::WriteInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to access instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path = scoped_join_win_safe(root, relative_path)?;
    // if target has a protected extension, or no extension, deny
    if !requester.can_perform_action(&UserAction::WriteGlobalFile) && is_file_protected(&path) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: format!(
                "File extension {} is protected",
                path.extension()
                    .map(|s| s.to_str().unwrap())
                    .unwrap_or("none")
            ),
        });
    }
    if !path.exists() {
        return Err(Error {
            inner: ErrorInner::FileOrDirNotFound,
            detail: "Path does not exist".to_string(),
        });
    }
    if path.is_file() {
        tokio::fs::remove_file(path).await.map_err(|_| Error {
            inner: ErrorInner::MalformedRequest,
            detail: "Failed to remove file".to_string(),
        })?;
    } else {
        return Err(Error {
            inner: ErrorInner::MalformedRequest,
            detail: "Path is not a file".to_string(),
        });
    }
    Ok(Json(()))
}

async fn remove_instance_dir(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    AuthBearer(token): AuthBearer,
) -> Result<Json<()>, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::WriteInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to access instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path = scoped_join_win_safe(root, relative_path)?;
    // if target has a protected extension, or no extension, deny
    if !requester.can_perform_action(&UserAction::WriteGlobalFile) && is_file_protected(&path) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: format!(
                "File extension {} is protected",
                path.extension()
                    .map(|s| s.to_str().unwrap())
                    .unwrap_or("none")
            ),
        });
    }
    if !path.exists() {
        return Err(Error {
            inner: ErrorInner::FileOrDirNotFound,
            detail: "Path does not exist".to_string(),
        });
    }
    if path.is_dir() {
        if requester.can_perform_action(&UserAction::WriteGlobalFile) {
            tokio::fs::remove_dir_all(path).await.map_err(|_| Error {
                inner: ErrorInner::MalformedRequest,
                detail: "Failed to remove directory".to_string(),
            })?;
        } else {
            // recursively access all files in the directory and check if they are protected
            for entry in WalkDir::new(path.clone()) {
                let entry = entry.map_err(|_| Error {
                    inner: ErrorInner::MalformedRequest,
                    detail: "Failed to read directory while scanning for protected files"
                        .to_string(),
                })?;
                if entry.file_type().is_file() {
                    if is_file_protected(&entry.path()) {
                        return Err(Error {
                            inner: ErrorInner::PermissionDenied,
                            detail: format!(
                                "File extension {} is protected",
                                entry
                                    .path()
                                    .extension()
                                    .map(|s| s.to_str().unwrap())
                                    .unwrap_or("none")
                            ),
                        });
                    }
                }
            }
            tokio::fs::remove_dir_all(path).await.map_err(|_| Error {
                inner: ErrorInner::MalformedRequest,
                detail: "Failed to remove directory".to_string(),
            })?;
        }
    } else {
        return Err(Error {
            inner: ErrorInner::MalformedRequest,
            detail: "Path is not a directory".to_string(),
        });
    }
    Ok(Json(()))
}

async fn new_instance_file(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    AuthBearer(token): AuthBearer,
) -> Result<Json<()>, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::WriteInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to access instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path = scoped_join_win_safe(root, relative_path)?;
    // if target has a protected extension, or no extension, deny
    if !requester.can_perform_action(&UserAction::WriteGlobalFile) && is_file_protected(&path) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: format!(
                "File extension {} is protected",
                path.extension()
                    .map(|s| s.to_str().unwrap())
                    .unwrap_or("none")
            ),
        });
    }
    if path.exists() {
        return Err(Error {
            inner: ErrorInner::FiledOrDirAlreadyExists,
            detail: "Path already exists".to_string(),
        });
    }
    tokio::fs::File::create(path).await.map_err(|_| Error {
        inner: ErrorInner::MalformedRequest,
        detail: "Failed to create file".to_string(),
    })?;
    Ok(Json(()))
}

async fn download_instance_file(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    AuthBearer(token): AuthBearer,
) -> Result<
    (
        TypedHeader<ContentType>,
        StreamBody<ReaderStream<tokio::fs::File>>,
    ),
    Error,
> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::ReadInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to access instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path = scoped_join_win_safe(&root, relative_path)?;
    if !path.exists() {
        return Err(Error {
            inner: ErrorInner::FileOrDirNotFound,
            detail: "File not found".to_string(),
        });
    }
    if !path.is_file() {
        return Err(Error {
            inner: ErrorInner::FileOrDirNotFound,
            detail: "Path is not a file".to_string(),
        });
    }
    let file = tokio::fs::File::open(&path).await.map_err(|_| Error {
        inner: ErrorInner::MalformedRequest,
        detail: "Failed to open file".to_string(),
    })?;
    let content_type = match path.extension() {
        Some(extension) => match extension.to_str().unwrap() {
            "html" => ContentType::html(),
            "json" => ContentType::json(),
            "txt" => ContentType::text_utf8(),
            "png" => ContentType::png(),
            "jpg" => ContentType::jpeg(),
            "jpeg" => ContentType::jpeg(),
            _ => ContentType::octet_stream(),
        },
        None => ContentType::octet_stream(),
    };

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);
    Ok((TypedHeader(content_type), body))
}

async fn upload_instance_file(
    Extension(state): Extension<AppState>,
    Path((uuid, relative_path)): Path<(String, String)>,
    AuthBearer(token): AuthBearer,
    mut multipart: Multipart,
) -> Result<Json<()>, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::Unauthorized,
        detail: "Token error".to_string(),
    })?;
    if !requester.can_perform_action(&UserAction::WriteInstanceFile(uuid.clone())) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to write instance files".to_string(),
        });
    }
    drop(users);
    let instances = state.instances.lock().await;
    let instance = instances
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    let root = instance.path().await;
    drop(instance);
    drop(instances);
    let path_to_dir = scoped_join_win_safe(&root, relative_path)?;
    if path_to_dir.exists() && !path_to_dir.is_dir() {
        return Err(Error {
            inner: ErrorInner::MalformedRequest,
            detail: "Path is not a directory".to_string(),
        });
    }
    if !path_to_dir.exists() {
        tokio::fs::create_dir_all(&path_to_dir)
            .await
            .map_err(|_| Error {
                inner: ErrorInner::FailedToCreateFileOrDir,
                detail: "Failed to create directory".to_string(),
            })?;
    }

    while let Ok(Some(mut field)) = multipart.next_field().await {
        let name = field.file_name().ok_or_else(|| Error {
            inner: ErrorInner::MalformedRequest,
            detail: "No file name".to_string(),
        })?;
        let name = sanitize_filename::sanitize(&name);
        let path = scoped_join_win_safe(&root, &name)?;
        // if the file has a protected extension, or no extension, deny
        if !requester.can_perform_action(&UserAction::WriteGlobalFile) && is_file_protected(&path) {
            return Err(Error {
                inner: ErrorInner::PermissionDenied,
                detail: format!(
                    "File extension {} is protected",
                    path.extension()
                        .map(|s| s.to_str().unwrap())
                        .unwrap_or("none")
                ),
            });
        }

        // if the file is more than 2GB, deny

        // if let Some(size_hint) = field.size_hint().1 {
        //     if size_hint > 2_000_000_000 {
        //         return Err(Error {
        //             inner: ErrorInner::MalformedRequest,
        //             detail: "File too large".to_string(),
        //         });
        //     }
        // } else {
        //     return Err(Error {
        //         inner: ErrorInner::MalformedRequest,
        //         detail: "Cannot determine the file size".to_string(),
        //     });
        // }

        let path = if path.exists() {
            // add a postfix to the file name
            let mut postfix = 1;
            // get the file name without the extension
            let file_name = path.file_stem().unwrap().to_str().unwrap().to_string();
            loop {
                let new_path = path.with_file_name(format!(
                    "{}_{}.{}",
                    file_name,
                    postfix,
                    path.extension().unwrap().to_str().unwrap()
                ));
                if !new_path.exists() {
                    break new_path;
                }
                postfix += 1;
            }
        } else {
            path
        };
        let mut file = tokio::fs::File::create(&path).await.map_err(|_| Error {
            inner: ErrorInner::FailedToCreateFileOrDir,
            detail: "Failed to create file".to_string(),
        })?;
        while let Some(chunk) = field.chunk().await.map_err(|_| {
            std::fs::remove_file(&path).ok();
            Error {
                inner: ErrorInner::MalformedRequest,
                detail: "Failed to read chunk".to_string(),
            }
        })? {
            debug!("Received chunk of size {}", chunk.len());
            file.write_all(&chunk).await.map_err(|_| {
                std::fs::remove_file(&path).ok();
                Error {
                    inner: ErrorInner::FailedToCreateFileOrDir,
                    detail: "Failed to write to file".to_string(),
                }
            })?;
        }
    }
    Ok(Json(()))
}

pub fn get_instance_fs_routes() -> Router {
    Router::new()
        .route(
            "/instance/:uuid/fs/ls/*relative_path",
            get(list_instance_files),
        )
        .route(
            "/instance/:uuid/fs/read/*relative_path",
            get(read_instance_file),
        )
        .route(
            "/instance/:uuid/fs/write/*relative_path",
            put(write_instance_file),
        )
        .route(
            "/instance/:uuid/fs/mkdir/*relative_path",
            put(make_instance_directory),
        )
        .route(
            "/instance/:uuid/fs/rm/*relative_path",
            delete(remove_instance_file),
        )
        .route(
            "/instance/:uuid/fs/rmdir/*relative_path",
            delete(remove_instance_dir),
        )
        .route(
            "/instance/:uuid/fs/new/*relative_path",
            put(new_instance_file),
        )
        .route(
            "/instance/:uuid/fs/download/*relative_path",
            get(download_instance_file),
        )
        .route(
            "/instance/:uuid/fs/upload/*relative_path",
            put(upload_instance_file),
        )
}