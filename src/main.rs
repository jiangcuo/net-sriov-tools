use clap::{Arg, Command};
use std::fs;
use std::path::Path;
use prettytable::{Table, row, cell};
use serde_json;


fn main() {
    let mut app = Command::new("net-sriov-tools")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Lierfang <itsupport@lierfang.com>")
        .about("PXVIRT SR-IOV Network Interface Card Manager")
        .subcommand(Command::new("list")
            .about("List SR-IOV capable network interfaces")
            .arg(Arg::new("interface")
                .help("Specific network interface to list SR-IOV devices for")
                .required(false))
            .arg(Arg::new("output")
                .long("output")
                .help("Output format")
                .num_args(1)
                .required(false)
                .value_parser(["json"])))
        .subcommand(Command::new("create")
            .about("Create SR-IOV devices for a network interface")
            .arg(Arg::new("interface")
                .help("Network interface to create SR-IOV devices for")
                .required(true))
            .arg(Arg::new("nums")
                .help("Number of SR-IOV devices to create")
                .required(true)))
        .subcommand(Command::new("save")
            .about("Save the current configuration of all SR-IOV capable network interfaces"))
        .subcommand(Command::new("load")
            .about("Load SR-IOV configurations from /etc/network/sriov.d and apply them"));

    let matches = app.clone().get_matches();

    match matches.subcommand() {
        Some(("list", sub_m)) => {
            let output_json = sub_m.get_one::<String>("output").map_or(false, |v| v == "json");
            if let Some(interface) = sub_m.get_one::<String>("interface") {
                list_sriov_devices(interface, output_json);
            } else {
                list_sriov_capable_interfaces(output_json);
            }
        }
        Some(("create", sub_m)) => {
            let interface = sub_m.get_one::<String>("interface").unwrap();
            let nums: usize = sub_m.get_one::<String>("nums").unwrap().parse().unwrap();
            create_sriov_devices(interface, nums);
        }
        Some(("save", sub_m)) => {
            save_configuration();
        }
        Some(("load", _)) => {
            load_configuration();
        }
        _ => {
            let _ = app.print_help();
            println!("");
        }
    }
}

