use std::{io, rc::Rc, cell::RefCell, time::Instant};

use ntex::{
  rt, ws, web, chain, fn_service, Service,
  util::Bytes,
  channel::{mpsc, oneshot},
  service::{map_config, fn_shutdown, fn_factory_with_config},
};
use futures::{StreamExt, future::ready};
use tokio::io::AsyncWriteExt;

use nanocl_error::http::{HttpError, HttpResult};

use bollard_next::container::AttachContainerOptions;
use nanocl_stubs::{
  process::OutputLog,
  generic::GenericNspQuery,
  vm_spec::{VmSpecPartial, VmSpecUpdate},
};

use crate::{
  utils,
  models::{DaemonState, WsConState, VmSpecDb},
};

/// List virtual machines
#[cfg_attr(feature = "dev", utoipa::path(
  get,
  tag = "Vms",
  path = "/vms",
  params(
    ("namespace" = Option<String>, Query, description = "The namespace of the virtual machine"),
  ),
  responses(
    (status = 200, description = "List of virtual machine", body = [VmSummary]),
  ),
))]
#[web::get("/vms")]
pub(crate) async fn list_vm(
  state: web::types::State<DaemonState>,
  qs: web::types::Query<GenericNspQuery>,
) -> HttpResult<web::HttpResponse> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let vms = utils::vm::list_by_namespace(&namespace, &state.pool).await?;
  Ok(web::HttpResponse::Ok().json(&vms))
}

/// Inspect a virtual machine
#[cfg_attr(feature = "dev", utoipa::path(
  get,
  tag = "Vms",
  path = "/vms/{name}/inspect",
  params(
    ("name" = String, Path, description = "The name of the virtual machine"),
    ("namespace" = Option<String>, Query, description = "The namespace of the virtual machine"),
  ),
  responses(
    (status = 200, description = "Detailed information about a virtual machine", body = VmInspect),
  ),
))]
#[web::get("/vms/{name}/inspect")]
pub(crate) async fn inspect_vm(
  state: web::types::State<DaemonState>,
  path: web::types::Path<(String, String)>,
  qs: web::types::Query<GenericNspQuery>,
) -> HttpResult<web::HttpResponse> {
  let name = path.1.to_owned();
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &name);
  let vm = utils::vm::inspect_by_key(&key, &state).await?;
  Ok(web::HttpResponse::Ok().json(&vm))
}

/// Delete a virtual machine
#[cfg_attr(feature = "dev", utoipa::path(
  delete,
  tag = "Vms",
  path = "/vms/{name}",
  params(
    ("name" = String, Path, description = "The name of the virtual machine"),
    ("namespace" = Option<String>, Query, description = "The namespace of the virtual machine"),
  ),
  responses(
    (status = 200, description = "The virtual machine has been deleted"),
  ),
))]
#[web::delete("/vms/{name}")]
pub(crate) async fn delete_vm(
  state: web::types::State<DaemonState>,
  path: web::types::Path<(String, String)>,
  qs: web::types::Query<GenericNspQuery>,
) -> HttpResult<web::HttpResponse> {
  let name = path.1.to_owned();
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &name);
  utils::vm::delete_by_key(&key, true, &state).await?;
  Ok(web::HttpResponse::Ok().finish())
}

/// Create a virtual machine
#[cfg_attr(feature = "dev", utoipa::path(
  post,
  tag = "Vms",
  path = "/vms",
  request_body = VmSpecPartial,
  params(
    ("namespace" = Option<String>, Query, description = "The namespace of the virtual machine"),
  ),
  responses(
    (status = 200, description = "The virtual machine has been created", body = Vm),
  ),
))]
#[web::post("/vms")]
pub(crate) async fn create_vm(
  state: web::types::State<DaemonState>,
  path: web::types::Path<String>,
  payload: web::types::Json<VmSpecPartial>,
  qs: web::types::Query<GenericNspQuery>,
) -> HttpResult<web::HttpResponse> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let item = utils::vm::create(&payload, &namespace, &path, &state).await?;
  Ok(web::HttpResponse::Ok().json(&item))
}

