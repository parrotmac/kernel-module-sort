use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alpha0, not_line_ending, u32 as nom_u32, u64 as nom_u64},
    character::complete::{alpha1, alphanumeric1, line_ending},
    combinator::recognize,
    multi::{many0, many0_count},
    sequence::{pair, terminated, tuple},
    IResult,
};

pub fn module_status_line(input: &str) -> IResult<&str, KernelModule> {
    let module_name = recognize(pair(alpha1, many0_count(alt((alphanumeric1, tag("_"))))));
    let space = take_while1(|c| c == ' ');
    let dependents = recognize(many0_count(alt((
        alphanumeric1,
        tag("_"),
        tag(","),
        tag("-"),
    ))));

    let (input, (module_name, _, module_size, _, refs, _, dependents, _, state, _, location, _)) =
        tuple((
            module_name,
            &space,
            nom_u64,
            &space,
            nom_u32,
            &space,
            dependents,
            &space,
            alpha1,
            &space,
            &alphanumeric1, // TODO: Parse this as an address, e.g. '0xffffffffc0a0c000'
            pair(alpha0, not_line_ending), // TODO: Handle this more elegantly
        ))(input)?;

    dbg!(module_name, module_size, refs, dependents, state, location);

    Ok((
        input,
        KernelModule {
            name: module_name.to_string(),
            size: module_size,
            refs,
            dependents: if dependents == "-" {
                None
            } else {
                Some(
                    dependents
                        .split(',')
                        .map(|s| s.to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                )
            },
            state: match state {
                "Live" => ModuleState::Live,
                "Loading" => ModuleState::Loading,
                "Unloading" => ModuleState::Unloading,
                _ => panic!("Unknown module state: {}", state),
            },
        },
    ))
}

#[derive(Debug)]
pub enum ModuleState {
    Live,
    Loading,
    Unloading,
}

#[derive(Debug)]
pub struct KernelModule {
    name: String,
    size: u64,
    refs: u32,
    dependents: Option<Vec<String>>,
    state: ModuleState,
    // TODO: parse address
    // address: Option<u64>,
}

pub fn parse_module_listing(data: &str) -> Vec<KernelModule> {
    many0(terminated(module_status_line, line_ending))(data)
        .map(|(_, module)| {
            dbg!(&module);
            module
        })
        .unwrap()
}

#[cfg(test)]
const TEST_PROC_MODULES_DATA: &str = "crypto_user 24576 0 - Live 0x0000000000000000
fuse 176128 3 - Live 0x0000000000000000
qemu_fw_cfg 20480 0 - Live 0x0000000000000000
ip_tables 36864 2 iptable_nat,iptable_filter, Live 0x0000000000000000
x_tables 57344 12 xt_nat,xt_tcpudp,xt_conntrack,xt_addrtype,xt_MASQUERADE,xt_mark,ip6table_nat,iptable_nat,ip6table_filter,ip6_tables,iptable_filter,ip_tables, Live 0x0000000000000000
ext4 1015808 1 - Live 0x0000000000000000
crc32c_generic 16384 0 - Live 0x0000000000000000
crc16 16384 1 ext4, Live 0x0000000000000000
mbcache 16384 1 ext4, Live 0x0000000000000000
jbd2 192512 1 ext4, Live 0x0000000000000000
virtio_net 65536 0 - Live 0x0000000000000000
net_failover 24576 1 virtio_net, Live 0x0000000000000000
virtio_balloon 28672 0 - Live 0x0000000000000000
virtio_scsi 28672 1 - Live 0x0000000000000000
failover 16384 1 net_failover, Live 0x0000000000000000
sr_mod 28672 0 - Live 0x0000000000000000
cdrom 81920 1 sr_mod, Live 0x0000000000000000
ata_generic 16384 0 - Live 0x0000000000000000
serio_raw 20480 0 - Live 0x0000000000000000
atkbd 36864 0 - Live 0x0000000000000000
pata_acpi 16384 0 - Live 0x0000000000000000
libps2 20480 2 psmouse,atkbd, Live 0x0000000000000000
i8042 40960 0 - Live 0x0000000000000000
virtio_pci 24576 0 - Live 0x0000000000000000
crc32c_intel 24576 3 - Live 0x0000000000000000
usbhid 77824 0 - Live 0x0000000000000000
virtio_pci_modern_dev 20480 1 virtio_pci, Live 0x0000000000000000
ata_piix 40960 0 - Live 0x0000000000000000
floppy 114688 0 - Live 0x0000000000000000
serio 28672 6 psmouse,serio_raw,atkbd,i8042, Live 0x0000000000000000
";

#[test]
fn test_parse_module_listing() {
    dbg!(parse_module_listing(TEST_PROC_MODULES_DATA));
}