fn list_sriov_capable_interfaces(output_json: bool) {
    let net_path = Path::new("/sys/class/net");
    let mut interfaces = Vec::new();

    if let Ok(entries) = fs::read_dir(net_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let interface_name = entry.file_name().into_string().unwrap();
                let sriov_path = entry.path().join("device/sriov_totalvfs");
                let addr_path = entry.path().join("address");
                let inuse_vf_path = entry.path().join("device/sriov_numvfs");
                let pcie_addr_path = entry.path().join("device").join("uevent");

                if sriov_path.exists() {
                    if let Ok(content) = fs::read_to_string(sriov_path) {
                        let total_vfs: u32 = content.trim().parse().unwrap_or(0);
                        if total_vfs > 0 {
                            let addr = fs::read_to_string(addr_path).unwrap_or_default().trim().to_string();
                            let inuse_vfs: u32 = fs::read_to_string(inuse_vf_path).unwrap_or_default().trim().parse().unwrap_or(0);
                            let pcie_addr = if let Ok(uevent_content) = fs::read_to_string(pcie_addr_path) {
                                uevent_content.lines().find(|line| line.starts_with("PCI_SLOT_NAME=")).map(|line| line.replace("PCI_SLOT_NAME=", "")).unwrap_or_default()
                            } else {
                                String::from("N/A")
                            };
                            interfaces.push((interface_name, addr, pcie_addr, total_vfs, inuse_vfs));
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("Failed to read network interfaces.");
    }

    if output_json {
        println!("{}", serde_json::to_string(&interfaces.iter().map(|(interface_name, addr, pcie_addr, total_vfs, inuse_vfs)| {
            serde_json::json!({
                "Name": interface_name,
                "Addr": addr,
                "PCIeAddr": pcie_addr,
                "Max-VF": total_vfs,
                "Inuse-VF": inuse_vfs
            })
        }).collect::<Vec<_>>()).unwrap());
    } else {
        let mut table = Table::new();
        table.add_row(row!["Name", "Addr", "PCIeAddr", "Max-VF", "Inuse-VF"]);
        for (interface_name, addr, pcie_addr, total_vfs, inuse_vfs) in interfaces {
            table.add_row(row![interface_name, addr, pcie_addr, total_vfs, inuse_vfs]);
        }
        table.printstd();
    }
}

fn list_sriov_devices(interface: &str, output_json: bool) {
    let net_path = Path::new("/sys/class/net").join(interface);
    let mut devices = Vec::new();

    let vf_path = net_path.join("device");
    if let Ok(total_vfs) = fs::read_to_string(vf_path.join("sriov_totalvfs")) {
        let total_vfs: u32 = total_vfs.trim().parse().unwrap_or(0);
        for vf_index in 0..total_vfs {
            let vf_name = format!("{}v{}", interface, vf_index);
            let vf_net_path = Path::new("/sys/class/net").join(&vf_name);
            let addr_path = vf_net_path.join("address");
            let pcie_addr_path = vf_net_path.join("device").join("uevent");
            let driver_path = vf_net_path.join("device").join("driver");

            if let Ok(addr) = fs::read_to_string(addr_path) {
                let addr = addr.trim().to_string();
                let pcie_addr = if let Ok(uevent_content) = fs::read_to_string(pcie_addr_path) {
                    uevent_content.lines().find(|line| line.starts_with("PCI_SLOT_NAME=")).map(|line| line.replace("PCI_SLOT_NAME=", "")).unwrap_or_default()
                } else {
                    String::from("N/A")
                };
                let driver = if let Ok(driver_content) = fs::read_link(driver_path) {
                    driver_content.file_name().unwrap_or_default().to_string_lossy().into_owned()
                } else {
                    String::from("N/A")
                };
                devices.push((vf_name, addr, pcie_addr, driver));
            }
        }
    } else {
        eprintln!("Failed to read SR-IOV devices for interface: {}", interface);
    }

    if output_json {
        println!("{}", serde_json::to_string(&devices.iter().map(|(vf_name, addr, pcie_addr, driver)| {
            serde_json::json!({
                "Name": vf_name,
                "Addr": addr,
                "PCIeAddr": pcie_addr,
                "Driver": driver
            })
        }).collect::<Vec<_>>()).unwrap());
    } else {
        let mut table = Table::new();
        table.add_row(row!["Name", "Addr", "PCIeAddr", "Driver"]);
        for (vf_name, addr, pcie_addr, driver) in devices {
            table.add_row(row![vf_name, addr, pcie_addr, driver]);
        }
        table.printstd();
    }
}

fn create_sriov_devices(interface: &str, nums: usize) {
    let net_path = Path::new("/sys/class/net").join(interface).join("device");
    let total_vfs_path = net_path.join("sriov_totalvfs");
    let num_vfs_path = net_path.join("sriov_numvfs");
    let inuse_vf_path = net_path.join("sriov_numvfs");

    // 检查网卡支持的最大VF数量
    let max_vfs = match fs::read_to_string(&total_vfs_path) {
        Ok(content) => content.trim().parse::<usize>().unwrap_or(0),
        Err(_) => {
            eprintln!("Failed to read total VFs for interface: {}", interface);
            return;
        }
    };

    if nums > max_vfs {
        eprintln!("Requested VFs exceed the maximum supported VFs for interface: {}", interface);
        return;
    }
    let inuse_vfs = match fs::read_to_string(&inuse_vf_path) {
        Ok(content) => content.trim().parse::<usize>().unwrap_or(0),
        Err(_) => {
            eprintln!("Failed to read in-use VFs for interface: {}", interface);
            return;
        }
    };

    if inuse_vfs > 0 &&  nums > 0 {
        eprintln!("Interface: {} has {} in-use VFs, please remove them first", interface, inuse_vfs);
        return;
    }

    // 设置所需的VF数量
    if let Err(_) = fs::write(&num_vfs_path, nums.to_string()) {
        eprintln!("Failed to set the number of VFs for interface: {}", interface);
        return;
    }

    println!("Created {} SR-IOV devices for interface: {}", nums, interface);
}

fn save_configuration() {
    let net_path = Path::new("/sys/class/net");
    let config_dir = Path::new("/etc/network/sriov.d");

    // 确保配置目录存在
    if !config_dir.exists() {
        if let Err(e) = fs::create_dir_all(config_dir) {
            eprintln!("Failed to create configuration directory: {}", e);
            return;
        }
    }

    if let Ok(entries) = fs::read_dir(net_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let interface_name = entry.file_name().into_string().unwrap();
                let vf_path = entry.path().join("device");
                let sriov_path = entry.path().join("device/sriov_totalvfs");

                // 检查是否支持SR-IOV
                if sriov_path.exists() {
                    if let Ok(total_vfs) = fs::read_to_string(&sriov_path) {
                        let total_vfs: u32 = total_vfs.trim().parse().unwrap_or(0);
                        if total_vfs > 0 {
                            let config_file_path = config_dir.join(&interface_name);
                            let mut config_content = String::new();

                            // 读取当前的VF数量
                            if let Ok(num_vfs) = fs::read_to_string(vf_path.join("sriov_numvfs")) {
                                config_content.push_str(&format!("{}
", num_vfs.trim()));
                            }

                            // 保存配置到文件
                            if let Err(e) = fs::write(&config_file_path, config_content) {
                                eprintln!("Failed to write configuration for interface {}: {}", interface_name, e);
                            } else {
                                println!("Configuration for interface {} saved to {}", interface_name, config_file_path.display());
                            }
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("Failed to read network interfaces.");
    }
}

fn load_configuration() {
    let config_dir = Path::new("/etc/network/sriov.d");

    if let Ok(entries) = fs::read_dir(config_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let interface_name = entry.file_name().into_string().unwrap();
                let config_file_path = entry.path();

                if let Ok(content) = fs::read_to_string(&config_file_path) {
                    let num_vfs: usize = content.trim().parse().unwrap_or(0);
                    let net_path = Path::new("/sys/class/net").join(&interface_name).join("device").join("sriov_numvfs");

                    if let Err(e) = fs::write(&net_path, num_vfs.to_string()) {
                        eprintln!("Failed to apply configuration for interface {}: {}", interface_name, e);
                    } else {
                        println!("Applied configuration for interface {}", interface_name);
                    }
                } else {
                    eprintln!("Failed to read configuration file for interface {}", interface_name);
                }
            }
        }
    } else {
        eprintln!("Failed to read configuration directory.");
    }
}