/// List virtual machine histories
#[cfg_attr(feature = "dev", utoipa::path(
  get,
  tag = "Vms",
  path = "/vms/{name}/histories",
  params(
    ("name" = String, Path, description = "The name of the virtual machine"),
    ("namespace" = Option<String>, Query, description = "The namespace of the virtual machine"),
  ),
  responses(
    (status = 200, description = "The virtual machine histories have been listed", body = [VmSpec]),
  ),
))]
#[web::get("/vms/{name}/histories")]
pub(crate) async fn list_vm_history(
  state: web::types::State<DaemonState>,
  path: web::types::Path<(String, String)>,
  qs: web::types::Query<GenericNspQuery>,
) -> HttpResult<web::HttpResponse> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &path.1);
  let histories = VmSpecDb::find_by_vm(&key, &state.pool).await?;
  Ok(web::HttpResponse::Ok().json(&histories))
}

/// Patch a virtual machine config meaning merging current config with the new one and add history entry
#[cfg_attr(feature = "dev", utoipa::path(
  patch,
  tag = "Vms",
  request_body = VmSpecUpdate,
  path = "/vms/{name}",
  params(
    ("name" = String, Path, description = "Name of the virtual machine"),
    ("namespace" = Option<String>, Query, description = "Namespace of the virtual machine"),
  ),
  responses(
    (status = 200, description = "Updated virtual machine", body = Vm),
    (status = 404, description = "Virtual machine not found", body = ApiError),
  ),
))]
#[web::patch("/vms/{name}")]
pub(crate) async fn patch_vm(
  state: web::types::State<DaemonState>,
  path: web::types::Path<(String, String)>,
  payload: web::types::Json<VmSpecUpdate>,
  qs: web::types::Query<GenericNspQuery>,
) -> HttpResult<web::HttpResponse> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &path.1);
  let version = path.0.clone();
  let vm = utils::vm::patch(&key, &payload, &version, &state).await?;
  Ok(web::HttpResponse::Ok().json(&vm))
}

async fn ws_attach_service(
  (key, sink, state): (String, ws::WsSink, web::types::State<DaemonState>),
) -> Result<
  impl Service<ws::Frame, Response = Option<ws::Message>, Error = io::Error>,
  web::Error,
> {
  // start heartbeat task
  let con_state = Rc::new(RefCell::new(WsConState::new()));
  let (tx, rx) = oneshot::channel();
  rt::spawn(utils::ws::heartbeat(con_state.clone(), sink.clone(), rx));
  let (scmd, mut rcmd) = mpsc::channel::<Result<Bytes, web::Error>>();
  let stream = state
    .docker_api
    .attach_container(
      &format!("{key}.v"),
      Some(AttachContainerOptions::<String> {
        stdin: Some(true),
        stdout: Some(true),
        stderr: Some(true),
        stream: Some(true),
        logs: Some(false),
        detach_keys: Some("ctrl-c".to_owned()),
      }),
    )
    .await
    .map_err(HttpError::internal_server_error)?;
  rt::spawn(async move {
    let mut output = stream.output;
    while let Some(output) = output.next().await {
      let output = match output {
        Ok(output) => output,
        Err(e) => {
          log::error!("Error reading from container: {}", e);
          break;
        }
      };
      let outputlog: OutputLog = output.into();
      let output = serde_json::to_vec(&outputlog);
      let mut output = match output {
        Ok(output) => output,
        Err(e) => {
          log::error!("Error serializing output: {}", e);
          break;
        }
      };
      output.push(b'\n');
      let msg = ws::Message::Binary(Bytes::from(output));
      if sink.send(msg).await.is_err() {
        break;
      }
    }
  });
  rt::spawn(async move {
    let mut stdin = stream.input;
    while let Some(cmd) = rcmd.next().await {
      let cmd = match cmd {
        Ok(cmd) => cmd,
        Err(e) => {
          log::error!("Error reading from container: {}", e);
          break;
        }
      };
      if stdin.write_all(&cmd).await.is_err() {
        break;
      }
    }
  });
  // handler service for incoming websockets frames
  let service = fn_service(move |frame| {
    let item = match frame {
      ws::Frame::Ping(msg) => {
        con_state.borrow_mut().hb = Instant::now();
        Some(ws::Message::Pong(msg))
      }
      // update heartbeat
      ws::Frame::Pong(_) => {
        con_state.borrow_mut().hb = Instant::now();
        None
      }
      ws::Frame::Text(text) => {
        let _ = scmd.send(Ok(text));
        None
      }
      ws::Frame::Binary(_) => None,
      ws::Frame::Close(reason) => Some(ws::Message::Close(reason)),
      _ => Some(ws::Message::Close(None)),
    };
    ready(Ok(item))
  });
  // handler service for shutdown notification that stop heartbeat task
  let on_shutdown = fn_shutdown(move || {
    let _ = tx.send(());
  });
  // pipe our service with on_shutdown callback
  Ok(chain(service).and_then(on_shutdown))
}

