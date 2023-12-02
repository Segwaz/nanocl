use bollard_next::container::{
  StartContainerOptions, Config, CreateContainerOptions,
  InspectContainerOptions,
};

use nanocl_error::{
  http::{HttpResult, HttpError},
  io::FromIo,
};

use crate::models::{
  DaemonState, Repository, ProcessDb, JobDb, JobUpdateDb, ProcessPartial,
  Process, ProcessKind,
};

pub(crate) async fn create(
  name: &str,
  kind: &str,
  kind_key: &str,
  item: Config,
  state: &DaemonState,
) -> HttpResult<Process> {
  let kind: ProcessKind = kind.to_owned().try_into()?;
  let mut config = item.clone();
  let mut labels = item.labels.to_owned().unwrap_or_default();
  labels.insert("io.nanocl".to_owned(), "enabled".to_owned());
  labels.insert("io.nanocl.kind".to_owned(), kind.to_string());
  config.labels = Some(labels);
  let res = state
    .docker_api
    .create_container(
      Some(CreateContainerOptions {
        name,
        ..Default::default()
      }),
      config,
    )
    .await?;
  let inspect = state
    .docker_api
    .inspect_container(&res.id, None::<InspectContainerOptions>)
    .await?;
  let new_instance = ProcessPartial {
    key: res.id,
    name: name.to_owned(),
    kind,
    data: serde_json::to_value(&inspect)
      .map_err(|err| err.map_err_context(|| "CreateProcess"))?,
    node_key: state.config.hostname.clone(),
    kind_key: kind_key.to_owned(),
  };
  let process = ProcessDb::create(&new_instance, &state.pool).await??;
  Process::try_from(process)
    .map_err(|err| HttpError::internal_server_error(err.to_string()))
}

pub(crate) async fn start_by_kind(
  kind: &ProcessKind,
  kind_key: &str,
  state: &DaemonState,
) -> HttpResult<()> {
  let processes = ProcessDb::find_by_kind_key(kind_key, &state.pool).await?;
  log::debug!("process::start_by_kind: {processes:#?}");
  for process in processes {
    let process_state = process.data.state.unwrap_or_default();
    if process_state.running.unwrap_or_default() {
      return Ok(());
    }
    state
      .docker_api
      .start_container(
        &process.data.id.unwrap_or_default(),
        None::<StartContainerOptions<String>>,
      )
      .await?;
  }
  if kind == &ProcessKind::Job {
    JobDb::update_by_pk(
      kind_key,
      JobUpdateDb {
        updated_at: Some(chrono::Utc::now().naive_utc()),
      },
      &state.pool,
    )
    .await??;
  }
  Ok(())
}
