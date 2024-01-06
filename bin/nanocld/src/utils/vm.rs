use std::collections::HashMap;

use bollard_next::service::{HostConfig, DeviceMapping};

use nanocl_error::http::HttpResult;

use nanocl_stubs::vm::{Vm, VmSummary, VmInspect};

use crate::{
  utils,
  repositories::generic::*,
  models::{
    Pool, VmImageDb, SystemState, ProcessDb, NamespaceDb, VmDb, SpecDb,
  },
};

/// Get detailed information about a VM by his key
pub async fn inspect_by_key(
  vm_key: &str,
  state: &SystemState,
) -> HttpResult<VmInspect> {
  let vm = VmDb::transform_read_by_pk(vm_key, &state.pool).await?;
  let processes =
    ProcessDb::read_by_kind_key(&vm.spec.vm_key, &state.pool).await?;
  let (_, _, _, running_instances) = utils::process::count_status(&processes);
  Ok(VmInspect {
    created_at: vm.created_at,
    namespace_name: vm.namespace_name,
    spec: vm.spec,
    instance_total: processes.len(),
    instance_running: running_instances,
    instances: processes,
  })
}

/// List VMs by namespace
pub async fn list_by_namespace(
  nsp: &str,
  pool: &Pool,
) -> HttpResult<Vec<VmSummary>> {
  let namespace = NamespaceDb::read_by_pk(nsp, pool).await?;
  let vmes = VmDb::read_by_namespace(&namespace.name, pool).await?;
  let mut vm_summaries = Vec::new();
  for vm in vmes {
    let spec = SpecDb::read_by_pk(&vm.spec.key, pool)
      .await?
      .try_to_vm_spec()?;
    let processes = ProcessDb::read_by_kind_key(&vm.spec.vm_key, pool).await?;
    let (_, _, _, running_instances) = utils::process::count_status(&processes);
    vm_summaries.push(VmSummary {
      created_at: vm.created_at,
      namespace_name: vm.namespace_name,
      instance_total: processes.len(),
      instance_running: running_instances,
      spec: spec.clone(),
    });
  }
  Ok(vm_summaries)
}

/// Create a VM instance from a VM image
pub async fn create_instance(
  vm: &Vm,
  image: &VmImageDb,
  disable_keygen: bool,
  state: &SystemState,
) -> HttpResult<()> {
  let mut labels: HashMap<String, String> = HashMap::new();
  let vmimagespath = format!("{}/vms/images", state.config.state_dir);
  labels.insert("io.nanocl.v".to_owned(), vm.spec.vm_key.clone());
  labels.insert("io.nanocl.n".to_owned(), vm.namespace_name.clone());
  let mut args: Vec<String> =
    vec!["-hda".into(), image.path.clone(), "--nographic".into()];
  let host_config = vm.spec.host_config.clone();
  let kvm = host_config.kvm.unwrap_or_default();
  let mut devices = vec![DeviceMapping {
    path_on_host: Some("/dev/net/tun".into()),
    path_in_container: Some("/dev/net/tun".into()),
    cgroup_permissions: Some("rwm".into()),
  }];
  if kvm {
    args.push("-accel".into());
    args.push("kvm".into());
    devices.push(DeviceMapping {
      path_on_host: Some("/dev/kvm".into()),
      path_in_container: Some("/dev/kvm".into()),
      cgroup_permissions: Some("rwm".into()),
    });
    log::debug!("KVM enabled /dev/kvm mapped");
  }
  let cpu = host_config.cpu;
  let cpu = if cpu > 0 { cpu.to_string() } else { "1".into() };
  let cpu = cpu.clone();
  args.push("-smp".into());
  args.push(cpu.clone());
  let memory = host_config.memory;
  let memory = if memory > 0 {
    format!("{memory}M")
  } else {
    "512M".into()
  };
  args.push("-m".into());
  args.push(memory);
  let mut envs: Vec<String> = Vec::new();
  let net_iface = vm
    .spec
    .host_config
    .net_iface
    .clone()
    .unwrap_or("ens3".into());
  let link_net_iface = vm
    .spec
    .host_config
    .link_net_iface
    .clone()
    .unwrap_or("eth0".into());
  envs.push(format!("DEFAULT_INTERFACE={link_net_iface}"));
  envs.push(format!("FROM_NETWORK={net_iface}"));
  envs.push(format!("DELETE_SSH_KEY={disable_keygen}"));
  if let Some(user) = &vm.spec.user {
    envs.push(format!("USER={user}"));
  }
  if let Some(password) = &vm.spec.password {
    envs.push(format!("PASSWORD={password}"));
  }
  if let Some(ssh_key) = &vm.spec.ssh_key {
    envs.push(format!("SSH_KEY={ssh_key}"));
  }
  let image = match &vm.spec.host_config.runtime {
    Some(runtime) => runtime.to_owned(),
    None => "ghcr.io/next-hat/nanocl-qemu:8.0.2.0".into(),
  };
  let spec = bollard_next::container::Config {
    image: Some(image),
    tty: Some(true),
    hostname: vm.spec.hostname.clone(),
    env: Some(envs),
    labels: Some(labels),
    cmd: Some(args),
    attach_stderr: Some(true),
    attach_stdin: Some(true),
    attach_stdout: Some(true),
    open_stdin: Some(true),
    host_config: Some(HostConfig {
      network_mode: Some(
        vm.spec
          .host_config
          .runtime_network
          .clone()
          .unwrap_or(vm.namespace_name.to_owned()),
      ),
      binds: Some(vec![format!("{vmimagespath}:{vmimagespath}")]),
      devices: Some(devices),
      cap_add: Some(vec!["NET_ADMIN".into()]),
      ..Default::default()
    }),
    ..Default::default()
  };
  let name = format!("{}.v", &vm.spec.vm_key);
  utils::process::create(&name, "vm", &vm.spec.vm_key, spec, state).await?;
  Ok(())
}
