/// Cargo service
/// Endpoints to manage cargoes
use ntex::web;

use nanocl_models::cargo::CargoPartial;

use crate::utils;
use crate::errors::HttpResponseError;
use crate::models::{Pool, GenericNspQuery};

/// Endpoint to create a new cargo
#[cfg_attr(feature = "dev", utoipa::path(
    post,
    request_body = CargoPartial,
    path = "/cargoes",
    params(
      ("namespace" = Option<String>, Query, description = "Name of the namespace where the cargo will be stored"),
    ),
    responses(
      (status = 201, description = "New cargo", body = Cargo),
      (status = 400, description = "Generic database error", body = ApiError),
      (status = 404, description = "Namespace name not valid", body = ApiError),
    ),
  ))]
#[web::post("/cargoes")]
pub async fn create_cargo(
  pool: web::types::State<Pool>,
  docker_api: web::types::State<bollard::Docker>,
  web::types::Query(qs): web::types::Query<GenericNspQuery>,
  web::types::Json(payload): web::types::Json<CargoPartial>,
) -> Result<web::HttpResponse, HttpResponseError> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);

  println!("Creating cargo: {:?}", &payload);

  let cargo =
    utils::cargo::create(payload, namespace, &docker_api, &pool).await?;

  Ok(web::HttpResponse::Created().json(&cargo))
}

/// Endpoint to delete a cargo
#[cfg_attr(feature = "dev", utoipa::path(
    delete,
    path = "/cargoes/{name}",
    params(
      ("name" = String, Path, description = "Name of the cargo to delete"),
      ("namespace" = Option<String>, Query, description = "Name of the namespace where the cargo is stored"),
    ),
    responses(
      (status = 204, description = "Cargo deleted", body = GenericDelete),
      (status = 400, description = "Generic database error", body = ApiError),
      (status = 404, description = "Cargo not found", body = ApiError),
    ),
  ))]
#[web::delete("/cargoes/{name}")]
pub async fn delete_cargo(
  pool: web::types::State<Pool>,
  docker_api: web::types::State<bollard::Docker>,
  id: web::types::Path<String>,
  web::types::Query(qs): web::types::Query<GenericNspQuery>,
) -> Result<web::HttpResponse, HttpResponseError> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &id);

  println!("Deleting cargo: {}", &key);

  utils::cargo::delete(&key, &docker_api, &pool).await?;

  Ok(web::HttpResponse::NoContent().finish())
}

/// Endpoint to start a cargo
#[cfg_attr(feature = "dev", utoipa::path(
    post,
    path = "/cargoes/{name}/start",
    params(
      ("name" = String, Path, description = "Name of the cargo to start"),
      ("namespace" = Option<String>, Query, description = "Name of the namespace where the cargo is stored"),
    ),
    responses(
      (status = 202, description = "Cargo started"),
      (status = 400, description = "Generic database error", body = ApiError),
      (status = 404, description = "Cargo not found", body = ApiError),
    ),
  ))]
#[web::post("/cargoes/{name}/start")]
pub async fn start_cargo(
  docker_api: web::types::State<bollard::Docker>,
  id: web::types::Path<String>,
  web::types::Query(qs): web::types::Query<GenericNspQuery>,
) -> Result<web::HttpResponse, HttpResponseError> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &id);

  println!("Starting cargo: {}", &key);

  utils::cargo::start(&key, &docker_api).await?;

  Ok(web::HttpResponse::Accepted().finish())
}

/// Endpoint to stop a cargo
#[cfg_attr(feature = "dev", utoipa::path(
    post,
    path = "/cargoes/{name}/stop",
    params(
      ("name" = String, Path, description = "Name of the cargo to stop"),
      ("namespace" = Option<String>, Query, description = "Name of the namespace where the cargo is stored"),
    ),
    responses(
      (status = 202, description = "Cargo stopped"),
      (status = 400, description = "Generic database error", body = ApiError),
      (status = 404, description = "Cargo not found", body = ApiError),
    ),
  ))]
#[web::post("/cargoes/{name}/stop")]
pub async fn stop_cargo(
  docker_api: web::types::State<bollard::Docker>,
  id: web::types::Path<String>,
  web::types::Query(qs): web::types::Query<GenericNspQuery>,
) -> Result<web::HttpResponse, HttpResponseError> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &id);

  println!("Stopping cargo: {}", &key);

  utils::cargo::stop(&key, &docker_api).await?;

  Ok(web::HttpResponse::Accepted().finish())
}

/// Endpoint to patch a cargo
#[cfg_attr(feature = "dev", utoipa::path(
    patch,
    path = "/cargoes/{name}",
    request_body = CargoPartial,
    params(
      ("name" = String, Path, description = "Name of the cargo to patch"),
      ("namespace" = Option<String>, Query, description = "Name of the namespace where the cargo is stored"),
    ),
    responses(
      (status = 200, description = "Cargo patched", body = Cargo),
      (status = 400, description = "Generic database error", body = ApiError),
      (status = 404, description = "Cargo not found", body = ApiError),
    ),
  ))]
