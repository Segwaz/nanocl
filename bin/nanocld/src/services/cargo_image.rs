/*
* Endpoints to manipulate cargo images
*/
use std::io::Write;

use ntex::{web, rt};
use ntex::http::StatusCode;
use futures::StreamExt;
use tokio_util::codec;
use bollard::image::ImportImageOptions;

use nanocl_stubs::cargo_image::{
  CargoImagePartial, ListCargoImagesOptions, CargoImageImportOptions,
  CargoImageImportContext, CargoImageImportInfo,
};

use crate::utils;
use crate::error::HttpResponseError;

/// Endpoint to list installed cargo images
#[cfg_attr(feature = "dev", utoipa::path(
  get,
  path = "/cargoes/images",
  responses(
    (status = 200, description = "List of installed images", body = [ImageSummary]),
    (status = 400, description = "Generic database error", body = ApiError),
  ),
))]
#[web::get("/cargoes/images")]
async fn list_cargo_image(
  docker_api: web::types::State<bollard::Docker>,
  web::types::Query(query): web::types::Query<ListCargoImagesOptions>,
) -> Result<web::HttpResponse, HttpResponseError> {
  let images = utils::cargo_image::list(&docker_api, query.into()).await?;

  Ok(web::HttpResponse::Ok().json(&images))
}

/// Endpoint to inspect an existing cargo image
#[cfg_attr(feature = "dev", utoipa::path(
  get,
  path = "/cargoes/images/{id_or_name}/inspect",
  params(
    ("id_or_name" = String, Path, description = "id or name of the image"),
  ),
  responses(
    (status = 200, description = "Advanced information about a given cargo image", body = ImageInspect),
    (status = 400, description = "Generic database error", body = ApiError),
    (status = 404, description = "Image id or name not valid", body = ApiError),
  ),
))]
#[web::get("/cargoes/images/{id_or_name}*")]
async fn inspect_cargo_image(
  name: web::types::Path<String>,
  docker_api: web::types::State<bollard::Docker>,
) -> Result<web::HttpResponse, HttpResponseError> {
  println!("Inspect image : {name}");
  let image =
    utils::cargo_image::inspect(&name.into_inner(), &docker_api).await?;

  Ok(web::HttpResponse::Ok().json(&image))
}

/// Endpoint to download a cargo image
#[cfg_attr(feature = "dev", utoipa::path(
  post,
  path = "/cargoes/images",
  request_body = CargoImagePartial,
  responses(
    (status = 200, description = "Stream to give information about the download status", content_type = "nanocl/streaming-v1", body = String),
    (status = 400, description = "Generic database error", body = ApiError),
    (status = 404, description = "Image name or label is not valid", body = ApiError),
  ),
))]
#[web::post("/cargoes/images")]
async fn create_cargo_image(
  docker_api: web::types::State<bollard::Docker>,
  web::types::Json(payload): web::types::Json<CargoImagePartial>,
) -> Result<web::HttpResponse, HttpResponseError> {
  println!("Pulling cargo image {}", &payload.name);
  let (from_image, tag) = utils::cargo_image::parse_image_info(&payload.name)?;
  let rx_body =
    utils::cargo_image::download(&from_image, &tag, &docker_api).await?;
  Ok(
    web::HttpResponse::Ok()
      .keep_alive()
      .content_type("nanocl/streaming-v1")
      .streaming(rx_body),
  )
}

/// Endpoint to delete a cargo image
#[cfg_attr(feature = "dev", utoipa::path(
  delete,
  path = "/cargoes/images/{id_or_name}",
  params(
    ("id_or_name" = String, Path, description = "id or name of the image"),
  ),
  responses(
    (status = 200, description = "Generic delete response", body = GenericDelete),
    (status = 400, description = "Generic database error", body = ApiError),
    (status = 404, description = "Image name or id is not valid", body = ApiError),
  ),
))]
#[web::delete("/cargoes/images/{id_or_name}*")]
async fn delete_cargo_image_by_name(
  docker_api: web::types::State<bollard::Docker>,
  id_or_name: web::types::Path<String>,
) -> Result<web::HttpResponse, HttpResponseError> {
  let id_or_name = id_or_name.into_inner();
  let res = utils::cargo_image::delete(&id_or_name, &docker_api).await?;
  Ok(web::HttpResponse::Ok().json(&res))
}

