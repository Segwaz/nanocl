use ntex::web;

use nanocl_utils::http_error::HttpError;

use crate::version;

/// Get version information
#[cfg_attr(feature = "dev", utoipa::path(
  head,
  tag = "System",
  path = "/_ping",
  responses(
    (status = 202, description = "Server is up"),
  ),
))]
#[web::head("/_ping")]
pub(crate) async fn head_ping() -> Result<web::HttpResponse, HttpError> {
  Ok(web::HttpResponse::Accepted().into())
}

/// Get version information
#[cfg_attr(feature = "dev", utoipa::path(
  get,
  tag = "System",
  path = "/version",
  responses(
    (status = 200, description = "Version information", body = Version),
  ),
))]
#[web::get("/version")]
pub(crate) async fn get_version() -> web::HttpResponse {
  web::HttpResponse::Ok().json(&serde_json::json!({
    "Arch": version::ARCH,
    "Channel": version::CHANNEL,
    "Version": version::VERSION,
    "CommitId": version::COMMIT_ID,
  }))
}

pub fn ntex_config(config: &mut web::ServiceConfig) {
  config.service(head_ping);
  config.service(get_version);
}

#[cfg(test)]
mod tests {
  use ntex::http;

  use nanocld_client::stubs::system::Version;

  use crate::utils::tests;

  #[ntex::test]
  async fn head_ping() {
    let srv = tests::generate_server();

    let res = srv
      .head("/v0.1/_ping")
      .send()
      .await
      .expect("Failed to execute request");

    assert_eq!(res.status(), http::StatusCode::ACCEPTED);
  }

  #[ntex::test]
  async fn get_version() {
    let srv = tests::generate_server();

    let mut res = srv
      .get("/v0.1/version")
      .send()
      .await
      .expect("Failed to execute request");

    assert_eq!(res.status(), http::StatusCode::OK);
    let _ = res
      .json::<Version>()
      .await
      .expect("Expect to get a valid version");
  }
}
