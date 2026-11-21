//! What a recovery does to use each phone's keys, found in what the
//! phone's own partitions say, read from the files `skyward-pack described`
//! took out of the phone's stock build into tests/described. For the phone
//! this build is made for, what is found is what was started on the phone
//! then.

use skyward_config::Service;
use skyward_programs::naming::{self, Does};
use skyward_programs::phone::{Happens, Partition, Phone, PARTITIONS};
use skyward_programs::secure::{self, Started, Step};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn described(codename: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/described").join(codename)
}

/// Where a recovery has each partition mounted: the phone's own where the
/// phone has them, the installed system's under /mnt.
fn mounted(partition: &str) -> String {
    match partition {
        "vendor" | "odm" => format!("/{partition}"),
        other => format!("/mnt/{other}"),
    }
}

/// The phone `codename` names, as its stock build describes it, with the
/// properties its bootloader and its build give.
fn phone(codename: &str, properties: &[(&str, &str)]) -> Phone {
    with_bundle(codename, properties, None)
}

/// That phone, with `bundle` as where a recovery has opened the libraries
/// its partitions were built against.
fn with_bundle(codename: &str, properties: &[(&str, &str)], bundle: Option<&Path>) -> Phone {
    let at = described(codename);
    let partitions = PARTITIONS.iter().map(|name| Partition { name: name.to_string(), mounted: mounted(name).into(), at: at.join(name) }).collect();
    let properties: HashMap<String, String> = properties.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect();
    Phone::read(partitions, properties, bundle).unwrap()
}

/// The steps, each as the name of the program's file, or as the secure
/// element service where the recovery serves it.
fn names(steps: &[Step]) -> Vec<&str> {
    steps
        .iter()
        .map(|step| match step {
            Step::Start(started) => started.service.path.rsplit('/').next().unwrap(),
            Step::ServeElements => "(the secure element service)",
        })
        .collect()
}