/// Endpoint to import a cargo image
#[cfg_attr(feature = "dev", utoipa::path(
  post,
  path = "/cargoes/images/import",
  descriptions(
    (status = 200, description = "Image import successful", body = GenericImport),
    (status = 400, description = "Invalid image data", body = ApiError),
    (status = 404, description = "Image data not found", body = ApiError),
  ),
))]
#[web::post("/cargoes/images/import")]
async fn import_images(
  docker_api: web::types::State<bollard::Docker>,
  mut payload: web::types::Payload,
  web::types::Query(query): web::types::Query<CargoImageImportOptions>,
) -> Result<web::HttpResponse, HttpResponseError> {
  // generate a random filename
  let filename = uuid::Uuid::new_v4().to_string();
  let filepath = format!("/tmp/{filename}");
  let (tx, rx) =
    ntex::channel::mpsc::channel::<Result<ntex::util::Bytes, std::io::Error>>();
  rt::spawn(async move {
    // File::create is blocking operation, use threadpool
    let file_path_ptr = filepath.clone();
    let mut f = web::block(|| std::fs::File::create(file_path_ptr))
      .await
      .map_err(|err| HttpResponseError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Error while creating the file {err}"),
      })?;
    let mut total_size = 0;
    while let Some(bytes) = payload.next().await {
      let bytes = bytes.map_err(|err| HttpResponseError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Error while reading the multipart field {err}"),
      })?;
      // Field in turn is stream of *Bytes* object
      // filesystem operations are blocking, we have to use threadpool
      total_size += f.write(&bytes).map_err(|err| HttpResponseError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Error while writing the file {err}"),
      })?;

      let _ = tx.send(Ok(ntex::util::Bytes::from(
        serde_json::to_string(&CargoImageImportInfo::Context(
          CargoImageImportContext { writed: total_size },
        ))
        .map_err(|err| HttpResponseError {
          status: StatusCode::INTERNAL_SERVER_ERROR,
          msg: format!("Error while serializing the context {err}"),
        })?,
      )));
    }

    let file = tokio::fs::File::open(&filepath).await.map_err(|err| {
      HttpResponseError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Error while opening the file {err}"),
      }
    })?;

    let byte_stream = codec::FramedRead::new(file, codec::BytesCodec::new())
      .map(|r| {
        let bytes = r?.freeze();
        Ok::<_, std::io::Error>(bytes)
      });

    let quiet = query.quiet.unwrap_or(false);
    let body = hyper::Body::wrap_stream(byte_stream);
    let options = ImportImageOptions { quiet };
    let mut stream = docker_api.import_image(options, body, None);
    while let Some(res) = stream.next().await {
      let res = res.map_err(|err| HttpResponseError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Error while importing the image {err}"),
      })?;
      let _ = tx.send(Ok(ntex::util::Bytes::from(
        serde_json::to_string(&CargoImageImportInfo::BuildInfo(res)).map_err(
          |err| HttpResponseError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            msg: format!("Error while serializing the context {err}"),
          },
        )?,
      )));
    }
    web::block(move || std::fs::remove_file(filepath))
      .await
      .map_err(|err| HttpResponseError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Error while removing the file {err}"),
      })?;
    let _ = tx.send(Ok(ntex::util::Bytes::from("Done")));
    Ok::<_, HttpResponseError>(())
  });

  Ok(web::HttpResponse::Ok().streaming(rx))
}

pub fn ntex_config(config: &mut web::ServiceConfig) {
  config.service(list_cargo_image);
  config.service(create_cargo_image);
  config.service(delete_cargo_image_by_name);
  config.service(inspect_cargo_image);
  config.service(import_images);
}

/// Cargo image unit tests
#[cfg(test)]
pub mod tests {
  use super::*;

  use ntex::http::StatusCode;
  use bollard::service::ImageInspect;
  use futures::{StreamExt, TryStreamExt};

  use nanocl_stubs::generic::GenericDelete;

  use crate::utils::tests::*;

  /// Test utils to list cargo images
  pub async fn list(srv: &TestServer) -> TestReqRet {
    srv.get("/cargoes/images").send().await
  }

  /// Test utils to create cargo image
  pub async fn create(
    srv: &TestServer,
    payload: &CargoImagePartial,
  ) -> TestReqRet {
    srv.post("/cargoes/images").send_json(payload).await
  }

  /// Test utils to inspect cargo image
  pub async fn inspect(srv: &TestServer, id_or_name: &str) -> TestReqRet {
    srv
      .get(format!("/cargoes/images/{id_or_name}"))
      .send()
      .await
  }