/// Attach to a virtual machine via websocket
#[cfg_attr(feature = "dev", utoipa::path(
  get,
  tag = "Vms",
  path = "/vms/{name}/attach",
  params(
    ("name" = String, Path, description = "Name of the virtual machine"),
    ("namespace" = Option<String>, Query, description = "Namespace of the virtual machine"),
  ),
  responses(
    (status = 101, description = "Websocket connection"),
  ),
))]
pub(crate) async fn vm_attach(
  state: web::types::State<DaemonState>,
  path: web::types::Path<(String, String)>,
  req: web::HttpRequest,
  qs: web::types::Query<GenericNspQuery>,
) -> Result<web::HttpResponse, web::Error> {
  let namespace = utils::key::resolve_nsp(&qs.namespace);
  let key = utils::key::gen_key(&namespace, &path.1);
  web::ws::start(
    req,
    // inject state to ws_attach_service factory
    map_config(fn_factory_with_config(ws_attach_service), move |cfg| {
      (key.clone(), cfg, state.clone())
    }),
  )
  .await
}

pub(crate) fn ntex_config(config: &mut web::ServiceConfig) {
  config.service(list_vm);
  config.service(create_vm);
  config.service(delete_vm);
  config.service(inspect_vm);
  config.service(list_vm_history);
  config.service(patch_vm);
  config.service(
    web::resource("/vms/{name}/attach").route(web::get().to(vm_attach)),
  );
}

#[cfg(test)]
mod tests {
  use ntex::http;
  use nanocl_stubs::vm::{VmInspect, VmSummary};
  use nanocl_stubs::vm_spec::{VmSpecPartial, VmDisk};

  use crate::utils::tests::*;
  use crate::services::vm_image::tests::ensure_test_image;

  #[ntex::test]
  async fn basic() {
    ensure_test_image().await;
    let client = gen_default_test_client().await;
    let name = "api-test-vm";
    let image = "ubuntu-22-test";
    let res = client
      .post("/vms")
      .send_json(&VmSpecPartial {
        name: name.to_owned(),
        disk: VmDisk {
          image: image.to_owned(),
          ..Default::default()
        },
        ..Default::default()
      })
      .await
      .unwrap();
    test_status_code!(res.status(), http::StatusCode::OK, "create vm");
    let mut res = client
      .get(&format!("/vms/{name}/inspect"))
      .send()
      .await
      .unwrap();
    test_status_code!(res.status(), http::StatusCode::OK, "inspect vm");
    let vm = res.json::<VmInspect>().await.unwrap();
    assert_eq!(vm.spec.name, name);
    let mut res = client.get("/vms").send().await.unwrap();
    test_status_code!(res.status(), http::StatusCode::OK, "list vm");
    let vms = res.json::<Vec<VmSummary>>().await.unwrap();
    assert!(vms.iter().any(|i| i.spec.name == name));
    let res = client.delete("/vms/api-test-vm").send().await.unwrap();
    test_status_code!(res.status(), http::StatusCode::OK, "delete vm");
  }
}