fn strings(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

/// The folders a program finds its libraries in, by the partition it is
/// on.
const ON_VENDOR: [&str; 5] = ["/system/lib64", "/vendor/lib64", "/vendor/lib64/hw", "/mnt/system/system/lib64", "/mnt/system/system/lib64/bootstrap"];

/// A program kept running as `user` in `groups`, with nothing else said
/// of it.
fn program(path: &str, libraries: &[&str], user: &str, groups: &[&str]) -> Service {
    Service {
        path: path.into(),
        args: Vec::new(),
        libraries: strings(libraries),
        oneshot: false,
        ready_property: None,
        ready_service: None,
        registers: Vec::new(),
        ready_hardware_service: None,
        when: None,
        user: Some(user.into()),
        groups: strings(groups),
        capabilities: Vec::new(),
    }
}




/// The builds of the hardware a partition's main descriptions are named
/// after, which a phone is one of: `manifest_<build>.xml` in the
/// partition's description folder.
fn builds(codename: &str, partition: &str) -> Vec<String> {
    let folder = described(codename).join(partition).join("etc/vintf");
    let mut builds: Vec<String> = std::fs::read_dir(&folder)
        .unwrap_or_else(|e| panic!("list {}: {e}", folder.display()))
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter_map(|name| Some(name.strip_prefix("manifest_")?.strip_suffix(".xml")?.to_string()))
        .collect();
    builds.sort();
    builds
}

/// Every phone's stock build names one program for each service the
/// recovery asks, whichever build of the hardware the phone is: the odm
/// partition's description is the one named after the phone where it has
/// one, and the vendor partition's any of those it carries. A phone with a
/// weaver starts what the weaver goes through before it. No service is
/// told of as registered by two programs, all the phone describes reads
/// back from the description written of it for the service directory, what
/// the programs register among it, and every account a program runs as or
/// is handed a socket of is one the phone's own lists of accounts give a
/// number.
#[test]
fn every_phone_names_one_program_for_each_service() {
    let mut codenames: Vec<String> = std::fs::read_dir(described("")).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    codenames.sort();
    assert_eq!(codenames, vec!["luming".to_string()], "the phones described");
    for codename in &codenames {
        let mut vendor = builds(codename, "vendor");
        if vendor.is_empty() {
            vendor.push(String::new());
        }
        let hardware = builds(codename, "odm").into_iter().find(|build| build == codename).unwrap_or_default();
        let mut found: Vec<(String, Vec<Step>)> = Vec::new();
        for build in &vendor {
            let properties = [("ro.boot.hardware", "qcom"), ("ro.boot.product.vendor.sku", build.as_str()), ("ro.boot.product.hardware.sku", hardware.as_str())];
            let read = phone(codename, &properties);
            let steps = secure::steps(&read).unwrap_or_else(|e| panic!("{codename}, as the build {build:?} of its hardware: {e:#}"));
            let told_of = read.told_of();
            let written = skyward_programs::description::offering(&told_of).unwrap_or_else(|e| panic!("{codename}, as the build {build:?} of its hardware: {e:#}"));
            let read_back: Vec<String> = skyward_programs::description::read(&written).unwrap().iter().map(|described| described.name()).collect();
            assert_eq!(read_back, told_of, "{codename}, as the build {build:?} of its hardware: what the directory is told of is all the phone describes");
            for started in secure::programs(&steps) {
                for name in started.service.ready_service.iter().chain(&started.service.registers) {
                    assert!(told_of.contains(name), "{codename}, as the build {build:?} of its hardware, describes {name}");
                }
            }
            found.push((build.clone(), steps));
        }
        let (_, first) = &found[0];
        for (build, steps) in &found {
            assert_eq!(steps, first, "{codename} does the same as the build {build:?} of its hardware as any other");
        }
        let steps = names(first);
        if let Some(weaver) = steps.iter().position(|name| name.contains("weaver-service")) {
            assert!(weaver > 0 && (steps[weaver - 1].contains("secure_element") || steps[weaver - 1] == "(the secure element service)"), "{codename}: {steps:?}");
        }
        let lists: Vec<String> = ["vendor/etc/passwd", "odm/etc/passwd", "vendor/etc/group", "odm/etc/group"].iter().map(|list| described(codename).join(list).to_string_lossy().into_owned()).collect();
        let known = skyward_sys::ueventd::accounts(&lists.iter().map(String::as_str).collect::<Vec<_>>());
        let mut registered: Vec<&String> = Vec::new();
        for started in secure::programs(first) {
            let service = &started.service;
            let handed = started.sockets.iter().flat_map(|socket| socket.user.iter().chain(&socket.group));
            for account in service.user.iter().chain(&service.groups).chain(handed) {
                assert!(skyward_sys::ueventd::account(&known, account).is_some(), "{codename}: {} runs with the account {account}, which the phone gives no number", service.path);
            }
            assert!(Path::new(&service.path).starts_with("/vendor") || Path::new(&service.path).starts_with("/odm") || Path::new(&service.path).starts_with("/mnt/system_ext"), "{codename}: {}", service.path);
            assert!(service.registers.len() < 8, "{codename}: {} registers what one program does, {:?}", service.path, service.registers);
            for name in service.ready_service.iter().chain(&service.registers) {
                assert!(!registered.contains(&name), "{codename}: {name} is registered once");
                registered.push(name);
            }
            let said: Vec<String> = service.ready_property.iter().chain(&service.ready_service).chain(&service.ready_hardware_service).chain(&service.registers).cloned().collect();
            println!("{codename}: {} as {} in {:?}: {}", service.path, service.user.as_deref().unwrap_or("root"), service.groups, said.join(", "));
        }
        println!("{codename}: {}", steps.join(", "));
    }
}

/// The 17 Ultra in the build whose weaver is Goodix's, which goes through
/// Goodix's own service: the seven programs the recovery was given by hand
/// for it and used its keys with.


#[test]
fn luming_starts_its_secure_element_before_its_weaver() {
    for build in builds("luming", "vendor") {
        let properties = [
            ("ro.boot.hardware", "qcom"),
            ("ro.boot.product.vendor.sku", build.as_str()),
            ("ro.boot.product.hardware.sku", "luming"),
        ];
        let steps = secure::steps(&phone("luming", &properties)).unwrap();
        let names = names(&steps);
        let element = names.iter().position(|name| *name == "android.hardware.secure_element-service.qti").unwrap();
        let serve = names.iter().position(|name| *name == "(the secure element service)").unwrap();
        let weaver = names.iter().position(|name| name.contains("weaver-service.nxp-qti")).unwrap();

        assert!(element < serve && serve < weaver, "{build}: {names:?}");
        assert!(!names.contains(&"tee-supplicant"), "{build}: {names:?}");
        // The NXP NFC daemon declares the embedded element in its own
        // description but registers only the chip's own services: the
        // element's is the one Qualcomm's secure element service serves.
        assert!(!names.contains(&"android.hardware.nqnfc-service.nxp"), "{build}: {names:?}");

        let programs = secure::programs(&steps);
        let element = programs.iter().find(|started| started.service.path == "/vendor/bin/hw/android.hardware.secure_element-service.qti").unwrap();
        assert_eq!(element.service.ready_service.as_deref(), Some("android.hardware.secure_element.ISecureElement/eSE1"));
        assert_eq!(element.service.user.as_deref(), Some("system"));
        assert_eq!(element.service.groups, strings(&["nfc", "system"]));
    }
}

/// A script may name a program its partition does not carry, which init
/// leaves unstarted: the phone runs only those that are there, where the
/// script names them or behind the link one partition keeps to another.
#[test]
fn a_program_its_partition_does_not_carry_is_not_one_the_phone_runs() {
    let tree = tempfile::tempdir().unwrap();
    let (vendor, odm) = (tree.path().join("vendor"), tree.path().join("odm"));
    for folder in [vendor.join("etc/init"), vendor.join("bin"), odm.join("bin")] {
        std::fs::create_dir_all(folder).unwrap();
    }
    let script = "service there /vendor/bin/there\n    class hal\n\
                  service gone /vendor/bin/gone\n    class hal\n\
                  service linked /vendor/odm/bin/linked\n    class hal\n\
                  service elsewhere /system/bin/elsewhere\n    class hal\n";
    std::fs::write(vendor.join("etc/init/programs.rc"), script).unwrap();
    std::fs::write(vendor.join("bin/there"), b"").unwrap();
    std::fs::write(odm.join("bin/linked"), b"").unwrap();
    let partitions = vec![Partition { name: "vendor".into(), mounted: "/vendor".into(), at: vendor }, Partition { name: "odm".into(), mounted: "/odm".into(), at: odm }];
    let phone = Phone::read(partitions, HashMap::new(), None).unwrap();
    let runs = |name: &str| phone.runs(phone.programs.iter().find(|p| p.name == name).unwrap());
    assert_eq!(runs("there"), Happens::Yes);
    assert_eq!(runs("gone"), Happens::No);
    assert_eq!(runs("linked"), Happens::Yes, "on the odm partition, which the vendor partition links to");
    assert_eq!(runs("elsewhere"), Happens::No, "on a partition that is not read");
    let linked = phone.programs.iter().find(|p| p.name == "linked").unwrap();
    assert_eq!(phone.started_from(&linked.path), "/odm/bin/linked");
}

/// No bootloader says which build of the hardware a phone's vendor
/// partition describes: on every phone here the phone's init names it as
/// the phone starts, after the chip a program of the vendor partition's
/// makes out, and the vendor partition keeps a main description for each
/// chip it serves and none for a phone left unnamed. A phone so named has
/// nothing more to do.
#[test]
fn every_phone_s_init_names_the_build_its_vendor_partition_describes_after_its_chip() {
    let codenames: Vec<String> = std::fs::read_dir(described("")).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(codenames, vec!["luming".to_string()], "the phones described");
    let makes_the_chip_out = Service { oneshot: true, user: None, groups: Vec::new(), ..program("/vendor/bin/init.qti.qcv.sh", &ON_VENDOR, "root", &[]) };
    let named_as_init_names_it = [
        Does::Run(Box::new(Started::from(makes_the_chip_out))),
        Does::Set { name: "ro.boot.product.vendor.sku".into(), value: "${ro.vendor.qti.soc_name}".into() },
    ];
    for codename in &codenames {
        let hardware = [("ro.boot.hardware", "qcom"), ("ro.boot.product.hardware.sku", codename.as_str())];
        assert_eq!(naming::steps(&phone(codename, &hardware)).unwrap(), named_as_init_names_it, "{codename}");
        let chips = builds(codename, "vendor");
        assert!(!chips.is_empty(), "{codename} keeps a main description for each chip its vendor partition serves");
        assert!(!described(codename).join("vendor/etc/vintf/manifest.xml").exists(), "{codename} keeps none for a phone left unnamed");
        for chip in &chips {
            let named = [("ro.boot.hardware", "qcom"), ("ro.boot.product.hardware.sku", codename.as_str()), ("ro.boot.product.vendor.sku", chip.as_str())];
            assert_eq!(naming::steps(&phone(codename, &named)).unwrap(), [], "{codename}, named {chip}");
        }
    }
}