  /// Test utils to delete cargo image
  pub async fn delete(srv: &TestServer, id_or_name: &str) -> TestReqRet {
    srv
      .delete(format!("/cargoes/images/{id_or_name}"))
      .send()
      .await
  }

  /// Test utils to ensure the cargo image exists
  pub async fn ensure_test_image() -> TestRet {
    let srv = generate_server(ntex_config).await;
    let image = CargoImagePartial {
      name: "nexthat/nanocl-get-started:latest".to_owned(),
    };
    let res = create(&srv, &image).await?;
    let mut stream = res.into_stream();
    while let Some(chunk) = stream.next().await {
      if let Err(err) = chunk {
        panic!("Error while creating image {}", &err);
      }
    }
    Ok(())
  }

  /// Basic test to list cargo images
  #[ntex::test]
  pub async fn basic_list() -> TestRet {
    let srv = generate_server(ntex_config).await;

    let resp = list(&srv).await?;
    let status = resp.status();
    assert_eq!(
      status,
      StatusCode::OK,
      "Expect basic to return status {} got {}",
      StatusCode::OK,
      status
    );

    Ok(())
  }

  /// Test to upload a cargo image as tarball
  #[ntex::test]
  pub async fn upload_tarball() -> TestRet {
    let srv = generate_server(ntex_config).await;

    let curr_path = std::env::current_dir().unwrap();
    let filepath =
      std::path::Path::new(&curr_path).join("../../tests/busybox.tar.gz");

    let file = tokio::fs::File::open(&filepath).await.map_err(|err| {
      HttpResponseError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        msg: format!("Error while opening the file {err}"),
      }
    })?;

    let byte_stream = codec::FramedRead::new(file, codec::BytesCodec::new())
      .map(|r| {
        let bytes = ntex::util::Bytes::from(r?.freeze().to_vec());
        Ok::<_, std::io::Error>(bytes)
      });

    srv
      .post("/cargoes/images/import")
      .send_stream(byte_stream)
      .await?;

    Ok(())
  }

  /// Basic test to create cargo image with wrong name
  #[ntex::test]
  pub async fn basic_create_wrong_name() -> TestRet {
    let srv = generate_server(ntex_config).await;

    let payload = CargoImagePartial {
      name: "test".to_string(),
    };
    let resp = create(&srv, &payload).await?;
    let status = resp.status();
    assert_eq!(
      status,
      StatusCode::BAD_REQUEST,
      "Expect basic to return status {} got {}",
      StatusCode::BAD_REQUEST,
      status
    );

    Ok(())
  }

  /// Basic test to create, inspect and delete a cargo image
  #[ntex::test]
  async fn crud() -> TestRet {
    const TEST_IMAGE: &str = "busybox:unstable-musl";
    let srv = generate_server(ntex_config).await;

    // Create
    let payload = CargoImagePartial {
      name: TEST_IMAGE.to_owned(),
    };
    let res = create(&srv, &payload).await?;
    let status = res.status();
    assert_eq!(
      status,
      StatusCode::OK,
      "Expect create to return status {} got {}",
      StatusCode::OK,
      status
    );
    let content_type = res
      .header("content-type")
      .expect("Expect create response to have content type header")
      .to_str()
      .unwrap();
    assert_eq!(
      content_type, "nanocl/streaming-v1",
      "Expect content type header to be nanocl/streaming-v1 got {content_type}"
    );
    let mut stream = res.into_stream();
    while let Some(chunk) = stream.next().await {
      if let Err(err) = chunk {
        panic!("Error while creating image {}", &err);
      }
    }

    // Inspect
    let mut res = inspect(&srv, TEST_IMAGE).await?;
    let status = res.status();
    assert_eq!(
      status,
      StatusCode::OK,
      "Expect inspect to return status {} got {}",
      StatusCode::OK,
      status
    );
    let _body: ImageInspect = res
      .json()
      .await
      .expect("Expect inspect to return ImageInspect json data");

    // Delete
    let mut res = delete(&srv, TEST_IMAGE).await?;
    let status = res.status();
    assert_eq!(
      status,
      StatusCode::OK,
      "Expect delete to return status {} got {}",
      StatusCode::OK,
      status
    );
    let body: GenericDelete = res
      .json()
      .await
      .expect("Expect delete to return GenericDelete json data");
    assert_eq!(
      body.count, 1,
      "Expect delete to return count 1 got {}",
      body.count
    );

    Ok(())
  }
}
