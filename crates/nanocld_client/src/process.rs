use ntex::channel::mpsc::Receiver;

use nanocl_error::http::HttpResult;
use nanocl_error::http_client::HttpClientResult;

use nanocl_stubs::generic::GenericNspQuery;
use nanocl_stubs::process::{ProcessLogQuery, ProcessOutputLog};

use super::NanocldClient;

impl NanocldClient {
  const PROCESS_PATH: &'static str = "/processes";

  /// Get logs of a process
  pub async fn logs_process(
    &self,
    kind: &str,
    name: &str,
    query: Option<&ProcessLogQuery>,
  ) -> HttpClientResult<Receiver<HttpResult<ProcessOutputLog>>> {
    let res = self
      .send_get(&format!("{}/{kind}/{name}/logs", Self::PROCESS_PATH), query)
      .await?;
    Ok(Self::res_stream(res).await)
  }

  /// Start a process by it's kind and name
  ///
  /// ## Example
  ///
  /// ```no_run,ignore
  /// use nanocld_client::NanocldClient;
  ///
  /// let client = NanocldClient::connect_to("http://localhost:8585", None);
  /// let res = client.start_process("cargo", "my-cargo", None).await;
  /// ```
  pub async fn start_process(
    &self,
    kind: &str,
    name: &str,
    namespace: Option<&str>,
  ) -> HttpClientResult<()> {
    self
      .send_post(
        &format!("{}/{kind}/{name}/start", Self::PROCESS_PATH),
        None::<String>,
        Some(GenericNspQuery::new(namespace)),
      )
      .await?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use futures::StreamExt;

  #[ntex::test]
  async fn logs_cargo() {
    let client =
      NanocldClient::connect_to("http://ndaemon.nanocl.internal:8585", None);
    let mut rx = client
      .logs_process(
        "cargo",
        "nstore",
        Some(&ProcessLogQuery::of_namespace("system")),
      )
      .await
      .unwrap();
    let _out = rx.next().await.unwrap().unwrap();
  }
}
