use serde::Deserialize;

use super::{Engine, ContainerInfo};
use crate::prelude::*;
use crate::util::command_extensions::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PodmanContainerInfoConfig {
    labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PodmanContainerInfo {
    name: String,
    config: PodmanContainerInfoConfig,
}

impl ContainerInfo for PodmanContainerInfo {
    fn get_name(&self) -> &str {
        "test"
    }

    fn get_label(&self, name: &str) -> Option<&str> {
        None
    }
}

pub struct Podman {
    path: String,
}

impl Engine for Podman {
    fn name(&self) -> &str {
        "podman"
    }

    fn exec<T: AsRef<std::ffi::OsStr>>(&self, container: &str, command: &[T]) -> Result<String> {
        let output = self.command()
            .args(["exec", "--user", "root", container])
            .args(command)
            .run_get_output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn command(&self) -> Command {
        Command::new(&self.path)
    }

    fn get_containers(&self, label: &str, value: Option<&str>) -> Result<Vec<String>> {
        let mut cmd = self.command();

        // just print names of the containers
        cmd.args(["container", "ls", "--format", "{{ .Names }}"]);

        if let Some(value) = value {
            cmd.arg(format!("--filter={label}={value}"));
        } else {
            cmd.arg(format!("--filter={label}"));
        }

        let output = cmd.run_get_output()?;

        Ok(String::from_utf8_lossy(&output.stdout).lines().map(|x| x.to_string()).collect())
    }

    fn inspect_container(&self, container: &str) -> Result<impl ContainerInfo> {
        let output = self.command()
            .args(["inspect", container])
            .run_get_output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(
            serde_json::from_str::<PodmanContainerInfo>(&stdout)
                .with_context(|| "Error parsing output from \"podman inspect\"")?
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: this is truncated output from `podman inspect`, i've removed mounts as it gets too
    // long
    const INSPECT_OUTPUT: &str = r#"[
     {
          "Id": "78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa",
          "Created": "2025-02-10T09:37:07.807524533+01:00",
          "Path": "/run/podman-init",
          "Args": [
               "--",
               "/arcam/exe",
               "init"
          ],
          "State": {
               "OciVersion": "1.2.0",
               "Status": "running",
               "Running": true,
               "Paused": false,
               "Restarting": false,
               "OOMKilled": false,
               "Dead": false,
               "Pid": 5778,
               "ConmonPid": 5775,
               "ExitCode": 0,
               "Error": "",
               "StartedAt": "2025-02-10T09:37:08.003102808+01:00",
               "FinishedAt": "0001-01-01T00:00:00Z",
               "CgroupPath": "/user.slice/user-1000.slice/user@1000.service/user.slice/libpod-78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa.scope",
               "CheckpointedAt": "0001-01-01T00:00:00Z",
               "RestoredAt": "0001-01-01T00:00:00Z"
          },
          "Image": "86a5658fe1c05df756f375cf8c8d3f7ab19b19811792bb7baa82f4ddbd23d1f9",
          "ImageDigest": "sha256:52ab3bb009758924c16bc7b4e72b82180445789f5ae79cf0366fe454b5a32a28",
          "ImageName": "ghcr.io/sandorex/arcam-fedora:latest",
          "Rootfs": "",
          "Pod": "",
          "ResolvConfPath": "/run/user/1000/containers/overlay-containers/78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa/userdata/resolv.conf",
          "HostnamePath": "/run/user/1000/containers/overlay-containers/78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa/userdata/hostname",
          "HostsPath": "/run/user/1000/containers/overlay-containers/78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa/userdata/hosts",
          "StaticDir": "/home/sandorex/.local/share/containers/storage/overlay-containers/78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa/userdata",
          "OCIConfigPath": "/home/sandorex/.local/share/containers/storage/overlay-containers/78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa/userdata/config.json",
          "OCIRuntime": "crun",
          "ConmonPidFile": "/run/user/1000/containers/overlay-containers/78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa/userdata/conmon.pid",
          "PidFile": "/run/user/1000/containers/overlay-containers/78d8d26cc93cad9bddcfe52227353177511907ca1a326b391f7d1dff8146f0aa/userdata/pidfile",
          "Name": "wrathful-arcam",
          "RestartCount": 0,
          "Driver": "overlay",
          "MountLabel": "system_u:object_r:container_file_t:s0:c1022,c1023",
          "ProcessLabel": "",
          "AppArmorProfile": "",
          "EffectiveCaps": [
               "CAP_CHOWN",
               "CAP_DAC_OVERRIDE",
               "CAP_FOWNER",
               "CAP_FSETID",
               "CAP_KILL",
               "CAP_NET_BIND_SERVICE",
               "CAP_SETFCAP",
               "CAP_SETGID",
               "CAP_SETPCAP",
               "CAP_SETUID",
               "CAP_SYS_CHROOT"
          ],
          "BoundingCaps": [
               "CAP_CHOWN",
               "CAP_DAC_OVERRIDE",
               "CAP_FOWNER",
               "CAP_FSETID",
               "CAP_KILL",
               "CAP_NET_BIND_SERVICE",
               "CAP_SETFCAP",
               "CAP_SETGID",
               "CAP_SETPCAP",
               "CAP_SETUID",
               "CAP_SYS_CHROOT"
          ],
          "ExecIDs": [
               "1537ef48b64f98b3cec96e4d903eb9cceb7efeb189294926cb75e532ca510416"
          ],
          "GraphDriver": {
               "Name": "overlay",
               "Data": {
                    "LowerDir": "/home/sandorex/.local/share/containers/storage/overlay/26371b68f4cda9c76119cc6ae2c398dfa0211091254127076936502726db6db2/diff",
                    "MergedDir": "/home/sandorex/.local/share/containers/storage/overlay/43dece48d5035ffc1808d00a64451f7dd4ce08dcc31901e5b141ede7f84f9dac/merged",
                    "UpperDir": "/home/sandorex/.local/share/containers/storage/overlay/43dece48d5035ffc1808d00a64451f7dd4ce08dcc31901e5b141ede7f84f9dac/diff",
                    "WorkDir": "/home/sandorex/.local/share/containers/storage/overlay/43dece48d5035ffc1808d00a64451f7dd4ce08dcc31901e5b141ede7f84f9dac/work"
               }
          },
          "Mounts": [],
          "Dependencies": [],
          "NetworkSettings": {
               "EndpointID": "",
               "Gateway": "",
               "IPAddress": "",
               "IPPrefixLen": 0,
               "IPv6Gateway": "",
               "GlobalIPv6Address": "",
               "GlobalIPv6PrefixLen": 0,
               "MacAddress": "",
               "Bridge": "",
               "SandboxID": "",
               "HairpinMode": false,
               "LinkLocalIPv6Address": "",
               "LinkLocalIPv6PrefixLen": 0,
               "Ports": {},
               "SandboxKey": "/run/user/1000/netns/netns-d17c8fc1-f6d2-f5bf-e7d6-8f966f9c0344"
          },
          "Namespace": "",
          "IsInfra": false,
          "IsService": false,
          "KubeExitCodePropagation": "invalid",
          "lockNumber": 0,
          "Config": {
               "Hostname": "thorium",
               "Domainname": "",
               "User": "root",
               "AttachStdin": false,
               "AttachStdout": false,
               "AttachStderr": false,
               "Tty": false,
               "OpenStdin": false,
               "StdinOnce": false,
               "Env": [
                    "LANG=en_US.UTF-8",
                    "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                    "XDG_RUNTIME_DIR=/run/user/1000",
                    "ARCAM_VERSION=0.1.10",
                    "NVIM_FORCE_OSC52=true",
                    "HOST_USER_GID=1000",
                    "TERMINFO_DIRS=/host/usr/share/terminfo:/host/etc/terminfo:/usr/share/terminfo:/etc/terminfo",
                    "HOST_USER=sandorex",
                    "HOST_USER_UID=1000",
                    "LC_ALL=en_US.UTF-8",
                    "CONTAINER_ENGINE=podman",
                    "manager=podman",
                    "RUSTUP_HOME=/opt/rustup",
                    "container=oci",
                    "arcam=arcam",
                    "CONTAINER_NAME=wrathful-arcam",
                    "HOSTNAME=thorium",
                    "HOME=/root"
               ],
               "Cmd": [
                    "init"
               ],
               "Image": "ghcr.io/sandorex/arcam-fedora:latest",
               "Volumes": null,
               "WorkingDir": "/",
               "Entrypoint": [
                    "/arcam/exe"
               ],
               "OnBuild": null,
               "Labels": {
                    "arcam": "0.1.10",
                    "com.github.containers.toolbox": "true",
                    "container_dir": "/home/sandorex/ws/arcam",
                    "default_shell": "/bin/fish",
                    "host_dir": "/mnt/slowmf/ws/projects/arcam",
               },
               "Annotations": {
                    "io.container.manager": "libpod",
                    "io.podman.annotations.autoremove": "TRUE",
                    "io.podman.annotations.init": "TRUE",
                    "io.podman.annotations.label": "disable",
                    "io.podman.annotations.userns": "keep-id",
                    "org.opencontainers.image.stopSignal": "15",
                    "org.systemd.property.KillSignal": "15",
                    "org.systemd.property.TimeoutStopUSec": "uint64 10000000",
                    "run.oci.keep_original_groups": "1"
               },
               "StopSignal": "SIGTERM",
               "HealthcheckOnFailureAction": "none",
               "HealthLogDestination": "local",
               "HealthcheckMaxLogCount": 5,
               "HealthcheckMaxLogSize": 500,
               "CreateCommand": [
               ],
               "Timezone": "local",
               "Umask": "0022",
               "Timeout": 0,
               "StopTimeout": 10,
               "Passwd": true,
               "sdNotifyMode": "container"
          },
          "HostConfig": {
               "Binds": [],
               "CgroupManager": "systemd",
               "CgroupMode": "private",
               "ContainerIDFile": "",
               "LogConfig": {
                    "Type": "journald",
                    "Config": null,
                    "Path": "",
                    "Tag": "",
                    "Size": "0B"
               },
               "NetworkMode": "pasta",
               "PortBindings": {},
               "RestartPolicy": {
                    "Name": "no",
                    "MaximumRetryCount": 0
               },
               "AutoRemove": true,
               "AutoRemoveImage": false,
               "Annotations": {
                    "io.container.manager": "libpod",
                    "io.podman.annotations.autoremove": "TRUE",
                    "io.podman.annotations.init": "TRUE",
                    "io.podman.annotations.label": "disable",
                    "io.podman.annotations.userns": "keep-id",
                    "org.opencontainers.image.stopSignal": "15",
                    "org.systemd.property.KillSignal": "15",
                    "org.systemd.property.TimeoutStopUSec": "uint64 10000000",
                    "run.oci.keep_original_groups": "1"
               },
               "VolumeDriver": "",
               "VolumesFrom": null,
               "CapAdd": [],
               "CapDrop": [],
               "Dns": [],
               "DnsOptions": [],
               "DnsSearch": [],
               "ExtraHosts": [],
               "GroupAdd": [],
               "IpcMode": "shareable",
               "Cgroup": "",
               "Cgroups": "default",
               "Links": null,
               "OomScoreAdj": 0,
               "PidMode": "private",
               "Privileged": false,
               "PublishAllPorts": false,
               "ReadonlyRootfs": false,
               "SecurityOpt": [
                    "label=disable"
               ],
               "Tmpfs": {},
               "UTSMode": "private",
               "UsernsMode": "private",
               "IDMappings": {
                    "UidMap": [
                         "0:1:1000",
                         "1000:0:1",
                         "1001:1001:64536"
                    ],
                    "GidMap": [
                         "0:1:1000",
                         "1000:0:1",
                         "1001:1001:64536"
                    ]
               },
               "ShmSize": 65536000,
               "Runtime": "oci",
               "ConsoleSize": [
                    0,
                    0
               ],
               "Isolation": "",
               "CpuShares": 0,
               "Memory": 0,
               "NanoCpus": 0,
               "CgroupParent": "user.slice",
               "BlkioWeight": 0,
               "BlkioWeightDevice": null,
               "BlkioDeviceReadBps": null,
               "BlkioDeviceWriteBps": null,
               "BlkioDeviceReadIOps": null,
               "BlkioDeviceWriteIOps": null,
               "CpuPeriod": 0,
               "CpuQuota": 0,
               "CpuRealtimePeriod": 0,
               "CpuRealtimeRuntime": 0,
               "CpusetCpus": "",
               "CpusetMems": "",
               "Devices": [],
               "DiskQuota": 0,
               "KernelMemory": 0,
               "MemoryReservation": 0,
               "MemorySwap": 0,
               "MemorySwappiness": 0,
               "OomKillDisable": false,
               "Init": true,
               "PidsLimit": 2048,
               "Ulimits": [
                    {
                         "Name": "RLIMIT_NOFILE",
                         "Soft": 1048576,
                         "Hard": 1048576
                    },
                    {
                         "Name": "RLIMIT_NPROC",
                         "Soft": 111136,
                         "Hard": 111136
                    }
               ],
               "CpuCount": 0,
               "CpuPercent": 0,
               "IOMaximumIOps": 0,
               "IOMaximumBandwidth": 0,
               "CgroupConf": null
          }
     }
]
"#;

    #[test]
    fn test_podman_inspect() {
        let obj = serde_json::from_str::<Vec<PodmanContainerInfo>>(INSPECT_OUTPUT);
        assert!(obj.is_ok(), "Error parsing: {:?}", obj);
        assert_eq!(
            obj.unwrap().first().take(),
            PodmanContainerInfo {
                name: "wrathful-arcam".to_string(),
                config: PodmanContainerInfoConfig {
                    labels: HashMap::from([
                        ("arcam".to_string(), "0.1.10".to_string()),
                        ("com.github.containers.toolbox".to_string(), "true".to_string()),
                        ("container_dir".to_string(), "/home/sandorex/ws/arcam".to_string()),
                        ("default_shell".to_string(), "/bin/fish".to_string()),
                        ("host_dir".to_string(), "/mnt/slowmf/ws/projects/arcam".to_string()),
                    ]),
                },
            }
        );
    }
}