#[web::patch("/cargoes/{name}")]
pub async fn patch_cargo(
  pool: web::types::State<Pool>,
  docker_api: web::types::State<bollard::Docker>,
  id: web::types::Path<String>,
  web::types::Query(qs): web::types::Query<GenericNspQuery>,
  payload: web::types::Json<CargoPartial>,
) -> Result<web::HttpResponse, HttpResponseError> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &id);

  println!("Patching cargo: {}", &key);

  let cargo =
    utils::cargo::patch(&key, payload.into_inner(), &docker_api, &pool).await?;

  Ok(web::HttpResponse::Ok().json(&cargo))
}

/// Endpoint to list cargo
#[cfg_attr(feature = "dev", utoipa::path(
    get,
    path = "/cargoes",
    params(
      ("namespace" = Option<String>, Query, description = "Name of the namespace where the cargo is stored"),
    ),
    responses(
      (status = 200, description = "Cargo list", body = [CargoSummary]),
      (status = 400, description = "Generic database error", body = ApiError),
    ),
  ))]
#[web::get("/cargoes")]
pub async fn list_cargo(
  pool: web::types::State<Pool>,
  docker_api: web::types::State<bollard::Docker>,
  web::types::Query(qs): web::types::Query<GenericNspQuery>,
) -> Result<web::HttpResponse, HttpResponseError> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);

  println!("Listing cargoes in namespace: {}", &namespace);

  let cargoes = utils::cargo::list(&namespace, &docker_api, &pool).await?;

  Ok(web::HttpResponse::Ok().json(&cargoes))
}

pub fn ntex_config(config: &mut web::ServiceConfig) {
  config.service(create_cargo);
  config.service(delete_cargo);
  config.service(start_cargo);
  config.service(stop_cargo);
  config.service(patch_cargo);
  config.service(list_cargo);
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::services::cargo_image::tests::ensure_test_image;

  use nanocl_models::cargo::{Cargo, CargoPartial, CargoSummary};
  use nanocl_models::cargo_config::CargoConfigPartial;

  use crate::utils::tests::*;

  /// Test to create start patch stop and delete a cargo with valid data
  #[ntex::test]
  async fn test_basic() -> TestRet {
    let srv = generate_server(ntex_config).await;
    ensure_test_image().await?;

    let mut res = srv
      .post("/cargoes")
      .send_json(&CargoPartial {
        name: "test-cargo-cd".to_string(),
        config: CargoConfigPartial {
          container: bollard::container::Config {
            image: Some("nexthat/nanocl-get-started:latest".to_string()),
            ..Default::default()
          },
          ..Default::default()
        },
      })
      .await?;
    assert_eq!(res.status(), 201);

    let response = res.json::<Cargo>().await?;
    assert_eq!(response.name, "test-cargo-cd");
    assert_eq!(response.namespace_name, "global");
    assert_eq!(
      response.config.container.image,
      Some("nexthat/nanocl-get-started:latest".to_string())
    );

    let mut res = srv.get("/cargoes").send().await?;
    assert_eq!(res.status(), 200);
    let cargoes = res.json::<Vec<CargoSummary>>().await?;
    assert_eq!(cargoes.len(), 1);
    assert_eq!(cargoes[0].name, "test-cargo-cd");
    assert_eq!(cargoes[0].namespace_name, "global");
    assert_eq!(cargoes[0].running_instances, 0);

    let res = srv
      .post(format!("/cargoes/{}/start", response.name))
      .send()
      .await?;
    assert_eq!(res.status(), 202);

    let mut res = srv
      .patch(format!("/cargoes/{}", response.name))
      .send_json(&CargoPartial {
        name: "test-cargo-cd".to_string(),
        config: CargoConfigPartial {
          container: bollard::container::Config {
            image: Some("nexthat/nanocl-get-started:latest".to_string()),
            env: Some(vec!["TEST=1".to_string()]),
            ..Default::default()
          },
          ..Default::default()
        },
      })
      .await?;
    assert_eq!(res.status(), 200);

    let patch_response = res.json::<Cargo>().await?;
    assert_eq!(patch_response.name, "test-cargo-cd");
    assert_eq!(patch_response.namespace_name, "global");
    assert_eq!(
      patch_response.config.container.image,
      Some("nexthat/nanocl-get-started:latest".to_string())
    );
    assert_eq!(
      patch_response.config.container.env,
      Some(vec!["TEST=1".to_string()])
    );

    let res = srv
      .post(format!("/cargoes/{}/stop", response.name))
      .send()
      .await?;
    assert_eq!(res.status(), 202);

    let res = srv
      .delete(format!("/cargoes/{}", response.name))
      .send()
      .await?;
    assert_eq!(res.status(), 204);

    Ok(())
  }
}
